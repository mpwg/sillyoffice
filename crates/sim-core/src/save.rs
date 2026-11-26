//! Versioned snapshots for rebuilding an independent simulation instance.

use crate::world::checksum;
use crate::{CoreState, Designation, Location, PersonId, PlaceKind, Scenario, TickReport, World};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use thiserror::Error;

const FORMAT_VERSION: u32 = 1;
const RULE_VERSION: u32 = 1;

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SaveFile {
    version: u32,
    rule_version: u32,
    checksum: String,
    payload: Value,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Snapshot<State, Report> {
    state: State,
    last_report: Report,
}

/// Describes why a simulation snapshot could not be written or restored.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SaveError {
    #[error("Der Spielstand kann nicht serialisiert werden: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("Ungültiges Spielstand-JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("Unbekannte Spielstandformatversion {0}")]
    UnsupportedVersion(u32),
    #[error("Unbekannte Spielstandregelversion {0}")]
    UnsupportedRuleVersion(u32),
    #[error("Der Spielstand gehört zu einem anderen Szenario.")]
    ScenarioMismatch,
    #[error("Die Prüfsumme des Spielstands stimmt nicht.")]
    ChecksumMismatch,
    #[error("Widersprüchlicher Spielstand: {0}")]
    InvalidState(String),
}

impl World {
    /// Encodes the completed tick and the state needed to continue the simulation.
    ///
    /// # Errors
    ///
    /// Returns an error if the game state cannot be represented as JSON.
    pub fn save(&self) -> Result<Vec<u8>, SaveError> {
        let payload = serde_json::to_value(Snapshot {
            state: &self.state,
            last_report: &self.last_report,
        })
        .map_err(SaveError::Encode)?;
        let document = SaveFile {
            version: FORMAT_VERSION,
            rule_version: RULE_VERSION,
            checksum: checksum(&payload),
            payload,
        };
        serde_json::to_vec(&document).map_err(SaveError::Encode)
    }

    /// Restores a complete simulation from saved bytes and its original scenario.
    ///
    /// # Errors
    ///
    /// Reports incompatible versions, a different scenario, or invalid save data.
    pub fn restore(scenario: Scenario, bytes: &[u8]) -> Result<Self, SaveError> {
        let VersionHeader { version } =
            serde_json::from_slice(bytes).map_err(SaveError::InvalidJson)?;
        if version != FORMAT_VERSION {
            return Err(SaveError::UnsupportedVersion(version));
        }
        let document: SaveFile = serde_json::from_slice(bytes).map_err(SaveError::InvalidJson)?;
        if document.rule_version != RULE_VERSION {
            return Err(SaveError::UnsupportedRuleVersion(document.rule_version));
        }
        if document.checksum != checksum(&document.payload) {
            return Err(SaveError::ChecksumMismatch);
        }
        let Snapshot { state, last_report }: Snapshot<CoreState, TickReport> =
            serde_json::from_value(document.payload).map_err(SaveError::InvalidJson)?;
        if state.scenario_checksum != checksum(&scenario) {
            return Err(SaveError::ScenarioMismatch);
        }
        let world = Self {
            scenario,
            state,
            last_report,
        };
        world.validate_saved_state()?;
        Ok(world)
    }

    fn validate_saved_state(&self) -> Result<(), SaveError> {
        self.validate_saved_commands()?;
        self.validate_saved_geometry()?;
        self.validate_saved_households()?;
        self.validate_saved_transport()?;
        self.validate_saved_initiatives()?;
        Ok(())
    }

    fn validate_saved_commands(&self) -> Result<(), SaveError> {
        let state = &self.state;
        let mut pending = BTreeSet::new();
        for (&tick, commands) in &state.pending {
            if tick <= state.tick || commands.is_empty() {
                return Err(SaveError::InvalidState(format!(
                    "Ungültiger Ziel-Tick {tick} in der Befehlswarteschlange."
                )));
            }
            // An unknown action target is an ordinary rule rejection at execution time.
            for &number in commands.keys() {
                if !state.submitted_numbers.contains(&number) || !pending.insert(number) {
                    return Err(SaveError::InvalidState(format!(
                        "Befehl {} ist nicht eindeutig als eingereiht erfasst.",
                        number.0
                    )));
                }
            }
        }
        let report = &self.last_report;
        if report.tick != state.tick
            || (state.tick == 0 && !report.results.is_empty())
            || !report.results.is_sorted_by(|a, b| a.number < b.number)
        {
            return Err(SaveError::InvalidState(format!(
                "Der Tickbericht passt nicht zu Tick {}.",
                state.tick
            )));
        }
        for result in &report.results {
            let number = result.number;
            if !state.submitted_numbers.contains(&number) || pending.contains(&number) {
                return Err(SaveError::InvalidState(format!(
                    "Befehl {} im Tickbericht ist nicht als ausgeführt erfasst.",
                    number.0
                )));
            }
        }
        Ok(())
    }

