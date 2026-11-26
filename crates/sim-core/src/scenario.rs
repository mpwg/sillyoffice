use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const FORMAT_VERSION: u32 = 1;

macro_rules! scenario_ids {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct $name(pub u32);

            impl From<$name> for u32 {
                fn from(id: $name) -> Self {
                    id.0
                }
            }
        )+
    };
}

scenario_ids!(
    FloorId, SectionId, PlaceId, WalkwayId, StairId, LiftStopId, PersonId
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Scenario {
    pub(crate) version: u32,
    pub(crate) geometry: Geometry,
    pub(crate) initial: InitialOccupancy,
}

#[derive(Deserialize)]
struct FormatVersion {
    version: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioV1 {
    version: u32,
    geometry: Geometry,
    initial: InitialOccupancy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub floors: Vec<Floor>,
    pub sections: Vec<Section>,
    pub places: Vec<Place>,
    pub walkways: Vec<Walkway>,
    pub stairs: Vec<Stair>,
    pub lift_stops: Vec<LiftStop>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Floor {
    pub id: FloorId,
    pub width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub id: SectionId,
    pub floor: FloorId,
    pub start: u32,
    /// First column outside this section.
    pub end: u32,
    pub rent_cost: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub id: PlaceId,
    pub section: SectionId,
    pub column: u32,
    pub kind: PlaceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    Reception,
    Office,
    Home,
    Landing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Walkway {
    pub id: WalkwayId,
    pub from: PlaceId,
    pub to: PlaceId,
    pub ticks: NonZeroU64,
    pub bidirectional: bool,
}

/// A bidirectional connection between floors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stair {
    pub id: StairId,
    pub from: PlaceId,
    pub to: PlaceId,
    pub ticks: NonZeroU64,
}

/// A permitted stop location in the building grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiftStop {
    pub id: LiftStopId,
    pub floor: FloorId,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialOccupancy {
    pub balance: i64,
    pub rented_sections: Vec<SectionId>,
    pub people: Vec<InitialPerson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialPerson {
    pub id: PersonId,
    pub location: PlaceId,
    pub home: PlaceId,
    pub office: PlaceId,
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error("Ungültiges Szenario-JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unbekannte Szenarioformatversion {version}")]
    UnsupportedVersion { version: u32 },

    #[error("{entry} mit ID {id} ist mehrfach eingetragen")]
    DuplicateId { entry: &'static str, id: u32 },

    #[error("{entry} {id} verweist in {field} auf die unbekannte ID {target}")]
    UnknownReference {
        entry: &'static str,
        id: u32,
        field: &'static str,
        target: u32,
    },

    #[error(
        "Lifthalt {stop:?} in Spalte {column} liegt außerhalb von Etage {floor:?} mit Breite {width}"
    )]
    StopOutsideFloor {
        stop: LiftStopId,
        floor: FloorId,
        column: u32,
        width: u32,
    },

    #[error("Etage {floor:?} hat Breite 0")]
    InvalidFloorWidth { floor: FloorId },

    #[error(
        "Abschnitt {section:?} mit Spalten {start} bis {end} passt nicht in Etage {floor:?} mit Breite {width}"
    )]
    SectionOutsideFloor {
        section: SectionId,
        floor: FloorId,
        start: u32,
        end: u32,
        width: u32,
    },

    #[error("Ort {place:?} in Spalte {column} liegt außerhalb von Abschnitt {section:?}")]
    PlaceOutsideSection {
        place: PlaceId,
        section: SectionId,
        column: u32,
    },

    #[error("Der Anfangssaldo {balance} ist negativ")]
    NegativeInitialBalance { balance: i64 },
}

