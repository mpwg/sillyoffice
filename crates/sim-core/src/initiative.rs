use crate::routing::{Routes, add_walk, walk};
use crate::view::{VisibleInitiative, VisiblePlace};
use crate::{Designation, Impact, Location, PersonId, PlaceId, PlaceKind, Rejection, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Slide {
    pub(crate) person: PersonId,
    pub(crate) ready_tick: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub(crate) enum CoffeeTrip {
    Outbound {
        office: PlaceId,
        kiosk: PlaceId,
        left: u64,
    },
    Returning {
        office: PlaceId,
    },
}

impl World {
    pub(crate) fn preview_coffee(
        &self,
        person: PersonId,
        kiosk: PlaceId,
    ) -> Result<Impact, Rejection> {
        if self.state.designations.get(&kiosk) != Some(&Designation::Kiosk) {
            return Err(Rejection::NotAKiosk);
        }
        let resident = self
            .state
            .people
            .get(&person)
            .ok_or(Rejection::UnknownPerson)?;
        if !matches!(resident.location, Location::At { place } if Some(place) == resident.office)
            || self.state.coffee_trips.contains_key(&person)
        {
            return Err(Rejection::CoffeeFromOffice);
        }
        self.planned_route(person, kiosk)?;
        Ok(Impact::default())
    }

    pub(crate) fn start_coffee(
        &mut self,
        person: PersonId,
        kiosk: PlaceId,
    ) -> Result<(), Rejection> {
        let (office, route) = self.planned_route(person, kiosk)?;
        self.state.coffee_trips.insert(
            person,
            CoffeeTrip::Outbound {
                office,
                kiosk,
                left: self.tick(),
            },
        );
        self.start_route(person, office, kiosk, route);
        Ok(())
    }

    pub(crate) fn add_initiative_routes(&self, edges: &mut Routes) {
        for (&from, branches) in &self.state.initiatives {
            for (&to, slide) in branches {
                if self.tick() >= slide.ready_tick && self.access(from) && self.access(to) {
                    add_walk(edges, from, to, 1);
                }
            }
        }
    }

    pub(crate) fn visible_initiatives<'a>(
        &'a self,
        places: &'a BTreeMap<PlaceId, VisiblePlace>,
    ) -> impl Iterator<Item = VisibleInitiative> + 'a {
        self.state
            .initiatives
            .iter()
            .flat_map(move |(&from, branches)| {
                branches.iter().map(move |(&to, slide)| VisibleInitiative {
                    from: places[&from],
                    to: places[&to],
                    ready_tick: slide.ready_tick,
                    person: slide.person,
                    built: slide.ready_tick <= self.tick(),
                })
            })
    }

    pub(crate) fn arrived_at(&mut self, id: PersonId, place: PlaceId) {
        let Some(trip) = self.state.coffee_trips.get(&id).copied() else {
            return;
        };
        match trip {
            CoffeeTrip::Outbound {
                office,
                kiosk,
                left,
            } if place == kiosk => {
                self.record_coffee_detour(id, office, kiosk, self.tick() - left);
                if self.send_to(id, office).is_ok() {
                    self.state
                        .coffee_trips
                        .insert(id, CoffeeTrip::Returning { office });
                } else {
                    self.state.coffee_trips.remove(&id);
                }
            }
            CoffeeTrip::Returning { office } if place == office => {
                self.state.coffee_trips.remove(&id);
            }
            _ => {}
        }
    }

    fn record_coffee_detour(
        &mut self,
        person: PersonId,
        office: PlaceId,
        kiosk: PlaceId,
        elapsed: u64,
    ) {
        let Some(rules) = self.scenario.rules else {
            return;
        };
        let Some((from, to, cost)) = self.slide_anchors(office, kiosk) else {
            return;
        };
        if elapsed <= cost
            || self
                .state
                .initiatives
                .get(&from)
                .is_some_and(|paths| paths.contains_key(&to))
        {
            return;
        }
        let observations = self
            .state
            .coffee_observations
            .entry(from)
            .or_default()
            .entry(to)
            .or_default();
        *observations = observations.saturating_add(1);
        if *observations >= rules.coffee_threshold.get() {
            let ready_tick = self.tick().saturating_add(rules.initiative_ticks.get());
            self.state
                .initiatives
                .entry(from)
                .or_default()
                .insert(to, Slide { person, ready_tick });
        }
    }

    fn slide_anchors(&self, office: PlaceId, kiosk: PlaceId) -> Option<(PlaceId, PlaceId, u64)> {
        let places = self.visible_places();
        let upper = places.get(&office)?.floor;
        let lower = places.get(&kiosk)?.floor;
        if upper <= lower {
            return None;
        }
        let routes = crate::routing::walking_edges(&self.scenario, |place| self.access(place));
        let landings = || {
            places
                .values()
                .filter(|place| place.kind == PlaceKind::Landing && self.access(place.id))
        };
        let mut best = None;
        for from in landings().filter(|place| place.floor == upper) {
            let Some(approach) = walk(&routes, office, from.id, 0) else {
                continue;
            };
            for to in landings().filter(|place| place.floor == lower && place.column == from.column)
            {
                let Some(exit) = walk(&routes, to.id, kiosk, 0) else {
                    continue;
                };
                let length = approach.iter().chain(&exit).try_fold(1_u64, |cost, leg| {
                    let crate::routing::RouteLeg::Walk { duration, .. } = *leg else {
                        return None;
                    };
                    cost.checked_add(duration)
                });
                if let Some(length) = length {
                    let candidate = (length, from.id, to.id);
                    if best.is_none_or(|old| candidate < old) {
                        best = Some(candidate);
                    }
                }
            }
        }
        best.map(|(cost, from, to)| (from, to, cost))
    }

    pub(crate) fn ease_frustration(&mut self) {
        if self.tick().is_multiple_of(30) {
            for person in self.state.people.values_mut() {
                person.frustration = person.frustration.saturating_sub(1);
            }
        }
    }
}
