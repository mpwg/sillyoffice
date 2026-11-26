use crate::routing::{walk, walking_edges};
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

        let edges = walking_edges(&self.scenario, |place| self.access(place));
        let leased_homes: BTreeSet<_> = self.state.households.values().copied().collect();
        let leased_offices: BTreeSet<_> = self
            .state
            .people
            .values()
            .filter_map(|person| person.office)
            .collect();
        let mut offices = self.available(Designation::Office, &leased_offices);
        let homes = self.available(Designation::Home, &leased_homes);
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

        for person in self
            .state
            .people
            .values_mut()
            .filter(|p| p.office.is_none())
        {
            let Location::At { place } = person.location else {
                continue;
            };
            let Some((office, route)) = offices
                .iter()
                .find_map(|&office| walk(&edges, place, office, tick).map(|route| (office, route)))
            else {
                continue;
            };
            offices.remove(&office);
            person.office = Some(office);
            person.begin_walk(place, route);
        }
        for (household, size) in applicants {
            let size_usize = size as usize;
            let Some((home, assignments)) = homes
                .iter()
                .filter(|home| !self.state.households.values().any(|leased| leased == *home))
                .find_map(|&home| {
                    walk(&edges, reception, home, tick)?;
                    let assignments: Vec<_> = offices
                        .iter()
                        .filter_map(|&office| {
                            walk(&edges, home, office, tick)?;
                            walk(&edges, reception, office, tick).map(|route| (office, route))
                        })
                        .take(size_usize)
                        .collect();
                    (assignments.len() == size_usize).then_some((home, assignments))
                })
            else {
                continue;
            };
            if self.state.next_person_id + u64::from(size) > u64::from(u32::MAX) + 1 {
                break;
            }
            for (office, route) in assignments {
                let id = PersonId(self.state.next_person_id as u32);
                self.state.next_person_id += 1;
                offices.remove(&office);
                let mut person = Person {
                    home,
                    office: Some(office),
                    household,
                    location: Location::At { place: reception },
                    remaining_walk: Default::default(),
                };
                person.begin_walk(reception, route);
                self.state.people.insert(id, person);
            }
            self.state.households.insert(household, home);
            self.state.waiting_households.remove(&household);
        }
        self.award_population_points(rules.population_points);
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

impl Person {
    pub(crate) fn begin_walk(
        &mut self,
        from: PlaceId,
        mut route: std::collections::VecDeque<crate::WalkSegment>,
    ) {
        self.location = route
            .pop_front()
            .map_or(Location::At { place: from }, Location::Walking);
        self.remaining_walk = route;
    }
}
