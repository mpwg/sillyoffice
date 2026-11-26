//! Engine-independent building simulation.

mod scenario;

pub use scenario::{
    Applicant, DataError, Floor, FloorId, Geometry, HouseholdId, InitialOccupancy, InitialPerson,
    LiftStop, LiftStopId, OperatingRules, PersonId, Place, PlaceId, PlaceKind, Scenario, Section,
    SectionId, Stair, StairId, Walkway, WalkwayId,
};

mod command;
mod replay;
mod world;

pub use command::{
    Action, CommandNumber, CommandResult, CommuteTarget, Designation, Impact, Outcome, Rejection,
    ScheduledCommand, SubmitError,
};
pub use replay::{Replay, ReplayError, ReplayReport};
pub use world::{
    Checkpoint, CoreState, Location, Person, TickReport, TransportStats, WalkSegment, World,
};

mod view;
pub use view::{ViewFrame, ViewRect};

mod economy;
mod routing;
mod tenancy;

mod lift;
mod transit;
pub use lift::LiftId;

mod save;
pub use save::SaveError;

mod initiative;

mod inspect;
pub use inspect::PersonInfo;

mod return_section;
