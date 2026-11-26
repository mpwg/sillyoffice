use crate::{
    Designation, FloorId, LiftId, LiftStopId, Location, PersonId, PlaceId, PlaceKind, SectionId,
    World,
};
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
    pub balance: i64,
    pub waiting_households: usize,
    pub development_points: u32,
    pub lift_unlocked: bool,
    pub places: Vec<VisiblePlace>,
    pub sections: Vec<VisibleSection>,
    pub people: Vec<VisiblePerson>,
    pub stops: Vec<VisibleStop>,
    pub lifts: Vec<VisibleLift>,
    pub initiatives: Vec<VisibleInitiative>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct VisiblePlace {
    pub id: PlaceId,
    pub floor: FloorId,
    pub column: u32,
    pub kind: PlaceKind,
    pub section: SectionId,
    #[serde(rename = "use")]
    pub designation: Option<Designation>,
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
    WaitingLift {
        place: VisiblePlace,
        stop: LiftStopId,
        arrival_tick: u64,
    },
    RidingLift {
        place: VisiblePlace,
        lift: LiftId,
        to: VisiblePlace,
    },
}

#[derive(Debug, Serialize)]
pub struct VisibleStop {
    pub id: LiftStopId,
    pub floor: FloorId,
    pub column: u32,
    pub section: SectionId,
}

#[derive(Debug, Serialize)]
pub struct VisibleLift {
    pub id: LiftId,
    pub stop: LiftStopId,
    pub floor: FloorId,
    pub column: u32,
    pub waiting: usize,
    pub riders: usize,
    pub motion: Option<VisibleMotion>,
}

#[derive(Debug, Serialize)]
pub struct VisibleMotion {
    pub from_floor: FloorId,
    pub to_floor: FloorId,
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Debug, Serialize)]
pub struct VisibleInitiative {
    pub from: VisiblePlace,
    pub to: VisiblePlace,
    pub ready_tick: u64,
    pub built: bool,
    pub person: PersonId,
}

impl World {
    pub(crate) fn visible_places(&self) -> BTreeMap<PlaceId, VisiblePlace> {
        let geometry = self.scenario.geometry();
        let section_floors: BTreeMap<_, _> = geometry
            .sections
            .iter()
            .map(|section| (section.id, section.floor))
            .collect();
        geometry
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
                        section: place.section,
                        designation: self.state.designations.get(&place.id).copied(),
                    },
                )
            })
            .collect()
    }

    pub(crate) fn visible_location(
        &self,
        places: &BTreeMap<PlaceId, VisiblePlace>,
        location: Location,
    ) -> VisibleLocation {
        match location {
            Location::At { place } => VisibleLocation::At {
                place: places[&place],
            },
            Location::Walking(segment) => VisibleLocation::Walking {
                from: places[&segment.from],
                to: places[&segment.to],
                start_tick: segment.start_tick,
                end_tick: segment.end_tick,
            },
            Location::WaitingLift {
                place,
                from,
                arrival_tick,
                ..
            } => VisibleLocation::WaitingLift {
                place: places[&place],
                stop: from,
                arrival_tick,
            },
            Location::RidingLift {
                place, lift, to, ..
            } => {
                let car_place = self
                    .state
                    .lifts
                    .lines()
                    .find(|(id, _)| *id == lift)
                    .and_then(|(_, line)| self.stop_place(line.car_stop()))
                    .unwrap_or(place);
                let target = self.stop_place(to).unwrap_or(car_place);
                VisibleLocation::RidingLift {
                    place: places[&car_place],
                    lift,
                    to: places[&target],
                }
            }
        }
    }

    pub fn view(&self, area: ViewRect) -> ViewFrame {
        let floors = area.first_floor..=area.last_floor;
        let geometry = self.scenario.geometry();
        let places = self.visible_places();
        let people = self
            .state
            .people
            .iter()
            .filter_map(|(&id, person)| {
                let location = self.visible_location(&places, person.location);
                let visible = match location {
                    VisibleLocation::At { place }
                    | VisibleLocation::WaitingLift { place, .. }
                    | VisibleLocation::RidingLift { place, .. } => floors.contains(&place.floor),
                    VisibleLocation::Walking { from, to, .. } => {
                        !floors.is_empty()
                            && from.floor.min(to.floor) <= *floors.end()
                            && from.floor.max(to.floor) >= *floors.start()
                    }
                };
                visible.then_some(VisiblePerson { id, location })
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
        let stops = geometry
            .lift_stops
            .iter()
            .filter(|stop| floors.contains(&stop.floor))
            .filter_map(|stop| {
                Some(VisibleStop {
                    id: stop.id,
                    floor: stop.floor,
                    column: stop.column,
                    section: self.stop_section(stop.id)?,
                })
            })
            .collect();
        let lifts = self
            .state
            .lifts
            .lines()
            .filter_map(|(id, line)| {
                let stop = line.car_stop();
                let halt = self.stop(stop)?;
                let motion = line.car_motion().and_then(|(from, to, end_tick)| {
                    Some(VisibleMotion {
                        from_floor: self.stop(from)?.floor,
                        to_floor: self.stop(to)?.floor,
                        start_tick: end_tick.saturating_sub(line.travel_ticks()),
                        end_tick,
                    })
                });
                let visible = motion
                    .as_ref()
                    .map_or(floors.contains(&halt.floor), |motion| {
                        !floors.is_empty()
                            && motion.from_floor.min(motion.to_floor) <= *floors.end()
                            && motion.from_floor.max(motion.to_floor) >= *floors.start()
                    });
                visible.then(|| VisibleLift {
                    id,
                    stop,
                    floor: halt.floor,
                    column: halt.column,
                    waiting: self.state.lifts.waiting_count(id),
                    riders: self.state.lifts.rider_count(id),
                    motion,
                })
            })
            .collect();
        let initiatives = self
            .visible_initiatives(&places)
            .filter(|entry| floors.contains(&entry.from.floor) || floors.contains(&entry.to.floor))
            .collect();
        ViewFrame {
            tick: self.tick(),
            balance: self.state.balance,
            waiting_households: self.state.waiting_households.len(),
            development_points: self.state.development_points,
            lift_unlocked: self.state.lift_unlocked,
            places: places
                .into_values()
                .filter(|place| floors.contains(&place.floor))
                .collect(),
            sections,
            people,
            stops,
            lifts,
            initiatives,
        }
    }
}
