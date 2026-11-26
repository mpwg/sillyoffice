use crate::{
    Action, CommandNumber, CommandResult, DataError, Outcome, PersonId, PlaceId, Rejection,
    Scenario, ScheduledCommand, SectionId, SubmitError,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WalkSegment {
    pub from: PlaceId,
    pub to: PlaceId,
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    At { place: PlaceId },
    Walking(WalkSegment),
}

#[derive(Debug, Clone, Serialize)]
pub struct Person {
    pub home: PlaceId,
    pub office: PlaceId,
    pub location: Location,
    remaining_walk: VecDeque<WalkSegment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoreState {
    tick: u64,
    scenario_checksum: String,
    random_state: u64,
    balance: i64,
    rented_sections: BTreeSet<SectionId>,
    pub(crate) people: BTreeMap<PersonId, Person>,
    pending: BTreeMap<u64, BTreeMap<CommandNumber, Action>>,
    submitted_numbers: BTreeSet<CommandNumber>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TickReport {
    pub tick: u64,
    pub results: Vec<CommandResult>,
}

#[derive(Debug, Serialize)]
pub struct Checkpoint {
    #[serde(flatten)]
    pub report: TickReport,
    pub checksum: String,
    pub state: CoreState,
}

pub struct World {
    pub(crate) scenario: Scenario,
    pub(crate) state: CoreState,
    last_report: TickReport,
}

impl World {
    pub fn new(scenario: Scenario, seed: u64) -> Result<Self, DataError> {
        let edges = walking_edges(&scenario);
        let initial = scenario.initial();
        let people = initial
            .people
            .iter()
            .map(|person| {
                let mut walk = first_walk(&edges, person.location, person.office);
                let location = walk.pop_front().map_or(
                    Location::At {
                        place: person.location,
                    },
                    Location::Walking,
                );
                (
                    person.id,
                    Person {
                        home: person.home,
                        office: person.office,
                        location,
                        remaining_walk: walk,
                    },
                )
            })
            .collect();
        let state = CoreState {
            tick: 0,
            scenario_checksum: checksum(&scenario),
            random_state: seed,
            balance: initial.balance,
            rented_sections: initial.rented_sections.iter().copied().collect(),
            people,
            pending: BTreeMap::new(),
            submitted_numbers: BTreeSet::new(),
        };
        Ok(Self {
            scenario,
            state,
            last_report: TickReport {
                tick: 0,
                results: Vec::new(),
            },
        })
    }

    pub fn tick(&self) -> u64 {
        self.state.tick
    }

    pub fn submit(&mut self, command: ScheduledCommand) -> Result<(), SubmitError> {
        if command.tick <= self.tick() {
            return Err(SubmitError::PastTick {
                number: command.number,
                target: command.tick,
                current: self.tick(),
            });
        }
        if !self.state.submitted_numbers.insert(command.number) {
            return Err(SubmitError::DuplicateNumber(command.number));
        }
        self.state
            .pending
            .entry(command.tick)
            .or_default()
            .insert(command.number, command.action);
        Ok(())
    }

    pub fn step(&mut self) -> TickReport {
        let Some(tick) = self.tick().checked_add(1) else {
            return self.last_report.clone();
        };
        self.state.tick = tick;
        let results = self
            .state
            .pending
            .remove(&tick)
            .into_iter()
            .flatten()
            .map(|(number, action)| CommandResult {
                number,
                outcome: match self.apply(action) {
                    Ok(()) => Outcome::Applied,
                    Err(reason) => Outcome::Rejected { reason },
                },
            })
            .collect();

        for person in self.state.people.values_mut() {
            if let Location::Walking(segment) = person.location
                && segment.end_tick == tick
            {
                person.location = person
                    .remaining_walk
                    .pop_front()
                    .map_or(Location::At { place: segment.to }, Location::Walking);
            }
        }

        self.last_report = TickReport { tick, results };
        self.last_report.clone()
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            report: self.last_report.clone(),
            checksum: checksum(&self.state),
            state: self.state.clone(),
        }
    }

    pub(crate) fn is_rented(&self, section: SectionId) -> bool {
        self.state.rented_sections.contains(&section)
    }

    fn apply(&mut self, action: Action) -> Result<(), Rejection> {
        let Action::RentSection { section } = action;
        let definition = self
            .scenario
            .geometry()
            .sections
            .iter()
            .find(|candidate| candidate.id == section)
            .ok_or(Rejection::UnknownSection)?;
        let cost = i64::from(definition.rent_cost);
        if self.is_rented(section) {
            return Err(Rejection::AlreadyRented);
        }
        if self.state.balance < cost {
            return Err(Rejection::InsufficientFunds);
        }
        self.state.balance -= cost;
        self.state.rented_sections.insert(section);
        Ok(())
    }
}

fn checksum(state: &impl Serialize) -> String {
    // These snapshots contain integers, strings, sequences and ordered maps with integer keys.
    let bytes =
        serde_json::to_vec(state).expect("core snapshots contain only JSON-compatible data");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    Sha256::digest(bytes)
        .into_iter()
        .flat_map(|byte| [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 15)]])
        .map(char::from)
        .collect()
}

type WalkingEdges = BTreeMap<PlaceId, Vec<(PlaceId, u64)>>;

fn walking_edges(scenario: &Scenario) -> WalkingEdges {
    let geometry = scenario.geometry();
    let mut edges = WalkingEdges::new();
    for walkway in &geometry.walkways {
        edges
            .entry(walkway.from)
            .or_default()
            .push((walkway.to, walkway.ticks.get()));
        if walkway.bidirectional {
            edges
                .entry(walkway.to)
                .or_default()
                .push((walkway.from, walkway.ticks.get()));
        }
    }
    for stair in &geometry.stairs {
        edges
            .entry(stair.from)
            .or_default()
            .push((stair.to, stair.ticks.get()));
        edges
            .entry(stair.to)
            .or_default()
            .push((stair.from, stair.ticks.get()));
    }
    for outgoing in edges.values_mut() {
        outgoing.sort_unstable();
    }
    edges
}

fn first_walk(edges: &WalkingEdges, start: PlaceId, destination: PlaceId) -> VecDeque<WalkSegment> {
    let mut frontier = BinaryHeap::from([Reverse((0_u64, start))]);
    let mut costs = BTreeMap::from([(start, 0)]);
    let mut previous = BTreeMap::new();
    while let Some(Reverse((cost, from))) = frontier.pop() {
        if from == destination {
            break;
        }
        if costs[&from] != cost {
            continue;
        }
        for &(to, duration) in edges.get(&from).into_iter().flatten() {
            if let Some(end_tick) = cost.checked_add(duration)
                && costs.get(&to).is_none_or(|&known| end_tick < known)
            {
                costs.insert(to, end_tick);
                previous.insert(
                    to,
                    WalkSegment {
                        from,
                        to,
                        start_tick: cost,
                        end_tick,
                    },
                );
                frontier.push(Reverse((end_tick, to)));
            }
        }
    }
    let mut walk = VecDeque::new();
    let mut place = destination;
    while let Some(segment) = previous.get(&place) {
        walk.push_front(*segment);
        place = segment.from;
    }
    walk
}
