use crate::lift::LiftSystem;
use crate::routing::{RouteLeg, walk, walking_edges};
use crate::{
    Action, CommandNumber, CommandResult, DataError, Designation, HouseholdId, Impact, Outcome,
    PersonId, Place, PlaceId, PlaceKind, Rejection, Scenario, ScheduledCommand, SectionId,
    SubmitError,
};
use crate::{LiftId, LiftStopId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkSegment {
    pub from: PlaceId,
    pub to: PlaceId,
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    At {
        place: PlaceId,
    },
    Walking(WalkSegment),
    WaitingLift {
        place: PlaceId,
        lift: LiftId,
        from: LiftStopId,
        to: LiftStopId,
        arrival_tick: u64,
    },
    RidingLift {
        place: PlaceId,
        lift: LiftId,
        from: LiftStopId,
        to: LiftStopId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub home: PlaceId,
    pub office: Option<PlaceId>,
    pub household: HouseholdId,
    pub location: Location,
    pub(crate) destination: Option<PlaceId>,
    pub(crate) remaining_route: VecDeque<RouteLeg>,
    pub(crate) waited_ticks: u64,
    pub(crate) frustration: u32,
    pub(crate) dissatisfaction: u32,
    pub(crate) last_lift: Option<LiftId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreState {
    pub(crate) tick: u64,
    pub(crate) scenario_checksum: String,
    pub(crate) random_state: u64,
    pub(crate) balance: i64,
    pub(crate) rented_sections: BTreeSet<SectionId>,
    pub(crate) households: BTreeMap<HouseholdId, PlaceId>,
    pub(crate) designations: BTreeMap<PlaceId, Designation>,
    pub(crate) waiting_households: BTreeMap<HouseholdId, std::num::NonZeroU32>,
    pub(crate) next_person_id: u64,
    pub(crate) development_points: u32,
    pub(crate) first_population_awarded: bool,
    pub(crate) lift_unlocked: bool,
    pub(crate) lifts: LiftSystem,
    pub(crate) express_lifts: BTreeSet<LiftId>,
    pub(crate) next_lift_id: u64,
    pub(crate) route_searches: u64,
    pub(crate) lift_boardings: u64,
    pub(crate) lift_transfers: u64,
    pub(crate) total_waited_ticks: u64,
    pub(crate) longest_queue: usize,
    pub(crate) coffee_trips: BTreeMap<PersonId, crate::initiative::CoffeeTrip>,
    pub(crate) coffee_observations: BTreeMap<PlaceId, BTreeMap<PlaceId, u32>>,
    pub(crate) initiatives: BTreeMap<PlaceId, BTreeMap<PlaceId, crate::initiative::Slide>>,
    pub(crate) people: BTreeMap<PersonId, Person>,
    pub(crate) pending: BTreeMap<u64, BTreeMap<CommandNumber, Action>>,
    pub(crate) submitted_numbers: BTreeSet<CommandNumber>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TransportStats {
    pub route_searches: u64,
    pub waiting_people: usize,
    pub lift_boardings: u64,
    pub lift_transfers: u64,
    pub longest_queue: usize,
    pub total_waited_ticks: u64,
}

pub struct World {
    pub(crate) scenario: Scenario,
    pub(crate) state: CoreState,
    pub(crate) last_report: TickReport,
}

impl World {
    pub fn new(scenario: Scenario, seed: u64) -> Result<Self, DataError> {
        let initial = scenario.initial();
        let edges =
            walking_edges(&scenario, |place| {
                scenario.geometry().places.iter().any(|entry| {
                    entry.id == place && initial.rented_sections.contains(&entry.section)
                })
            });
        let people: BTreeMap<_, _> = initial
            .people
            .iter()
            .map(|person| {
                let route = walk(&edges, person.location, person.office, 0).unwrap_or_default();
                let mut rider = Person {
                    home: person.home,
                    office: Some(person.office),
                    household: person.household.unwrap_or(HouseholdId(person.id.0)),
                    location: Location::At {
                        place: person.location,
                    },
                    destination: Some(person.office),
                    remaining_route: route,
                    waited_ticks: 0,
                    frustration: 0,
                    dissatisfaction: 0,
                    last_lift: None,
                };
                rider.begin_initial_walk(person.location, 0);
                (person.id, rider)
            })
            .collect();
        let households = people
            .values()
            .map(|person| (person.household, person.home))
            .collect();
        let waiting_households = initial
            .applicants
            .iter()
            .map(|applicant| (applicant.id, applicant.size))
            .collect();
        let next_person_id = initial
            .people
            .last()
            .map_or(1, |person| u64::from(person.id.0) + 1);
        let designations = scenario
            .geometry()
            .places
            .iter()
            .filter_map(|place| {
                let usage = match place.kind {
                    PlaceKind::Space => Designation::Vacant,
                    PlaceKind::Home => Designation::Home,
                    PlaceKind::Office => Designation::Office,
                    PlaceKind::Landing | PlaceKind::Reception => return None,
                };
                Some((place.id, usage))
            })
            .collect();
        let state = CoreState {
            tick: 0,
            scenario_checksum: checksum(&scenario),
            random_state: seed,
            balance: initial.balance,
            rented_sections: initial.rented_sections.iter().copied().collect(),
            people,
            households,
            waiting_households,
            next_person_id,
            designations,
            development_points: 0,
            first_population_awarded: false,
            lift_unlocked: false,
            lifts: LiftSystem::default(),
            express_lifts: BTreeSet::new(),
            next_lift_id: 1,
            route_searches: 0,
            lift_boardings: 0,
            lift_transfers: 0,
            total_waited_ticks: 0,
            longest_queue: 0,
            coffee_trips: BTreeMap::new(),
            coffee_observations: BTreeMap::new(),
            initiatives: BTreeMap::new(),
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

    pub fn submit_next(&mut self, action: Action) -> Result<(), SubmitError> {
        let tick = self.tick().checked_add(1).ok_or(SubmitError::Exhausted)?;
        let number = self
            .state
            .submitted_numbers
            .last()
            .map_or(Some(1), |number| number.0.checked_add(1))
            .ok_or(SubmitError::Exhausted)?;
        self.submit(ScheduledCommand {
            tick,
            number: CommandNumber(number),
            action,
        })
    }

    pub fn person_location(&self, id: PersonId) -> Option<Location> {
        self.state.people.get(&id).map(|person| person.location)
    }

    pub fn transport_stats(&self) -> TransportStats {
        TransportStats {
            route_searches: self.state.route_searches,
            waiting_people: self
                .state
                .lifts
                .lines()
                .map(|(lift, _)| self.state.lifts.waiting_count(lift))
                .sum(),
            lift_boardings: self.state.lift_boardings,
            lift_transfers: self.state.lift_transfers,
            longest_queue: self.state.longest_queue,
            total_waited_ticks: self.state.total_waited_ticks,
        }
    }

    pub fn tick_report(&self) -> &TickReport {
        &self.last_report
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
                outcome: match self.apply(&action) {
                    Ok(()) => Outcome::Applied,
                    Err(reason) => Outcome::Rejected { reason },
                },
            })
            .collect();

        self.progress_transport();
        self.daily_commutes();
        self.ease_frustration();
        self.find_tenants();
        self.settle_day();
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

    pub(crate) fn place(&self, place: PlaceId) -> Option<&Place> {
        let places = &self.scenario.geometry().places;
        places
            .binary_search_by_key(&place, |entry| entry.id)
            .ok()
            .map(|index| &places[index])
    }

    pub(crate) fn access(&self, place: PlaceId) -> bool {
        self.place(place)
            .is_some_and(|entry| self.is_rented(entry.section))
    }

    pub fn preview(&self, action: &Action) -> Result<Impact, Rejection> {
        let (cost, section_rent_change) = match *action {
            Action::RentSection { section } => {
                let definition = self
                    .scenario
                    .geometry()
                    .sections
                    .iter()
                    .find(|candidate| candidate.id == section)
                    .ok_or(Rejection::UnknownSection)?;
                if self.is_rented(section) {
                    return Err(Rejection::AlreadyRented);
                }
                (definition.rent_cost, i64::from(definition.rent_cost))
            }
            Action::ReturnSection { section, .. } => return self.preview_return(section),
            Action::CoffeeTrip { person, kiosk } => return self.preview_coffee(person, kiosk),
            Action::UnlockLift => return self.preview_unlock(),
            Action::BuildLift { ref stops } => return self.preview_lift(stops, false),
            Action::BuildExpress { ref stops } => return self.preview_lift(stops, true),
            Action::Commute { person, toward } => {
                let goal = self.commute_destination(person, toward)?;
                self.planned_route(person, goal)?;
                return Ok(Impact::default());
            }
            Action::Designate { place, designation } => {
                let entry = self.place(place).ok_or(Rejection::UnknownPlace)?;
                if entry.kind != PlaceKind::Space {
                    return Err(Rejection::FixedPlace);
                }
                if !self.is_rented(entry.section) {
                    return Err(Rejection::SectionNotRented);
                }
                if self.state.designations[&place] == designation {
                    return Err(Rejection::AlreadyDesignated);
                }
                if self
                    .state
                    .people
                    .values()
                    .any(|person| person.home == place || person.office == Some(place))
                {
                    return Err(Rejection::PlaceOccupied);
                }
                (
                    self.scenario
                        .rules
                        .map_or(0, |rules| rules.designation_cost),
                    0,
                )
            }
        };
        if self.state.balance < i64::from(cost) {
            return Err(Rejection::InsufficientFunds);
        }
        Ok(Impact {
            immediate_cost: cost,
            section_rent_change,
            ..Impact::default()
        })
    }

    fn apply(&mut self, action: &Action) -> Result<(), Rejection> {
        let impact = self.preview(action)?;
        match *action {
            Action::RentSection { section } => {
                self.state.rented_sections.insert(section);
            }
            Action::Designate { place, designation } => {
                self.state.designations.insert(place, designation);
                self.refresh_access_widths();
            }
            Action::ReturnSection {
                section,
                ref confirmation,
            } => {
                return self.return_section(section, confirmation);
            }
            Action::CoffeeTrip { person, kiosk } => self.start_coffee(person, kiosk)?,
            Action::UnlockLift => {
                self.state.development_points -= 1;
                self.state.lift_unlocked = true;
            }
            Action::BuildLift { .. } => self.install_lift(impact.stops, false)?,
            Action::BuildExpress { .. } => self.install_lift(impact.stops, true)?,
            Action::Commute { person, toward } => {
                let goal = self.commute_destination(person, toward)?;
                self.send_to(person, goal)?;
            }
        }
        self.state.balance -= i64::from(impact.immediate_cost);
        Ok(())
    }
}

pub(crate) fn checksum(state: &impl Serialize) -> String {
    // Core snapshots consist exclusively of JSON-compatible values and ordered maps.
    let bytes =
        serde_json::to_vec(state).expect("core snapshots contain only JSON-compatible data");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    Sha256::digest(bytes)
        .into_iter()
        .flat_map(|byte| [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 15)]])
        .map(char::from)
        .collect()
}
