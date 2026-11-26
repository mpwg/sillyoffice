use std::collections::VecDeque;
use std::num::{NonZeroU32, NonZeroU64};

use crate::lift::{LiftEvent, LiftLine};
use crate::routing::{RouteLeg, Routes, walk, walking_edges};
use crate::{
    CommuteTarget, Impact, LiftId, LiftStop, LiftStopId, Location, Person, PersonId, PlaceId,
    Rejection, SectionId, WalkSegment, World,
};

impl World {
    pub(crate) fn preview_unlock(&self) -> Result<Impact, Rejection> {
        if self.state.lift_unlocked {
            return Err(Rejection::LiftAlreadyUnlocked);
        }
        if self.state.development_points == 0 {
            return Err(Rejection::NoDevelopmentPoints);
        }
        Ok(Impact::default())
    }

    pub(crate) fn stop(&self, id: LiftStopId) -> Option<&LiftStop> {
        let stops = &self.scenario.geometry().lift_stops;
        stops
            .binary_search_by_key(&id, |stop| stop.id)
            .ok()
            .map(|index| &stops[index])
    }

    pub(crate) fn stop_place(&self, stop: LiftStopId) -> Option<PlaceId> {
        let stop = self.stop(stop)?;
        self.scenario
            .geometry()
            .places
            .iter()
            .find(|place| {
                place.column == stop.column
                    && self
                        .scenario
                        .geometry()
                        .sections
                        .iter()
                        .any(|section| place.section == section.id && section.floor == stop.floor)
                    && self.access(place.id)
            })
            .map(|place| place.id)
    }

    pub(crate) fn stop_section(&self, stop: LiftStopId) -> Option<SectionId> {
        let stop = self.stop(stop)?;
        self.scenario
            .geometry()
            .sections
            .iter()
            .find(|section| {
                section.floor == stop.floor && (section.start..section.end).contains(&stop.column)
            })
            .map(|section| section.id)
    }

    pub(crate) fn preview_lift(
        &self,
        stops: &[LiftStopId],
        express: bool,
    ) -> Result<Impact, Rejection> {
        if !self.state.lift_unlocked {
            return Err(Rejection::LiftLocked);
        }
        if stops.len() < 2
            || (!express && stops.len() > 15)
            || self.state.next_lift_id > u64::from(u32::MAX)
        {
            return Err(Rejection::InvalidLiftStops);
        }
        let definitions: Vec<_> = stops
            .iter()
            .map(|&id| self.stop(id))
            .collect::<Option<_>>()
            .ok_or(Rejection::InvalidLiftStops)?;
        let floor_step = if express { 10 } else { 1 };
        if (express && !definitions[0].floor.0.is_multiple_of(10))
            || definitions.windows(2).any(|pair| {
                pair[0].floor.0.checked_add(floor_step) != Some(pair[1].floor.0)
                    || pair[0].column != pair[1].column
            })
        {
            return Err(Rejection::InvalidLiftStops);
        }
        for &stop in stops {
            if self.stop_place(stop).is_none() {
                return Err(Rejection::LiftStopBlocked);
            }
            if self.state.lifts.lines().any(|(lift, line)| {
                self.state.express_lifts.contains(&lift) == express && line.stops().contains(&stop)
            }) {
                return Err(Rejection::LiftStopOccupied);
            }
        }
        let cost = self.scenario.rules.map_or(0, |rules| rules.lift_cost);
        if self.state.balance < i64::from(cost) {
            return Err(Rejection::InsufficientFunds);
        }
        Ok(Impact {
            stops: stops.to_vec(),
            immediate_cost: cost,
            ..Impact::default()
        })
    }

    pub(crate) fn install_lift(
        &mut self,
        stops: Vec<LiftStopId>,
        express: bool,
    ) -> Result<(), Rejection> {
        let id = LiftId(self.state.next_lift_id as u32);
        let line = LiftLine::new(
            stops,
            NonZeroU32::new(if express { 18 } else { 8 }).unwrap(),
            NonZeroU64::new(if express { 4 } else { 2 }).unwrap(),
            NonZeroU64::MIN,
        )
        .map_err(|_| Rejection::InvalidLiftStops)?;
        self.state
            .lifts
            .insert(id, line)
            .map_err(|_| Rejection::LiftStopOccupied)?;
        self.state.next_lift_id += 1;
        if express {
            self.state.express_lifts.insert(id);
        }
        self.refresh_access_widths();
        self.sample_wait_estimates();
        Ok(())
    }