impl Scenario {
    /// Loads the scenario and orders its entries by ID.
    ///
    /// # Errors
    ///
    /// Reports incompatible versions, malformed JSON, and invalid scenario entries.
    pub fn from_json(bytes: &[u8]) -> Result<Self, DataError> {
        let FormatVersion { version } = serde_json::from_slice(bytes)?;
        if version != FORMAT_VERSION {
            return Err(DataError::UnsupportedVersion { version });
        }

        let ScenarioV1 {
            version,
            geometry,
            initial,
        } = serde_json::from_slice(bytes)?;
        let mut scenario = Self {
            version,
            geometry,
            initial,
        };
        scenario.canonicalize()?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    pub fn initial(&self) -> &InitialOccupancy {
        &self.initial
    }

    fn canonicalize(&mut self) -> Result<(), DataError> {
        let geometry = &mut self.geometry;
        sort_unique(&mut geometry.floors, |floor| floor.id, "Etage")?;
        sort_unique(&mut geometry.sections, |section| section.id, "Abschnitt")?;
        sort_unique(&mut geometry.places, |place| place.id, "Ort")?;
        sort_unique(&mut geometry.walkways, |walkway| walkway.id, "Gang")?;
        sort_unique(&mut geometry.stairs, |stair| stair.id, "Treppe")?;
        sort_unique(&mut geometry.lift_stops, |stop| stop.id, "Lifthalt")?;
        sort_unique(
            &mut self.initial.rented_sections,
            |id| *id,
            "Anfangsmietvertrag",
        )?;
        sort_unique(&mut self.initial.people, |person| person.id, "Person")
    }

    fn validate(&self) -> Result<(), DataError> {
        self.geometry.validate()?;
        if self.initial.balance < 0 {
            return Err(DataError::NegativeInitialBalance {
                balance: self.initial.balance,
            });
        }

        for section in &self.initial.rented_sections {
            lookup(
                &self.geometry.sections,
                *section,
                |row| row.id,
                ("Anfangsmietvertrag", section.0, "rented_sections"),
            )?;
        }
        for person in &self.initial.people {
            for (field, place) in [
                ("location", person.location),
                ("home", person.home),
                ("office", person.office),
            ] {
                lookup(
                    &self.geometry.places,
                    place,
                    |row| row.id,
                    ("Person", person.id.0, field),
                )?;
            }
        }
        Ok(())
    }
}

impl Geometry {
    fn validate(&self) -> Result<(), DataError> {
        for floor in &self.floors {
            if floor.width == 0 {
                return Err(DataError::InvalidFloorWidth { floor: floor.id });
            }
        }
        for section in &self.sections {
            let floor = lookup(
                &self.floors,
                section.floor,
                |row| row.id,
                ("Abschnitt", section.id.0, "floor"),
            )?;
            if section.start >= section.end || section.end > floor.width {
                return Err(DataError::SectionOutsideFloor {
                    section: section.id,
                    floor: floor.id,
                    start: section.start,
                    end: section.end,
                    width: floor.width,
                });
            }
        }
        for place in &self.places {
            let section = lookup(
                &self.sections,
                place.section,
                |row| row.id,
                ("Ort", place.id.0, "section"),
            )?;
            if !(section.start..section.end).contains(&place.column) {
                return Err(DataError::PlaceOutsideSection {
                    place: place.id,
                    section: section.id,
                    column: place.column,
                });
            }
        }
        for stop in &self.lift_stops {
            let floor = lookup(
                &self.floors,
                stop.floor,
                |row| row.id,
                ("Lifthalt", stop.id.0, "floor"),
            )?;
            if stop.column >= floor.width {
                return Err(DataError::StopOutsideFloor {
                    stop: stop.id,
                    floor: floor.id,
                    column: stop.column,
                    width: floor.width,
                });
            }
        }
        for walkway in &self.walkways {
            self.validate_endpoints(walkway.from, walkway.to, "Gang", walkway.id.0)?;
        }
        for stair in &self.stairs {
            self.validate_endpoints(stair.from, stair.to, "Treppe", stair.id.0)?;
        }
        Ok(())
    }

    fn validate_endpoints(
        &self,
        from: PlaceId,
        to: PlaceId,
        entry: &'static str,
        id: u32,
    ) -> Result<(), DataError> {
        lookup(&self.places, from, |row| row.id, (entry, id, "from"))?;
        lookup(&self.places, to, |row| row.id, (entry, id, "to"))?;
        Ok(())
    }
}

fn sort_unique<T, Id: Copy + Ord + Into<u32>>(
    rows: &mut [T],
    key: impl Fn(&T) -> Id,
    entry: &'static str,
) -> Result<(), DataError> {
    rows.sort_unstable_by_key(&key);
    for [previous, current] in rows.array_windows() {
        let id = key(current);
        if key(previous) == id {
            return Err(DataError::DuplicateId {
                entry,
                id: id.into(),
            });
        }
    }
    Ok(())
}

fn lookup<'a, T, Id: Copy + Ord + Into<u32>>(
    rows: &'a [T],
    target: Id,
    key: impl Fn(&T) -> Id,
    (entry, id, field): (&'static str, u32, &'static str),
) -> Result<&'a T, DataError> {
    rows.binary_search_by_key(&target, key)
        .map(|index| &rows[index])
        .map_err(|_| DataError::UnknownReference {
            entry,
            id,
            field,
            target: target.into(),
        })
}
