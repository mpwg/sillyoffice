use crate::{LiftStopId, PersonId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::num::{NonZeroU32, NonZeroU64};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LiftId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LiftLine {
    stops: Vec<LiftStopId>,
    capacity: NonZeroU32,
    travel_ticks: NonZeroU64,
    door_ticks: NonZeroU64,
    motion: Motion,
    direction: Option<Direction>,
    waiting: BTreeMap<PersonId, RideRequest>,
    riders: BTreeMap<PersonId, Destination>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredSystem")]
pub(crate) struct LiftSystem {
    lines: BTreeMap<LiftId, LiftLine>,
    access_widths: BTreeMap<LiftStopId, NonZeroU32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    access_groups: BTreeMap<LiftStopId, LiftStopId>,
}

#[derive(Deserialize)]
struct StoredSystem {
    lines: BTreeMap<LiftId, LiftLine>,
    access_widths: BTreeMap<LiftStopId, NonZeroU32>,
    #[serde(default)]
    access_groups: BTreeMap<LiftStopId, LiftStopId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Motion {
    At {
        stop: usize,
        doors_until: Option<u64>,
    },
    Moving {
        // A returning car can finish its leg after the departure stop is removed.
        from: Option<usize>,
        to: usize,
        arrival: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct RideRequest {
    from: LiftStopId,
    to: LiftStopId,
    arrival: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Destination {
    Requested(LiftStopId),
    Replan(LiftStopId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum LiftError {
    #[error("Unbekannter Lift")]
    UnknownLift,
    #[error("Ungültige Halte für die Fahrt")]
    InvalidStops,
    #[error("Lift-ID ist bereits belegt")]
    DuplicateLift,
    #[error("Für die Person besteht bereits ein Fahrwunsch oder eine Fahrt")]
    DuplicatePerson,
    #[error("Ungültiger Zustand der Liftanlage")]
    InvalidState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiftEvent {
    Boarded {
        lift: LiftId,
        person: PersonId,
        stop: LiftStopId,
    },
    Exited {
        lift: LiftId,
        person: PersonId,
        stop: LiftStopId,
    },
    Replan {
        lift: LiftId,
        person: PersonId,
        stop: Option<LiftStopId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InterruptedRide {
    pub(crate) person: PersonId,
    pub(crate) destination: LiftStopId,
    pub(crate) last_stop: LiftStopId,
    pub(crate) next_stop: Option<LiftStopId>,
}

impl LiftLine {
    pub(crate) fn new(
        stops: Vec<LiftStopId>,
        capacity: NonZeroU32,
        travel_ticks: NonZeroU64,
        door_ticks: NonZeroU64,
    ) -> Result<Self, LiftError> {
        if !valid_stops(&stops) {
            return Err(LiftError::InvalidStops);
        }
        Ok(Self {
            stops,
            capacity,
            travel_ticks,
            door_ticks,
            motion: Motion::parked(0),
            direction: None,
            waiting: BTreeMap::new(),
            riders: BTreeMap::new(),
        })
    }

    pub(crate) fn stops(&self) -> &[LiftStopId] {
        &self.stops
    }

    pub(crate) fn car_stop(&self) -> LiftStopId {
        self.stops[self.motion.last_stop()]
    }

    /// Returns the endpoints and arrival tick of a leg with two surviving stops.
    pub(crate) fn car_motion(&self) -> Option<(LiftStopId, LiftStopId, u64)> {
        let Motion::Moving {
            from: Some(from),
            to,
            arrival,
        } = self.motion
        else {
            return None;
        };
        Some((self.stops[from], self.stops[to], arrival))
    }

    pub(crate) fn travel_ticks(&self) -> u64 {
        self.travel_ticks.get()
    }

    pub(crate) fn waiting_people(
        &self,
    ) -> impl Iterator<Item = (PersonId, LiftStopId, LiftStopId, u64)> + '_ {
        self.waiting
            .iter()
            .map(|(&person, request)| (person, request.from, request.to, request.arrival))
    }

    pub(crate) fn riding_people(&self) -> impl Iterator<Item = PersonId> + '_ {
        self.riders.keys().copied()
    }

    pub(crate) fn rider_destination(&self, person: PersonId) -> Option<LiftStopId> {
        self.riders
            .get(&person)
            .map(|destination| destination.stop())
    }

    fn stop_index(&self, stop: LiftStopId) -> Option<usize> {
        self.stops.iter().position(|&candidate| candidate == stop)
    }

    fn accepts(&self, from: LiftStopId, to: LiftStopId) -> bool {
        from != to && self.stops.contains(&from) && self.stops.contains(&to)
    }

    fn valid_motion(&self) -> bool {
        if self.motion.last_stop() >= self.stops.len()
            || self.motion.next_stop() >= self.stops.len()
        {
            return false;
        }
        match self.motion {
            Motion::At { stop, .. } => {
                self.direction.is_some()
                    || self
                        .riders
                        .values()
                        .all(|rider| rider.stop() == self.stops[stop])
            }
            Motion::Moving {
                from: Some(from),
                to,
                ..
            } => from.abs_diff(to) == 1 && self.direction == Direction::between(from, to),
            Motion::Moving { from: None, to, .. } => {
                to == self.stops.len() - 1 && self.direction == Some(Direction::Down)
            }
        }
    }

    fn has_seat(&self) -> bool {
        self.riders.len() < self.capacity.get() as usize
    }

    fn has_exit(&self, stop: LiftStopId) -> bool {
        self.riders.values().any(|rider| rider.stop() == stop)
    }

    fn can_board(&self, stop: usize, request: RideRequest, tick: u64) -> bool {
        request.arrival <= tick
            && request.from == self.stops[stop]
            && self.direction.is_some_and(|direction| {
                direction.leads_to(
                    stop,
                    self.stop_index(request.to).expect("validated ride request"),
                )
            })
    }

    fn has_boarding(&self, stop: usize, tick: u64) -> bool {
        self.has_seat()
            && self
                .waiting
                .values()
                .any(|&request| self.can_board(stop, request, tick))
    }

    fn has_demand(&self, stop: usize, direction: Direction, tick: u64) -> bool {
        self.riders.values().any(|rider| {
            direction.leads_to(
                stop,
                self.stop_index(rider.stop())
                    .expect("validated destination"),
            )
        }) || self
            .waiting
            .values()
            .filter(|request| request.arrival <= tick)
            .any(|request| {
                let from = self
                    .stop_index(request.from)
                    .expect("validated ride request");
                let to = self.stop_index(request.to).expect("validated ride request");
                direction.leads_to(stop, from) || (from == stop && direction.leads_to(stop, to))
            })
    }

    fn choose_direction(&self, stop: usize, tick: u64) -> Option<Direction> {
        if let Some(direction) = self.direction {
            return [direction, direction.opposite()]
                .into_iter()
                .find(|&candidate| self.has_demand(stop, candidate, tick));
        }
        let (_, oldest) = self
            .waiting
            .iter()
            .filter(|(_, request)| request.arrival <= tick)
            .min_by_key(|(person, request)| (request.arrival, **person))?;
        let from = self
            .stop_index(oldest.from)
            .expect("validated ride request");
        let to = self.stop_index(oldest.to).expect("validated ride request");
        Direction::between(stop, if from == stop { to } else { from })
    }

    fn open_doors(&mut self, tick: u64) {
        if let Motion::Moving { to, arrival, .. } = self.motion
            && arrival <= tick
        {
            self.motion = Motion::parked(to);
        }
        let Motion::At { stop, doors_until } = self.motion else {
            return;
        };
        self.direction = self.choose_direction(stop, tick);
        if doors_until.is_none()
            && (self.has_exit(self.stops[stop]) || self.has_boarding(stop, tick))
        {
            self.motion = Motion::At {
                stop,
                doors_until: Some(tick.saturating_add(self.door_ticks.get())),
            };
        }
    }

    fn exit_candidates(&self) -> impl Iterator<Item = (PersonId, Destination)> + '_ {
        let stop = self.motion.open_stop().map(|index| self.stops[index]);
        self.riders.iter().filter_map(move |(&person, &rider)| {
            (Some(rider.stop()) == stop).then_some((person, rider))
        })
    }

    fn entry_candidates(
        &self,
        tick: u64,
    ) -> impl Iterator<Item = (u64, PersonId, LiftStopId)> + '_ {
        self.waiting.iter().filter_map(move |(&person, &request)| {
            let stop = self.motion.open_stop()?;
            (self.has_seat() && self.can_board(stop, request, tick)).then_some((
                request.arrival,
                person,
                request.from,
            ))
        })
    }

    fn depart(&mut self, tick: u64) {
        let Motion::At { stop, doors_until } = self.motion else {
            return;
        };
        if doors_until.is_some_and(|until| {
            tick < until || self.has_exit(self.stops[stop]) || self.has_boarding(stop, tick)
        }) {
            return;
        }
        self.direction = self.choose_direction(stop, tick);
        let Some(to) = self
            .direction
            .and_then(|direction| direction.next(stop, self.stops.len()))
        else {
            self.motion = Motion::parked(stop);
            return;
        };
        self.motion = Motion::Moving {
            from: Some(stop),
            to,
            arrival: tick.saturating_add(self.travel_ticks()),
        };
    }
}

impl LiftSystem {
    pub(crate) fn insert(&mut self, id: LiftId, line: LiftLine) -> Result<(), LiftError> {
        match self.lines.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(line);
                Ok(())
            }
            Entry::Occupied(_) => Err(LiftError::DuplicateLift),
        }
    }

    pub(crate) fn lines(&self) -> impl Iterator<Item = (LiftId, &LiftLine)> {
        self.lines.iter().map(|(&id, line)| (id, line))
    }

    pub(crate) fn request(
        &mut self,
        id: LiftId,
        person: PersonId,
        from: LiftStopId,
        to: LiftStopId,
        tick: u64,
    ) -> Result<(), LiftError> {
        let line = self.lines.get(&id).ok_or(LiftError::UnknownLift)?;
        if !line.accepts(from, to) {
            return Err(LiftError::InvalidStops);
        }
        if self
            .lines
            .values()
            .any(|line| line.waiting.contains_key(&person) || line.riders.contains_key(&person))
        {
            return Err(LiftError::DuplicatePerson);
        }
        self.lines
            .get_mut(&id)
            .expect("lift checked above")
            .waiting
            .insert(
                person,
                RideRequest {
                    from,
                    to,
                    arrival: tick,
                },
            );
        Ok(())
    }

    pub(crate) fn step(&mut self, tick: u64) -> Vec<LiftEvent> {
        for line in self.lines.values_mut() {
            line.open_doors(tick);
        }
        let mut events = Vec::new();
        let mut access = AccessReservations {
            widths: &self.access_widths,
            groups: &self.access_groups,
            occupied: BTreeMap::new(),
        };

        // Reserve the shared access for exiting passengers before any neighboring door admits a
        // waiting person. Unserved passengers remain on their side of the doorway until next tick.
        for (&lift, line) in &mut self.lines {
            let exits: Vec<_> = line.exit_candidates().collect();
            for (person, destination) in exits {
                let stop = destination.stop();
                if !access.reserve(stop) {
                    continue;
                }
                line.riders.remove(&person);
                events.push(match destination {
                    Destination::Requested(_) => LiftEvent::Exited { lift, person, stop },
                    Destination::Replan(_) => LiftEvent::Replan {
                        lift,
                        person,
                        stop: Some(stop),
                    },
                });
            }
        }

        let mut entries: Vec<_> = self
            .lines
            .iter()
            .flat_map(|(&lift, line)| {
                line.entry_candidates(tick)
                    .map(move |(arrival, person, stop)| (arrival, person, lift, stop))
            })
            .collect();
        entries.sort_unstable();
        for (_, person, lift, stop) in entries {
            let line = self
                .lines
                .get_mut(&lift)
                .expect("lift with candidate exists");
            if !line.has_seat() || !access.reserve(stop) {
                continue;
            }
            let request = line
                .waiting
                .remove(&person)
                .expect("candidate is still queued");
            line.riders
                .insert(person, Destination::Requested(request.to));
            events.push(LiftEvent::Boarded { lift, person, stop });
        }
        for line in self.lines.values_mut() {
            line.depart(tick);
        }
        events
    }

    pub(crate) fn set_access_width(&mut self, stop: LiftStopId, width: NonZeroU32) {
        let access = access_group(&self.access_groups, stop);
        if width == NonZeroU32::MIN {
            self.access_widths.remove(&access);
        } else {
            self.access_widths.insert(access, width);
        }
    }

    /// Stops in the same group share stand and passage capacity across cabin doors.
    pub(crate) fn set_access_group(&mut self, stop: LiftStopId, representative: LiftStopId) {
        if stop == representative {
            self.access_groups.remove(&stop);
        } else {
            self.access_groups.insert(stop, representative);
        }
    }

    pub(crate) fn cancel(&mut self, person: PersonId) {
        for line in self.lines.values_mut() {
            if line.waiting.remove(&person).is_some() || line.riders.remove(&person).is_some() {
                break;
            }
        }
    }

    pub(crate) fn affected_by_interrupt(
        &self,
        id: LiftId,
        removed: &BTreeSet<LiftStopId>,
    ) -> Vec<InterruptedRide> {
        let Some(line) = self.lines.get(&id) else {
            return Vec::new();
        };
        let Some(change) = Interruption::new(line, removed) else {
            return Vec::new();
        };
        let surviving = &line.stops[..change.retained];
        let departure_removed = line.motion.last_stop() >= change.retained;
        let mut rides: Vec<_> = line
            .waiting
            .iter()
            .filter(|(_, request)| !change.keeps_request(surviving, **request))
            .map(|(&person, request)| InterruptedRide {
                person,
                destination: request.to,
                last_stop: request.from,
                next_stop: (!removed.contains(&request.from)).then_some(request.from),
            })
            .chain(line.riders.iter().filter_map(|(&person, destination)| {
                let destination = destination.stop();
                let next_stop = match change.cabin {
                    CabinChange::Relocate { stop } => stop,
                    CabinChange::Continue { next }
                        if departure_removed || !surviving.contains(&destination) =>
                    {
                        Some(next)
                    }
                    CabinChange::Continue { .. } => return None,
                };
                Some(InterruptedRide {
                    person,
                    destination,
                    last_stop: line.car_stop(),
                    next_stop,
                })
            }))
            .collect();
        rides.sort_unstable_by_key(|ride| ride.person);
        rides
    }

    pub(crate) fn interrupt(
        &mut self,
        id: LiftId,
        removed: &BTreeSet<LiftStopId>,
    ) -> Vec<LiftEvent> {
        let Some(line) = self.lines.get_mut(&id) else {
            return Vec::new();
        };
        let Some(change) = Interruption::new(line, removed) else {
            return Vec::new();
        };
        let surviving = &line.stops[..change.retained];
        let mut events = Vec::new();
        line.waiting.retain(|&person, request| {
            if change.keeps_request(surviving, *request) {
                return true;
            }
            let stop = (!removed.contains(&request.from)).then_some(request.from);
            events.push(LiftEvent::Replan {
                lift: id,
                person,
                stop,
            });
            false
        });

        match change.cabin {
            CabinChange::Relocate { stop } => {
                events.extend(std::mem::take(&mut line.riders).into_keys().map(|person| {
                    LiftEvent::Replan {
                        lift: id,
                        person,
                        stop,
                    }
                }));
                if change.active() {
                    line.motion = Motion::parked(change.retained - 1);
                }
            }
            CabinChange::Continue { next } => {
                if let Motion::Moving { from, .. } = &mut line.motion {
                    *from = from.filter(|&start| start < change.retained);
                }
                for destination in line.riders.values_mut() {
                    if !surviving.contains(&destination.stop()) {
                        *destination = Destination::Replan(next);
                    }
                }
            }
        }

        if change.active() {
            line.stops.truncate(change.retained);
        } else {
            self.lines.remove(&id);
        }
        events
    }

    pub(crate) fn estimated_wait(&self, id: LiftId, stop: LiftStopId) -> u64 {
        let Some(line) = self.lines.get(&id) else {
            return u64::MAX;
        };
        let Some(index) = line.stop_index(stop) else {
            return u64::MAX;
        };
        let access = access_group(&self.access_groups, stop);
        let access_width = self
            .access_widths
            .get(&access)
            .copied()
            .unwrap_or(NonZeroU32::MIN);
        let people_at_stop = self
            .lines
            .values()
            .flat_map(|line| line.waiting.values())
            .filter(|request| access_group(&self.access_groups, request.from) == access)
            .count() as u64;
        let distance = line.motion.last_stop().abs_diff(index) as u64;
        let door_delay = if distance == 0 {
            0
        } else {
            line.door_ticks.get()
        };
        distance
            .saturating_mul(line.travel_ticks())
            .saturating_add(door_delay)
            .saturating_add(people_at_stop / u64::from(access_width.get()))
    }

    pub(crate) fn waiting_count(&self, id: LiftId) -> usize {
        self.lines.get(&id).map_or(0, |line| line.waiting.len())
    }

    pub(crate) fn rider_count(&self, id: LiftId) -> usize {
        self.lines.get(&id).map_or(0, |line| line.riders.len())
    }
}

#[derive(Clone, Copy)]
enum CabinChange {
    Continue { next: LiftStopId },
    Relocate { stop: Option<LiftStopId> },
}

struct Interruption {
    retained: usize,
    cabin: CabinChange,
}

impl Interruption {
    fn new(line: &LiftLine, removed: &BTreeSet<LiftStopId>) -> Option<Self> {
        let retained = line
            .stops
            .iter()
            .take_while(|stop| !removed.contains(stop))
            .count();
        if retained == line.stops.len() {
            return None;
        }
        let target = line.motion.next_stop();
        let cabin = if retained >= 2 && target < retained {
            let next = match line.motion {
                Motion::Moving { to, .. } => to,
                Motion::At { stop, .. } => line
                    .direction
                    .and_then(|direction| direction.next(stop, retained))
                    .unwrap_or(stop),
            };
            CabinChange::Continue {
                next: line.stops[next],
            }
        } else if retained >= 2 {
            CabinChange::Relocate {
                stop: Some(line.stops[retained - 1]),
            }
        } else {
            let accessible = line
                .stops
                .iter()
                .enumerate()
                .filter(|(_, stop)| !removed.contains(stop));
            let distance = |(index, _): &(usize, &LiftStopId)| index.abs_diff(target);
            // If the whole line shuts down, prefer a still accessible stop on the current leg.
            let forward = line.direction.and_then(|direction| {
                accessible
                    .clone()
                    .filter(|(index, _)| !direction.opposite().leads_to(target, *index))
                    .min_by_key(distance)
            });
            let stop = forward
                .or_else(|| accessible.min_by_key(distance))
                .map(|(_, &stop)| stop);
            CabinChange::Relocate { stop }
        };
        Some(Self { retained, cabin })
    }

    fn active(&self) -> bool {
        self.retained >= 2
    }

    fn keeps_request(&self, surviving: &[LiftStopId], request: RideRequest) -> bool {
        self.active() && surviving.contains(&request.from) && surviving.contains(&request.to)
    }
}

impl TryFrom<StoredSystem> for LiftSystem {
    type Error = LiftError;

    fn try_from(snapshot: StoredSystem) -> Result<Self, Self::Error> {
        let mut reserved = BTreeSet::new();
        for line in snapshot.lines.values() {
            if !valid_stops(&line.stops)
                || !line.valid_motion()
                || line.riders.len() > line.capacity.get() as usize
                || !line.waiting.iter().all(|(&person, request)| {
                    line.accepts(request.from, request.to) && reserved.insert(person)
                })
                || !line.riders.iter().all(|(&person, destination)| {
                    line.stops.contains(&destination.stop()) && reserved.insert(person)
                })
            {
                return Err(LiftError::InvalidState);
            }
        }
        Ok(Self {
            lines: snapshot.lines,
            access_widths: snapshot.access_widths,
            access_groups: snapshot.access_groups,
        })
    }
}

fn valid_stops(stops: &[LiftStopId]) -> bool {
    stops.len() >= 2 && stops.iter().copied().collect::<BTreeSet<_>>().len() == stops.len()
}

impl Motion {
    fn parked(stop: usize) -> Self {
        Self::At {
            stop,
            doors_until: None,
        }
    }

    fn last_stop(self) -> usize {
        match self {
            Self::At { stop, .. } => stop,
            Self::Moving { from, to, .. } => from.unwrap_or(to),
        }
    }

    fn next_stop(self) -> usize {
        match self {
            Self::At { stop, .. } => stop,
            Self::Moving { to, .. } => to,
        }
    }

    fn open_stop(self) -> Option<usize> {
        match self {
            Self::At {
                stop,
                doors_until: Some(_),
            } => Some(stop),
            _ => None,
        }
    }
}

impl Direction {
    fn between(from: usize, to: usize) -> Option<Self> {
        use std::cmp::Ordering;
        match from.cmp(&to) {
            Ordering::Less => Some(Self::Up),
            Ordering::Greater => Some(Self::Down),
            Ordering::Equal => None,
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }

    fn leads_to(self, from: usize, to: usize) -> bool {
        Self::between(from, to) == Some(self)
    }

    fn next(self, from: usize, stop_count: usize) -> Option<usize> {
        match self {
            Self::Up => from.checked_add(1).filter(|&next| next < stop_count),
            Self::Down => from.checked_sub(1),
        }
    }
}

impl Destination {
    fn stop(self) -> LiftStopId {
        match self {
            Self::Requested(stop) | Self::Replan(stop) => stop,
        }
    }
}

struct AccessReservations<'a> {
    widths: &'a BTreeMap<LiftStopId, NonZeroU32>,
    groups: &'a BTreeMap<LiftStopId, LiftStopId>,
    occupied: BTreeMap<LiftStopId, u32>,
}

impl AccessReservations<'_> {
    fn reserve(&mut self, stop: LiftStopId) -> bool {
        let access = access_group(self.groups, stop);
        let width = self.widths.get(&access).copied().unwrap_or(NonZeroU32::MIN);
        let occupied = self.occupied.entry(access).or_default();
        if *occupied >= width.get() {
            return false;
        }
        *occupied += 1;
        true
    }
}

fn access_group(groups: &BTreeMap<LiftStopId, LiftStopId>, stop: LiftStopId) -> LiftStopId {
    groups.get(&stop).copied().unwrap_or(stop)
}

#[cfg(test)]
mod tests;
