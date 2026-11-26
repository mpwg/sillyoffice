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
    UnlockLift,
    ReturnSection {
        section: SectionId,
        #[serde(default)]
        confirmation: String,
    },
    CoffeeTrip {
        person: PersonId,
        kiosk: PlaceId,
    },
    BuildLift {
        stops: Vec<LiftStopId>,
    },
    BuildExpress {
        stops: Vec<LiftStopId>,
    },
    Commute {
        person: PersonId,
        toward: CommuteTarget,
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
    #[error("Keine weiteren Befehlsnummern oder Ticks verfügbar")]
    Exhausted,
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
    #[error("Der Lift ist noch gesperrt.")]
    LiftLocked,
    #[error("Der Lift wurde bereits freigeschaltet.")]
    LiftAlreadyUnlocked,
    #[error("Für die Freischaltung fehlt ein Entwicklungspunkt.")]
    NoDevelopmentPoints,
    #[error("Die Halte bilden keine gültige Liftlinie.")]
    InvalidLiftStops,
    #[error("Mindestens ein Halt hat keinen zugänglichen Anschluss.")]
    LiftStopBlocked,
    #[error("Der Schacht wird bereits von einer Linie bedient.")]
    LiftStopOccupied,
    #[error("Diese Person ist nicht im Gebäude.")]
    UnknownPerson,
    #[error("Die Person ist bereits unterwegs.")]
    PersonInTransit,
    #[error("Das Ziel ist derzeit nicht erreichbar.")]
    NoRoute,
    #[error("Der Zielraum ist kein Kiosk.")]
    NotAKiosk,
    #[error("Der Kaffeegang beginnt am Arbeitsplatz.")]
    CoffeeFromOffice,
    #[error("Der Abschnitt mit der Rezeption bleibt geöffnet.")]
    ProtectedReception,
    #[error("Die Folgen haben sich geändert. Bitte Vorschau erneut bestätigen.")]
    ReturnChanged,
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
    pub stops: Vec<LiftStopId>,
    pub immediate_cost: u32,
    pub section_rent_change: i64,
    pub households_leaving: Vec<HouseholdId>,
    pub jobs_lost: Vec<PersonId>,
    pub affected_people: Vec<PersonId>,
    pub remaining_stops: Vec<LiftStopId>,
    pub confirmation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommuteTarget {
    Home,
    Work,
}
