use crate::initiative::CoffeeTrip;
use crate::routing::RouteLeg;
use crate::world::checksum;
use crate::{
    Designation, HouseholdId, Impact, LiftId, LiftStopId, Location, Person, PersonId, PlaceId,
    PlaceKind, Rejection, SectionId, World,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
struct ReturnPlan {
    section: SectionId,
    rent: u32,
    places: BTreeSet<PlaceId>,
    households: BTreeSet<HouseholdId>,
    people_leaving: BTreeSet<PersonId>,
    jobs: BTreeSet<PersonId>,
    routes: BTreeMap<PersonId, PlaceId>,
    disrupted_lifts: BTreeMap<LiftId, Vec<LiftStopId>>,
    rides: Vec<(PersonId, LiftStopId, LiftStopId, Option<LiftStopId>)>,
    removed_stops: BTreeSet<LiftStopId>,
    slides: Vec<(PlaceId, PlaceId)>,
}

impl ReturnPlan {
    fn impact(&self) -> Impact {
        let affected_people = self
            .people_leaving
            .iter()
            .chain(&self.jobs)
            .chain(self.routes.keys())
            .copied()
            .chain(self.rides.iter().map(|ride| ride.0))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let remaining_stops = self
            .disrupted_lifts
            .values()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Impact {
            section_rent_change: -i64::from(self.rent),
            households_leaving: self.households.iter().copied().collect(),
            jobs_lost: self.jobs.iter().copied().collect(),
            affected_people,
            remaining_stops,
            confirmation: Some(checksum(self)),
            ..Impact::default()
        }
    }
}

impl World {
    pub(crate) fn preview_return(&self, section: SectionId) -> Result<Impact, Rejection> {
        Ok(self.plan_return(section)?.impact())
    }

    fn plan_return(&self, section: SectionId) -> Result<ReturnPlan, Rejection> {
        let definition = self
            .scenario
            .geometry()
            .sections
            .iter()
            .find(|entry| entry.id == section)
            .ok_or(Rejection::UnknownSection)?;
        if !self.is_rented(section) {
            return Err(Rejection::SectionNotRented);
        }
        let places: BTreeSet<_> = self
            .scenario
            .geometry()
            .places
            .iter()
            .filter(|place| place.section == section)
            .map(|place| place.id)
            .collect();
        if self
            .scenario
            .geometry()
            .places
            .iter()
            .any(|place| place.kind == PlaceKind::Reception && place.section == section)
        {
            return Err(Rejection::ProtectedReception);
        }
        let reception = self.reception().ok_or(Rejection::ProtectedReception)?;
        let households: BTreeSet<_> = self
            .state
            .households
            .iter()
            .filter_map(|(&id, home)| places.contains(home).then_some(id))
            .collect();
        let mut people_leaving = BTreeSet::new();
        let mut jobs = BTreeSet::new();
        let mut routes = BTreeMap::new();

        let removed_stops: BTreeSet<_> = self
            .scenario
            .geometry()
            .lift_stops
            .iter()
            .filter(|halt| self.stop_section(halt.id) == Some(section))
            .map(|halt| halt.id)
            .collect();
        let mut disrupted_lifts = BTreeMap::new();
        let mut rides = Vec::new();
        for (id, line) in self.state.lifts.lines() {
            let Some(first_removed) = line
                .stops()
                .iter()
                .position(|halt| removed_stops.contains(halt))
            else {
                continue;
            };
            let remaining = if first_removed >= 2 {
                line.stops()[..first_removed].to_vec()
            } else {
                Vec::new()
            };
            disrupted_lifts.insert(id, remaining);
            rides.extend(
                self.state
                    .lifts
                    .affected_by_interrupt(id, &removed_stops)
                    .into_iter()
                    .map(|ride| {
                        (
                            ride.person,
                            ride.destination,
                            ride.last_stop,
                            ride.next_stop,
                        )
                    }),
            );
        }
        for (&id, person) in &self.state.people {
            if households.contains(&person.household) {
                people_leaving.insert(id);
                continue;
            }
            let job_lost = person.office.is_some_and(|office| places.contains(&office));
            if job_lost {
                jobs.insert(id);
            }
            let stops_removed = |lift, from, to| {
                disrupted_lifts
                    .get(&lift)
                    .is_some_and(|surviving| !surviving.contains(&from) || !surviving.contains(&to))
            };
            let route_removed = person.remaining_route.iter().any(|leg| match *leg {
                RouteLeg::Walk { from, to, .. } => places.contains(&from) || places.contains(&to),
                RouteLeg::Lift { lift, from, to } => stops_removed(lift, from, to),
            });
            let walking_removed = match person.location {
                Location::Walking(segment) => {
                    places.contains(&segment.from) || places.contains(&segment.to)
                }
                Location::At { place } => places.contains(&place),
                Location::WaitingLift { place, .. } | Location::RidingLift { place, .. } => {
                    places.contains(&place)
                }
            };
            if walking_removed
                || route_removed
                || job_lost
                || person
                    .destination
                    .is_some_and(|goal| places.contains(&goal))
            {
                routes.insert(id, self.return_anchor(person, &places).unwrap_or(reception));
            }
        }
        let slides = self
            .state
            .initiatives
            .iter()
            .flat_map(|(&from, paths)| paths.keys().map(move |&to| (from, to)))
            .filter(|(from, to)| places.contains(from) || places.contains(to))
            .collect();

        Ok(ReturnPlan {
            section,
            rent: definition.rent_cost,
            places,
            households,
            people_leaving,
            jobs,
            routes,
            disrupted_lifts,
            rides,
            removed_stops,
            slides,
        })
    }

    fn return_anchor(&self, person: &Person, removed: &BTreeSet<PlaceId>) -> Option<PlaceId> {
        let open = |place: PlaceId| !removed.contains(&place) && self.access(place);
        let (first, previous) = match person.location {
            Location::Walking(segment) => (segment.to, segment.from),
            Location::At { place }
            | Location::WaitingLift { place, .. }
            | Location::RidingLift { place, .. } => (place, place),
        };
        std::iter::once(first)
            .chain(person.remaining_route.iter().filter_map(|leg| match *leg {
                RouteLeg::Walk { to, .. } => Some(to),
                RouteLeg::Lift { to, .. } => self.stop_place(to),
            }))
            .chain(std::iter::once(previous))
            .find(|&place| open(place))
    }

    pub(crate) fn return_section(
        &mut self,
        section: SectionId,
        confirmation: &str,
    ) -> Result<(), Rejection> {
        let plan = self.plan_return(section)?;
        if checksum(&plan) != confirmation {
            return Err(Rejection::ReturnChanged);
        }
        self.state.rented_sections.remove(&section);
        for place in &plan.places {
            if self
                .place(*place)
                .is_some_and(|entry| entry.kind == PlaceKind::Space)
            {
                self.state.designations.insert(*place, Designation::Vacant);
            }
        }
        for household in plan.households {
            self.state.households.remove(&household);
        }
        for id in &plan.people_leaving {
            self.state.lifts.cancel(*id);
            self.state.people.remove(id);
        }
        for id in &plan.jobs {
            if let Some(person) = self.state.people.get_mut(id) {
                if person.destination == person.office {
                    person.destination = None;
                }
                person.office = None;
            }
        }
        for id in plan.routes.keys() {
            let person = self.state.people.get_mut(id).unwrap();
            if person
                .destination
                .is_some_and(|goal| plan.places.contains(&goal))
            {
                person.destination = None;
            }
        }
        self.state.coffee_trips.retain(|id, trip| {
            if plan.people_leaving.contains(id) {
                return false;
            }
            match *trip {
                CoffeeTrip::Outbound { office, kiosk, .. } => {
                    !plan.places.contains(&office) && !plan.places.contains(&kiosk)
                }
                CoffeeTrip::Returning { office } => !plan.places.contains(&office),
            }
        });
        for (from, to) in plan.slides {
            if let Some(paths) = self.state.initiatives.get_mut(&from) {
                paths.remove(&to);
            }
        }
        self.state.initiatives.retain(|_, paths| !paths.is_empty());
        self.state.coffee_observations.retain(|from, paths| {
            paths.retain(|to, _| !plan.places.contains(to));
            !plan.places.contains(from) && !paths.is_empty()
        });

        let mut replanned = BTreeSet::new();
        for (lift, remaining) in plan.disrupted_lifts {
            if remaining.is_empty() {
                self.state.express_lifts.remove(&lift);
            }
            for event in self.state.lifts.interrupt(lift, &plan.removed_stops) {
                if let crate::lift::LiftEvent::Replan { person, stop, .. } = event
                    && let Some(place) = stop
                        .and_then(|stop| self.stop_place(stop))
                        .or_else(|| plan.routes.get(&person).copied())
                        .or_else(|| self.reception())
                {
                    self.replan_at(person, place);
                    replanned.insert(person);
                }
            }
        }
        for (id, place) in plan.routes {
            if replanned.contains(&id) {
                continue;
            }
            let location = self.state.people[&id].location;
            if matches!(
                location,
                Location::RidingLift { .. } | Location::WaitingLift { .. }
            ) {
                // Riders on retained services finish at the next valid stop before replanning.
                self.state
                    .people
                    .get_mut(&id)
                    .unwrap()
                    .remaining_route
                    .clear();
            } else if let Location::Walking(segment) = location
                && self.access(segment.from)
                && self.access(segment.to)
            {
                let destination = self.state.people[&id].destination;
                let route = destination
                    .filter(|&goal| self.access(goal))
                    .and_then(|goal| {
                        crate::routing::walk(&self.routes(), segment.to, goal, segment.end_tick)
                    });
                self.state.people.get_mut(&id).unwrap().remaining_route = route.unwrap_or_default();
            } else {
                self.replan_at(id, place);
            }
        }
        self.refresh_access_widths();
        self.sample_wait_estimates();
        Ok(())
    }
}
