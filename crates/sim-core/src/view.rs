use crate::{FloorId, Location, PersonId, PlaceId, PlaceKind, SectionId, World};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct ViewRect {
    pub first_floor: FloorId,
    pub last_floor: FloorId,
}

#[derive(Debug, Serialize)]
pub struct ViewFrame {
    pub tick: u64,
    pub places: Vec<VisiblePlace>,
    pub sections: Vec<VisibleSection>,
    pub people: Vec<VisiblePerson>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct VisiblePlace {
    pub id: PlaceId,
    pub floor: FloorId,
    pub column: u32,
    pub kind: PlaceKind,
}

#[derive(Debug, Serialize)]
pub struct VisibleSection {
    pub id: SectionId,
    pub floor: FloorId,
    pub start: u32,
    pub end: u32,
    pub rented: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct VisiblePerson {
    pub id: PersonId,
    pub location: VisibleLocation,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibleLocation {
    At {
        place: VisiblePlace,
    },
    Walking {
        from: VisiblePlace,
        to: VisiblePlace,
        start_tick: u64,
        end_tick: u64,
    },
}

impl World {
    pub fn view(&self, area: ViewRect) -> ViewFrame {
        let floors = area.first_floor..=area.last_floor;
        let geometry = self.scenario.geometry();
        let section_floors: BTreeMap<_, _> = geometry
            .sections
            .iter()
            .map(|section| (section.id, section.floor))
            .collect();
        // The scenario validates every place's section and every movement endpoint at load.
        let places: BTreeMap<_, _> = geometry
            .places
            .iter()
            .map(|place| {
                (
                    place.id,
                    VisiblePlace {
                        id: place.id,
                        floor: section_floors[&place.section],
                        column: place.column,
                        kind: place.kind,
                    },
                )
            })
            .collect();

        let people = self
            .state
            .people
            .iter()
            .filter_map(|(&id, person)| {
                let location = match person.location {
                    Location::At { place } if floors.contains(&places[&place].floor) => {
                        VisibleLocation::At {
                            place: places[&place],
                        }
                    }
                    Location::Walking(segment) => {
                        let from = places[&segment.from];
                        let to = places[&segment.to];
                        let segment_floors = from.floor.min(to.floor)..=from.floor.max(to.floor);
                        if floors.is_empty()
                            || segment_floors.end() < floors.start()
                            || segment_floors.start() > floors.end()
                        {
                            return None;
                        }
                        VisibleLocation::Walking {
                            from,
                            to,
                            start_tick: segment.start_tick,
                            end_tick: segment.end_tick,
                        }
                    }
                    Location::At { .. } => return None,
                };
                Some(VisiblePerson { id, location })
            })
            .collect();

        let sections = geometry
            .sections
            .iter()
            .filter(|section| floors.contains(&section.floor))
            .map(|section| VisibleSection {
                id: section.id,
                floor: section.floor,
                start: section.start,
                end: section.end,
                rented: self.is_rented(section.id),
            })
            .collect();
        ViewFrame {
            tick: self.tick(),
            places: places
                .into_values()
                .filter(|place| floors.contains(&place.floor))
                .collect(),
            sections,
            people,
        }
    }
}