    pub(crate) fn refresh_access_widths(&mut self) {
        let mut entrances = std::collections::BTreeMap::new();
        for stop in &self.scenario.geometry().lift_stops {
            let Some(section) = self.stop_section(stop.id) else {
                continue;
            };
            let width = 1 + self
                .scenario
                .geometry()
                .places
                .iter()
                .filter(|place| {
                    place.section == section
                        && self.state.designations.get(&place.id)
                            == Some(&crate::Designation::Corridor)
                })
                .count() as u32
                * 2;
            let group = *entrances.entry(section).or_insert(stop.id);
            self.state.lifts.set_access_group(stop.id, group);
            self.state
                .lifts
                .set_access_width(group, NonZeroU32::new(width).unwrap());
        }
    }

    pub(crate) fn sample_wait_estimates(&mut self) {
        self.state.wait_estimates = self
            .state
            .lifts
            .lines()
            .map(|(id, line)| {
                let stops = line
                    .stops()
                    .iter()
                    .map(|&stop| (stop, self.state.lifts.estimated_wait(id, stop)))
                    .collect();
                (id, stops)
            })
            .collect();
    }

    fn wait_estimate(&self, lift: LiftId, stop: LiftStopId) -> u64 {
        self.state
            .wait_estimates
            .get(&lift)
            .and_then(|stops| stops.get(&stop))
            .copied()
            .unwrap_or_else(|| self.state.lifts.estimated_wait(lift, stop))
    }

    pub(crate) fn update_wait_estimates(&mut self) {
        let Some(rules) = self.scenario.rules else {
            return;
        };
        let tick = self.tick();
        if !tick.is_multiple_of(rules.wait_sample_ticks.get())
            || self.state.lifts.lines().next().is_none()
        {
            return;
        }
        self.sample_wait_estimates();

        let changes: Vec<_> = self
            .state
            .people
            .iter()
            .filter_map(|(&id, person)| {
                let Location::WaitingLift {
                    place,
                    lift,
                    from,
                    to,
                    arrival_tick,
                } = person.location
                else {
                    return None;
                };
                if tick.saturating_sub(arrival_tick) < rules.wait_sample_ticks.get() {
                    return None;
                }
                let current_cost = self.estimated_segment_cost(lift, from, to)?;
                let (candidate_cost, candidate) = self
                    .state
                    .lifts
                    .lines()
                    .filter(|(id, _)| *id != lift)
                    .filter_map(|(id, _)| {
                        self.estimated_segment_cost(id, from, to)
                            .map(|cost| (cost, id))
                    })
                    .min()?;
                (candidate_cost < current_cost
                    && current_cost - candidate_cost >= rules.route_switch_savings)
                    .then_some((id, place, lift, candidate, from, to, arrival_tick))
            })
            .collect();
        for (id, place, current, candidate, from, to, arrival_tick) in changes {
            self.state.lifts.cancel(id);
            let person = self.state.people.get_mut(&id).unwrap();
            if self
                .state
                .lifts
                .request(candidate, id, from, to, tick)
                .is_ok()
            {
                let waited = tick - arrival_tick;
                person.waited_ticks = person.waited_ticks.saturating_add(waited);
                self.state.total_waited_ticks =
                    self.state.total_waited_ticks.saturating_add(waited);
                person.location = Location::WaitingLift {
                    place,
                    lift: candidate,
                    from,
                    to,
                    arrival_tick: tick,
                };
            } else if self
                .state
                .lifts
                .request(current, id, from, to, arrival_tick)
                .is_err()
            {
                self.replan_at(id, place);
            }
        }
    }

    fn estimated_segment_cost(&self, id: LiftId, from: LiftStopId, to: LiftStopId) -> Option<u64> {
        let (_, line) = self.state.lifts.lines().find(|(lift, _)| *lift == id)?;
        let start = line.stops().iter().position(|stop| *stop == from)?;
        let end = line.stops().iter().position(|stop| *stop == to)?;
        Some(
            (start.abs_diff(end) as u64)
                .saturating_mul(line.travel_ticks())
                .saturating_add(self.wait_estimate(id, from)),
        )
    }

