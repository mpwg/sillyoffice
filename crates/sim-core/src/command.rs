use crate::SectionId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct CommandNumber(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledCommand {
    pub tick: u64,
    pub number: CommandNumber,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    RentSection { section: SectionId },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CommandResult {
    pub number: CommandNumber,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    Applied,
    Rejected { reason: Rejection },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rejection {
    UnknownSection,
    AlreadyRented,
    InsufficientFunds,
}
