use crate::view::{VisibleLocation, VisiblePlace};
use crate::{HouseholdId, Location, PersonId, PlaceId, World};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PersonInfo {
    pub id: PersonId,
    pub household: HouseholdId,
    pub home: PlaceId,
    pub office: Option<PlaceId>,
    pub location: VisibleLocation,
    pub destination: Option<VisiblePlace>,
    pub reason: &'static str,
    pub waited_ticks: u64,
    pub frustration: u32,
    pub dissatisfaction: u32,
}

impl World {
    pub fn explain(&self, id: PersonId) -> Option<PersonInfo> {
        let person = self.state.people.get(&id)?;
        let places = self.visible_places();
        let reason = match person.location {
            Location::WaitingLift { .. } => "waiting_for_lift",
            Location::RidingLift { .. } => "in_lift",
            Location::Walking(_) => "walking",
            Location::At { place } if person.office == Some(place) => "at_work",
            Location::At { place } if person.home == place => "at_home",
            Location::At { .. } => "idle",
        };
        let current_wait = match person.location {
            Location::WaitingLift { arrival_tick, .. } => self.tick().saturating_sub(arrival_tick),
            _ => 0,
        };
        Some(PersonInfo {
            id,
            household: person.household,
            home: person.home,
            office: person.office,
            location: self.visible_location(&places, person.location),
            destination: person
                .destination
                .and_then(|goal| places.get(&goal).copied()),
            reason,
            waited_ticks: person.waited_ticks.saturating_add(current_wait),
            frustration: person.frustration,
            dissatisfaction: person.dissatisfaction,
        })
    }
}
