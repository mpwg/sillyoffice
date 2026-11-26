use crate::routing::walk;
use crate::{Designation, Location, Person, PersonId, PlaceId, PlaceKind, World};
use std::collections::BTreeSet;

impl World {
    pub(crate) fn find_tenants(&mut self) {
        let Some(rules) = self.scenario.rules else {
            return;
        };
        if self.state.waiting_households.is_empty()
            && self.state.people.values().all(|p| p.office.is_some())
        {
            self.award_population_points(rules.population_points);
            return;
        }

        let edges = self.routes();
        let leased_homes: BTreeSet<_> = self.state.households.values().copied().collect();
        let leased_offices: BTreeSet<_> = self
            .state
            .people
            .values()
            .filter_map(|person| person.office)
            .collect();
        let mut offices = self.available(Designation::Office, &leased_offices);
        let mut homes = self.available(Designation::Home, &leased_homes);
        let applicants: Vec<_> = self
            .state
            .waiting_households
            .iter()
            .map(|(&id, &size)| (id, size.get()))
            .collect();
        let Some(reception) = self.reception() else {
            return;
        };
        let tick = self.tick();

        let unemployed: Vec<_> = self
            .state
            .people
            .iter()
            .filter_map(|(&id, person)| match person.location {
                Location::At { place } if person.office.is_none() => Some((id, place)),
                _ => None,
            })
            .collect();
        for (id, place) in unemployed {
            let Some((office, route)) = offices.iter().find_map(|&office| {
                self.state.route_searches = self.state.route_searches.saturating_add(1);
                walk(&edges, place, office, tick).map(|route| (office, route))
            }) else {
                continue;
            };
            offices.remove(&office);
            self.state.people.get_mut(&id).unwrap().office = Some(office);
            self.start_route(id, place, office, route);
        }
        for (household, size) in applicants {
            let size_usize = size as usize;
            if homes.is_empty() {
                break;
            }
            if offices.len() < size_usize {
                continue;
            }
            let mut choices: Vec<_> = homes
                .iter()
                .filter_map(|&home| {
                    self.state.route_searches = self.state.route_searches.saturating_add(1);
                    walk(&edges, reception, home, tick)?;
                    let assignments: Vec<_> = offices
                        .iter()
                        .filter_map(|&office| {
                            self.state.route_searches = self.state.route_searches.saturating_add(1);
                            walk(&edges, home, office, tick)?;
                            self.state.route_searches = self.state.route_searches.saturating_add(1);
                            walk(&edges, reception, office, tick).map(|route| (office, route))
                        })
                        .take(size_usize)
                        .collect();
                    (assignments.len() == size_usize).then_some((home, assignments))
                })
                .collect();
            if choices.is_empty() {
                continue;
            }
            if self.state.next_person_id + u64::from(size) > u64::from(u32::MAX) + 1 {
                continue;
            }
            let selection = if choices.len() == 1 {
                0
            } else {
                (self.next_random() % choices.len() as u64) as usize
            };
            let (home, assignments) = choices.swap_remove(selection);
            homes.remove(&home);
            for (office, route) in assignments {
                let id = PersonId(self.state.next_person_id as u32);
                self.state.next_person_id += 1;
                offices.remove(&office);
                self.state.people.insert(
                    id,
                    Person {
                        home,
                        office: Some(office),
                        household,
                        location: Location::At { place: reception },
                        remaining_route: Default::default(),
                        destination: None,
                        waited_ticks: 0,
                        frustration: 0,
                        dissatisfaction: 0,
                        last_lift: None,
                    },
                );
                self.start_route(id, reception, office, route);
            }
            self.state.households.insert(household, home);
            self.state.waiting_households.remove(&household);
        }
        self.award_population_points(rules.population_points);
    }

    fn next_random(&mut self) -> u64 {
        // SplitMix64 uses a single saved word and permits an initial seed of zero.
        self.state.random_state = self.state.random_state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state.random_state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn award_population_points(&mut self, points: u32) {
        if !self.state.first_population_awarded && !self.state.people.is_empty() {
            self.state.first_population_awarded = true;
            self.state.development_points += points;
        }
    }

    fn available(&self, designation: Designation, leased: &BTreeSet<PlaceId>) -> BTreeSet<PlaceId> {
        self.state
            .designations
            .iter()
            .filter_map(|(&place, &usage)| {
                (usage == designation && self.access(place) && !leased.contains(&place))
                    .then_some(place)
            })
            .collect()
    }

    pub(crate) fn reception(&self) -> Option<PlaceId> {
        self.scenario
            .geometry()
            .places
            .iter()
            .find(|entry| entry.kind == PlaceKind::Reception && self.is_rented(entry.section))
            .map(|entry| entry.id)
    }
}
