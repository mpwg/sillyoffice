//! Engine-independent building simulation.

mod scenario;

pub use scenario::{
    DataError, Floor, FloorId, Geometry, InitialOccupancy, InitialPerson, LiftStop, LiftStopId,
    PersonId, Place, PlaceId, PlaceKind, Scenario, Section, SectionId, Stair, StairId, Walkway,
    WalkwayId,
};

mod command;
mod replay;
mod world;

pub use command::{
    Action, CommandNumber, CommandResult, Outcome, Rejection, ScheduledCommand, SubmitError,
};
pub use replay::{Replay, ReplayError, ReplayReport};
pub use world::{Checkpoint, CoreState, Location, Person, TickReport, WalkSegment, World};

mod view;
pub use view::{ViewFrame, ViewRect};