    pub(crate) fn routes(&self) -> Routes {
        let mut edges = walking_edges(&self.scenario, |place| self.access(place));
        self.add_initiative_routes(&mut edges);
        for (lift, line) in self.state.lifts.lines() {
            let access: Vec<_> = line
                .stops()
                .iter()
                .enumerate()
                .filter_map(|(index, &stop)| {
                    self.stop_place(stop).map(|place| (index, stop, place))
                })
                .collect();
            for &(from_index, from, origin) in &access {
                let wait = self.wait_estimate(lift, from);
                for &(to_index, to, destination) in &access {
                    if from == to {
                        continue;
                    }
                    let cost = (from_index.abs_diff(to_index) as u64)
                        .saturating_mul(line.travel_ticks())
                        .saturating_add(wait)
                        .saturating_add(1);
                    edges.entry(origin).or_default().push((
                        destination,
                        cost,
                        RouteLeg::Lift { lift, from, to },
                    ));
                }
            }
        }
        for outgoing in edges.values_mut() {
            outgoing.sort_unstable();
        }
        edges
    }

    pub(crate) fn planned_route(
        &self,
        person: PersonId,
        goal: PlaceId,
    ) -> Result<(PlaceId, VecDeque<RouteLeg>), Rejection> {
        let person = self
            .state
            .people
            .get(&person)
            .ok_or(Rejection::UnknownPerson)?;
        let Location::At { place } = person.location else {
            return Err(Rejection::PersonInTransit);
        };
        if !self.access(goal) {
            return Err(Rejection::NoRoute);
        }
        let route = walk(&self.routes(), place, goal, self.tick()).ok_or(Rejection::NoRoute)?;
        Ok((place, route))
    }

    pub(crate) fn commute_destination(
        &self,
        person: PersonId,
        toward: CommuteTarget,
    ) -> Result<PlaceId, Rejection> {
        let person = self
            .state
            .people
            .get(&person)
            .ok_or(Rejection::UnknownPerson)?;
        match toward {
            CommuteTarget::Home => Ok(person.home),
            CommuteTarget::Work => person.office.ok_or(Rejection::NoRoute),
        }
    }

    pub(crate) fn send_to(&mut self, id: PersonId, goal: PlaceId) -> Result<(), Rejection> {
        self.state.route_searches = self.state.route_searches.saturating_add(1);
        let (from, route) = self.planned_route(id, goal)?;
        self.state.people.get_mut(&id).unwrap().last_lift = None;
        self.start_route(id, from, goal, route);
        Ok(())
    }

    pub(crate) fn start_route(
        &mut self,
        id: PersonId,
        from: PlaceId,
        goal: PlaceId,
        route: VecDeque<RouteLeg>,
    ) {
        let Some(person) = self.state.people.get_mut(&id) else {
            return;
        };
        person.destination = Some(goal);
        person.remaining_route = route;
        self.advance_person(id, from);
    }

    pub(crate) fn progress_transport(&mut self) {
        let tick = self.tick();
        let arrived: Vec<_> = self
            .state
            .people
            .iter()
            .filter_map(|(&id, person)| match person.location {
                Location::Walking(segment) if segment.end_tick <= tick => Some((id, segment.to)),
                _ => None,
            })
            .collect();
        for (id, to) in arrived {
            self.advance_person(id, to);
        }

        let waiting = self
            .state
            .lifts
            .lines()
            .map(|(lift, _)| self.state.lifts.waiting_count(lift))
            .sum();
        self.state.longest_queue = self.state.longest_queue.max(waiting);
        for event in self.state.lifts.step(tick) {
            self.handle_lift_event(event);
        }
    }

    fn advance_person(&mut self, id: PersonId, at: PlaceId) {
        let tick = self.tick();
        let leg = self
            .state
            .people
            .get_mut(&id)
            .and_then(|person| person.remaining_route.pop_front());
        let next = match leg {
            Some(RouteLeg::Walk { from, to, duration })
                if from == at && self.access(to) && self.access(from) =>
            {
                tick.checked_add(duration).map(|end_tick| {
                    Location::Walking(WalkSegment {
                        from,
                        to,
                        start_tick: tick,
                        end_tick,
                    })
                })
            }
            Some(RouteLeg::Lift { lift, from, to })
                if self.stop_place(from) == Some(at) && self.stop_place(to).is_some() =>
            {
                self.state
                    .lifts
                    .request(lift, id, from, to, tick)
                    .ok()
                    .map(|()| Location::WaitingLift {
                        place: at,
                        lift,
                        from,
                        to,
                        arrival_tick: tick,
                    })
            }
            _ => None,
        };

        let person = self.state.people.get_mut(&id).unwrap();
        person.location = next.unwrap_or(Location::At { place: at });
        if next.is_none() {
            person.remaining_route.clear();
            if person.destination.is_none() || person.destination == Some(at) {
                person.destination = None;
                self.arrived_at(id, at);
            } else {
                self.replan_at(id, at);
            }
        }
    }

