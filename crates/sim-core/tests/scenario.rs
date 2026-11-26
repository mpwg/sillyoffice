use serde_json::{Value, json};
use sim_core::{DataError, FloorId, LiftStopId, PersonId, PlaceId, PlaceKind, Scenario, SectionId};

const TWO_FLOORS: &[u8] = include_bytes!("../../../godot/content/zwei-etagen.json");

fn changed_scenario(change: impl FnOnce(&mut Value)) -> Result<Scenario, DataError> {
    let mut document = serde_json::from_slice(TWO_FLOORS).unwrap();
    change(&mut document);
    Scenario::from_json(&serde_json::to_vec(&document).unwrap())
}

#[test]
fn loads_two_floors() {
    let scenario = Scenario::from_json(TWO_FLOORS).unwrap();
    assert_eq!(scenario.geometry().floors.len(), 2);
    assert_eq!(scenario.geometry().places[0].kind, PlaceKind::Reception);
    assert_eq!(scenario.geometry().places[1].kind, PlaceKind::Office);
    assert_eq!(scenario.geometry().stairs.len(), 1);
    assert_eq!(scenario.geometry().sections[2].id, SectionId(3));
    assert!(!scenario.initial().rented_sections.contains(&SectionId(3)));
    assert_eq!(scenario.initial().people.len(), 1);
    let person = &scenario.initial().people[0];
    assert_eq!(person.id, PersonId(1));
    assert_eq!(person.location, PlaceId(1));
    assert_eq!(person.home, PlaceId(5));
    assert_eq!(person.office, PlaceId(2));
}

#[test]
fn rejects_duplicate_ids_with_the_entry() {
    let result = changed_scenario(|document| {
        document["geometry"]["sections"][2]["id"] = json!(2);
    });
    assert!(
        matches!(
            result,
            Err(DataError::DuplicateId {
                entry: "Abschnitt",
                id: 2
            })
        ),
        "{result:?}"
    );
}

#[test]
fn rejects_a_missing_reference_with_its_source_and_target() {
    let result = changed_scenario(|document| {
        document["initial"]["people"][0]["office"] = json!(99);
    });
    assert!(
        matches!(
            result,
            Err(DataError::UnknownReference {
                entry: "Person",
                id: 1,
                field: "office",
                target: 99,
            })
        ),
        "{result:?}"
    );
}

#[test]
fn rejects_a_stop_on_the_exclusive_floor_boundary() {
    let result = changed_scenario(|document| {
        document["geometry"]["lift_stops"][1]["column"] = json!(12);
    });
    assert!(
        matches!(
            result,
            Err(DataError::StopOutsideFloor {
                stop: LiftStopId(2),
                floor: FloorId(1),
                column: 12,
                width: 12
            })
        ),
        "{result:?}"
    );
}

#[test]
fn rejects_an_empty_floor() {
    let result = changed_scenario(|document| {
        document["geometry"]["floors"][0]["width"] = json!(0);
    });
    assert!(
        matches!(
            result,
            Err(DataError::InvalidFloorWidth { floor: FloorId(0) })
        ),
        "{result:?}"
    );
}

#[test]
fn rejects_a_section_beyond_its_floor() {
    let result = changed_scenario(|document| {
        document["geometry"]["sections"][2]["end"] = json!(13);
    });
    assert!(
        matches!(
            result,
            Err(DataError::SectionOutsideFloor {
                section: SectionId(3),
                floor: FloorId(1),
                ..
            })
        ),
        "{result:?}"
    );
}

#[test]
fn rejects_a_place_beyond_its_section() {
    let result = changed_scenario(|document| {
        document["geometry"]["places"][1]["column"] = json!(8);
    });
    assert!(
        matches!(
            result,
            Err(DataError::PlaceOutsideSection {
                place: PlaceId(2),
                section: SectionId(2),
                column: 8
            })
        ),
        "{result:?}"
    );
}

#[test]
fn rejects_an_unfunded_start() {
    let result = changed_scenario(|document| {
        document["initial"]["balance"] = json!(-1);
    });
    assert!(
        matches!(
            result,
            Err(DataError::NegativeInitialBalance { balance: -1 })
        ),
        "{result:?}"
    );
}

#[test]
fn rejects_an_unknown_version_before_reading_its_schema() {
    let result = Scenario::from_json(br#"{"version":7,"future_geometry":{}}"#);
    assert!(
        matches!(result, Err(DataError::UnsupportedVersion { version: 7 })),
        "{result:?}"
    );
}

#[test]
fn serializes_the_same_scenario_when_entries_are_reordered() {
    let reordered = changed_scenario(|document| {
        document["geometry"]["floors"]
            .as_array_mut()
            .unwrap()
            .reverse();
        document["geometry"]["places"]
            .as_array_mut()
            .unwrap()
            .reverse();
        document["initial"]["rented_sections"]
            .as_array_mut()
            .unwrap()
            .reverse();
    })
    .unwrap();
    let reference = Scenario::from_json(TWO_FLOORS).unwrap();
    assert_eq!(
        serde_json::to_vec(&reordered).unwrap(),
        serde_json::to_vec(&reference).unwrap()
    );
}
