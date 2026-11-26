use crate::{HouseholdId, LiftStopId, PersonId, PlaceId, SectionId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct CommandNumber(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledCommand {
    pub tick: u64,
    pub number: CommandNumber,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    RentSection {
        section: SectionId,
    },
    Designate {
        place: PlaceId,
        #[serde(rename = "use")]
        designation: Designation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SubmitError {
    #[error("Befehl {number:?} zielt auf Tick {target}, Tick {current} ist bereits abgeschlossen")]
    PastTick {
        number: CommandNumber,
        target: u64,
        current: u64,
    },
    #[error("Befehlsnummer {0:?} wurde bereits eingereiht")]
    DuplicateNumber(CommandNumber),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct CommandResult {
    pub number: CommandNumber,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    Applied,
    Rejected { reason: Rejection },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Error)]
#[serde(rename_all = "snake_case")]
pub enum Rejection {
    #[error("Der Abschnitt ist unbekannt.")]
    UnknownSection,
    #[error("Der Abschnitt ist bereits gemietet.")]
    AlreadyRented,
    #[error("Das Guthaben reicht nicht aus.")]
    InsufficientFunds,
    #[error("Der Ort ist unbekannt.")]
    UnknownPlace,
    #[error("Der Abschnitt muss zuerst gemietet werden.")]
    SectionNotRented,
    #[error("Diese Fläche kann nicht umgewidmet werden.")]
    FixedPlace,
    #[error("Die Fläche wird derzeit genutzt.")]
    PlaceOccupied,
    #[error("Die Fläche hat diese Widmung bereits.")]
    AlreadyDesignated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Designation {
    Home,
    Office,
    Kiosk,
    Corridor,
    Vacant,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Impact {
    pub immediate_cost: u32,
    pub section_rent_change: i64,
    pub households_leaving: Vec<HouseholdId>,
    pub jobs_lost: Vec<PersonId>,
    pub affected_people: Vec<PersonId>,
    pub remaining_stops: Vec<LiftStopId>,
    pub confirmation: Option<String>,
}