    fn validate_saved_geometry(&self) -> Result<(), SaveError> {
        let geometry = self.scenario.geometry();
        for section in &self.state.rented_sections {
            if geometry
                .sections
                .binary_search_by_key(section, |entry| entry.id)
                .is_err()
            {
                return Err(SaveError::InvalidState(format!(
                    "Unbekannter gemieteter Abschnitt {}.",
                    section.0
                )));
            }
        }
        for (&id, &designation) in &self.state.designations {
            let valid = match self.place(id).map(|place| place.kind) {
                Some(PlaceKind::Space) => true,
                Some(PlaceKind::Home) => designation == Designation::Home,
                Some(PlaceKind::Office) => designation == Designation::Office,
                _ => false,
            };
            if !valid {
                return Err(SaveError::InvalidState(format!(
                    "Ungültige Widmung von Ort {}.",
                    id.0
                )));
            }
        }
        for place in &geometry.places {
            if matches!(
                place.kind,
                PlaceKind::Space | PlaceKind::Home | PlaceKind::Office
            ) && !self.state.designations.contains_key(&place.id)
            {
                return Err(SaveError::InvalidState(format!(
                    "Widmung für Ort {} fehlt.",
                    place.id.0
                )));
            }
        }
        Ok(())
    }

    fn validate_saved_households(&self) -> Result<(), SaveError> {
        let mut households = BTreeSet::new();
        let next_person_id = self.state.next_person_id;
        if !(1..=u64::from(u32::MAX) + 1).contains(&next_person_id) {
            return Err(SaveError::InvalidState(
                "Ungültige nächste Personen-ID.".into(),
            ));
        }
        for (&id, person) in &self.state.people {
            if u64::from(id.0) >= next_person_id {
                return Err(SaveError::InvalidState(format!(
                    "Die nächste Personen-ID würde die ID {} wiederverwenden.",
                    id.0
                )));
            }
            for (field, place) in [
                ("Wohnung", Some(person.home)),
                ("Arbeitsplatz", person.office),
            ] {
                if let Some(place) = place
                    && self.place(place).is_none()
                {
                    return Err(SaveError::InvalidState(format!(
                        "Person {} verweist bei {field} auf den unbekannten Ort {}.",
                        id.0, place.0
                    )));
                }
            }
            if self.state.households.get(&person.household) != Some(&person.home) {
                return Err(SaveError::InvalidState(format!(
                    "Person {} hat keine passende Wohnung im Haushalt {}.",
                    id.0, person.household.0
                )));
            }
            households.insert(person.household);
            self.validate_person_location(id, person.location)?;
        }
        for household in self.state.households.keys() {
            if !households.contains(household)
                || self.state.waiting_households.contains_key(household)
            {
                return Err(SaveError::InvalidState(format!(
                    "Haushalt {} ist ohne Bewohner oder zugleich als wartend erfasst.",
                    household.0
                )));
            }
        }
        Ok(())
    }