    pub(crate) fn replan_at(&mut self, id: PersonId, at: PlaceId) {
        let tick = self.tick();
        let Some(person) = self.state.people.get_mut(&id) else {
            return;
        };
        person.location = Location::At { place: at };
        person.remaining_route.clear();
        let destination = person.destination.take();
        if destination == Some(at) {
            self.arrived_at(id, at);
            return;
        }
        if let Some(goal) = destination.filter(|&goal| self.access(goal)) {
            self.state.route_searches = self.state.route_searches.saturating_add(1);
            if let Some(route) = walk(&self.routes(), at, goal, tick) {
                self.start_route(id, at, goal, route);
                return;
            }
        }
        self.state.coffee_trips.remove(&id);
    }

    pub(crate) fn handle_lift_event(&mut self, event: LiftEvent) {
        match event {
            LiftEvent::Boarded {
                lift, person: id, ..
            } => {
                let tick = self.tick();
                if let Some(person) = self.state.people.get_mut(&id)
                    && let Location::WaitingLift {
                        place,
                        from,
                        to,
                        arrival_tick,
                        ..
                    } = person.location
                {
                    let wait = tick - arrival_tick;
                    person.waited_ticks = person.waited_ticks.saturating_add(wait);
                    self.state.total_waited_ticks =
                        self.state.total_waited_ticks.saturating_add(wait);
                    self.state.lift_boardings = self.state.lift_boardings.saturating_add(1);
                    if person.last_lift.is_some_and(|previous| previous != lift) {
                        self.state.lift_transfers = self.state.lift_transfers.saturating_add(1);
                    }
                    person.last_lift = Some(lift);
                    person.location = Location::RidingLift {
                        place,
                        lift,
                        from,
                        to,
                    };
                    if wait >= 8 {
                        if person.frustration > 0 {
                            person.dissatisfaction = person.dissatisfaction.saturating_add(1);
                        }
                        person.frustration = person.frustration.saturating_add(1);
                    }
                }
            }
            LiftEvent::Exited { person, stop, .. } => {
                if let Some(place) = self.stop_place(stop) {
                    self.advance_person(person, place);
                }
            }
            LiftEvent::Replan { person, stop, .. } => {
                if let Some(place) = stop
                    .and_then(|stop| self.stop_place(stop))
                    .or_else(|| self.reception())
                {
                    self.replan_at(person, place);
                }
            }
        }
    }

    pub(crate) fn daily_commutes(&mut self) {
        let Some(rules) = self.scenario.rules else {
            return;
        };
        let tick = self.tick();
        let since_day = tick % rules.day_ticks;
        let homeward = since_day == 0;
        if !homeward && since_day != (rules.day_ticks.get() / 5).max(1) {
            return;
        }

        let people: Vec<_> = self
            .state
            .people
            .iter()
            .filter_map(|(&id, person)| {
                let Location::At { place } = person.location else {
                    return None;
                };
                let goal = if homeward && person.office == Some(place) {
                    Some(person.home)
                } else if !homeward && place == person.home {
                    person.office
                } else {
                    None
                }?;
                Some((id, place, goal))
            })
            .collect();
        if people.is_empty() {
            return;
        }
        let routes = self.routes();
        let mut found = std::collections::BTreeMap::new();
        for (id, place, goal) in people {
            let route = found.entry((place, goal)).or_insert_with(|| {
                self.state.route_searches = self.state.route_searches.saturating_add(1);
                walk(&routes, place, goal, tick)
            });
            if let Some(route) = route.clone() {
                self.state.people.get_mut(&id).unwrap().last_lift = None;
                self.start_route(id, place, goal, route);
            }
        }
    }
}

impl Person {
    pub(crate) fn begin_initial_walk(&mut self, from: PlaceId, tick: u64) {
        if let Some(RouteLeg::Walk { to, duration, .. }) = self.remaining_route.pop_front()
            && let Some(end_tick) = tick.checked_add(duration)
        {
            self.location = Location::Walking(WalkSegment {
                from,
                to,
                start_tick: tick,
                end_tick,
            });
        } else {
            self.destination = None;
        }
    }
}