    fn validate_saved_transport(&self) -> Result<(), SaveError> {
        use crate::routing::RouteLeg;

        let state = &self.state;
        let lines: std::collections::BTreeMap<_, _> = state.lifts.lines().collect();
        let mut registered = BTreeSet::new();
        let invalid = |id: u32| {
            SaveError::InvalidState(format!("Ungültige Liftregistrierung für Person {id}."))
        };
        if !(1..=u64::from(u32::MAX) + 1).contains(&state.next_lift_id)
            || state.express_lifts.iter().any(|id| !lines.contains_key(id))
        {
            return Err(SaveError::InvalidState("Ungültige Lift-IDs.".into()));
        }
        for (&lift, line) in &lines {
            if u64::from(lift.0) >= state.next_lift_id
                || line.stops().iter().any(|&id| self.stop_place(id).is_none())
            {
                return Err(SaveError::InvalidState(format!(
                    "Ungültige Halte oder ID für Lift {}.",
                    lift.0
                )));
            }
            for (person, from, to, tick) in line.waiting_people() {
                if !registered.insert(person)
                    || tick > self.tick()
                    || !matches!(self.person_location(person), Some(Location::WaitingLift { place, lift: id, from: at, to: dest, arrival_tick }) if id == lift && at == from && dest == to && tick == arrival_tick && Some(place) == self.stop_place(from))
                {
                    return Err(invalid(person.0));
                }
            }
            for person in line.riding_people() {
                if !registered.insert(person)
                    || line.rider_destination(person).is_none()
                    || !matches!(self.person_location(person), Some(Location::RidingLift { place, lift: id, from, to }) if id == lift && self.place(place).is_some() && self.stop(from).is_some() && self.stop(to).is_some())
                {
                    return Err(invalid(person.0));
                }
            }
        }
        for (&id, person) in &state.people {
            if matches!(
                person.location,
                Location::WaitingLift { .. } | Location::RidingLift { .. }
            ) != registered.contains(&id)
            {
                return Err(invalid(id.0));
            }
            if person
                .destination
                .is_some_and(|target| !self.access(target))
                || person.remaining_route.iter().any(|leg| match *leg {
                    RouteLeg::Walk { from, to, duration } => {
                        duration == 0 || !self.access(from) || !self.access(to)
                    }
                    RouteLeg::Lift { lift, from, to } => {
                        from == to
                            || lines.get(&lift).is_none_or(|line| {
                                !line.stops().contains(&from) || !line.stops().contains(&to)
                            })
                    }
                })
            {
                return Err(SaveError::InvalidState(format!(
                    "Ungültiger Restweg für Person {}.",
                    id.0
                )));
            }
        }
        Ok(())
    }

    fn validate_saved_initiatives(&self) -> Result<(), SaveError> {
        let places = self.visible_places();
        let valid_pair = |from: crate::PlaceId, to: crate::PlaceId| {
            places
                .get(&from)
                .zip(places.get(&to))
                .is_some_and(|(upper, lower)| {
                    upper.kind == PlaceKind::Landing
                        && lower.kind == PlaceKind::Landing
                        && upper.floor > lower.floor
                        && upper.column == lower.column
                        && self.access(from)
                        && self.access(to)
                })
        };
        for (&from, paths) in &self.state.initiatives {
            if paths.is_empty()
                || paths.iter().any(|(&to, slide)| {
                    !valid_pair(from, to) || u64::from(slide.person.0) >= self.state.next_person_id
                })
            {
                return Err(SaveError::InvalidState(format!(
                    "Ungültige Bewohnerverbindung ab Ort {}.",
                    from.0
                )));
            }
        }
        for (&from, paths) in &self.state.coffee_observations {
            if paths.is_empty()
                || paths
                    .iter()
                    .any(|(&to, &count)| count == 0 || !valid_pair(from, to))
            {
                return Err(SaveError::InvalidState(format!(
                    "Ungültige Beobachtung ab Ort {}.",
                    from.0
                )));
            }
        }
        for (&id, trip) in &self.state.coffee_trips {
            use crate::initiative::CoffeeTrip;
            let valid = self
                .state
                .people
                .get(&id)
                .is_some_and(|person| match *trip {
                    CoffeeTrip::Outbound {
                        office,
                        kiosk,
                        left,
                    } => {
                        person.office == Some(office)
                            && person.destination == Some(kiosk)
                            && self.access(kiosk)
                            && left <= self.tick()
                    }
                    CoffeeTrip::Returning { office } => {
                        person.office == Some(office) && person.destination == Some(office)
                    }
                });
            if !valid {
                return Err(SaveError::InvalidState(format!(
                    "Ungültiger Kaffeegang für Person {}.",
                    id.0
                )));
            }
        }
        Ok(())
    }

    fn validate_person_location(&self, id: PersonId, location: Location) -> Result<(), SaveError> {
        if let Location::At { place } = location
            && self.place(place).is_none()
        {
            return Err(SaveError::InvalidState(format!(
                "Person {} befindet sich am unbekannten Ort {}.",
                id.0, place.0
            )));
        }
        if let Location::Walking(segment) = location {
            for (field, place) in [("Gehstart", segment.from), ("Gehziel", segment.to)] {
                if self.place(place).is_none() {
                    return Err(SaveError::InvalidState(format!(
                        "Person {} verweist bei {field} auf den unbekannten Ort {}.",
                        id.0, place.0
                    )));
                }
            }
            if !(segment.start_tick..segment.end_tick).contains(&self.state.tick) {
                return Err(SaveError::InvalidState(format!(
                    "Das aktive Gehsegment von Person {} passt nicht zu Tick {}.",
                    id.0, self.state.tick
                )));
            }
        }
        Ok(())
    }
}
