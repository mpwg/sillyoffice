use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sim_core::{CommandNumber, Replay, SaveError, Scenario, ScheduledCommand, SubmitError, World};

const HOUSE: &[u8] = include_bytes!("../../../godot/content/zwei-etagen.json");
const WALK: &[u8] = include_bytes!("../../../godot/content/erster-weg.json");

fn scenario() -> Scenario {
    Scenario::from_json(HOUSE).unwrap()
}

fn walking_world() -> World {
    let mut world = Replay::from_json(WALK).unwrap().start(scenario()).unwrap();
    for _ in 0..4 {
        world.step();
    }
    world
}

#[test]
fn save_mid_walk() {
    let mut original = walking_world();
    let checkpoint = serde_json::to_value(original.checkpoint()).unwrap();
    assert_eq!(
        checkpoint["state"]["people"]["1"]["location"]["kind"],
        "walking"
    );
    assert_eq!(checkpoint["state"]["pending"]["5"]["1"]["section"], 3);
    let saved = original.save().unwrap();
    let mut restored = World::restore(scenario(), &saved).unwrap();
    assert_eq!(
        serde_json::to_value(restored.checkpoint()).unwrap(),
        checkpoint
    );

    let duplicate: ScheduledCommand = serde_json::from_value(json!({
        "tick": 6, "number": 2, "action": {"kind": "rent_section", "section": 3}
    }))
    .unwrap();
    assert_eq!(
        restored.submit(duplicate),
        Err(SubmitError::DuplicateNumber(CommandNumber(2)))
    );
    for tick in 5..=12 {
        assert_eq!(restored.step(), original.step(), "Tick {tick}");
        assert_eq!(
            serde_json::to_value(restored.checkpoint()).unwrap(),
            serde_json::to_value(original.checkpoint()).unwrap(),
            "Tick {tick}"
        );
    }
    let destination = serde_json::to_value(restored.checkpoint()).unwrap();
    assert_eq!(
        destination["state"]["people"]["1"]["location"],
        json!({"kind": "at", "place": 2})
    );
}

#[test]
fn reports_why_a_save_cannot_be_loaded() {
    let saved = walking_world().save().unwrap();
    let failure = |bytes: &[u8]| World::restore(scenario(), bytes).err().unwrap();

    assert!(matches!(failure(b"{"), SaveError::InvalidJson(_)));
    assert!(matches!(
        failure(br#"{"version":99,"unknown_schema":{}}"#),
        SaveError::UnsupportedVersion(99)
    ));

    let mut document: Value = serde_json::from_slice(&saved).unwrap();
    document["rule_version"] = json!(99);
    assert!(matches!(
        failure(&serde_json::to_vec(&document).unwrap()),
        SaveError::UnsupportedRuleVersion(99)
    ));

    let mut other_house: Value = serde_json::from_slice(HOUSE).unwrap();
    other_house["initial"]["balance"] = json!(200);
    let other_scenario = Scenario::from_json(&serde_json::to_vec(&other_house).unwrap()).unwrap();
    assert!(matches!(
        World::restore(other_scenario, &saved),
        Err(SaveError::ScenarioMismatch)
    ));

    let mut document: Value = serde_json::from_slice(&saved).unwrap();
    document["payload"]["state"]["random_state"] = json!(81);
    assert!(matches!(
        failure(&serde_json::to_vec(&document).unwrap()),
        SaveError::ChecksumMismatch
    ));
}

#[test]
fn rejects_inconsistent_state_even_with_a_matching_checksum() {
    let saved = walking_world().save().unwrap();
    let cases = [
        ("/state/people/1/home", json!(99), "Wohnung"),
        ("/state/people/1/location/to", json!(99), "Gehziel"),
        ("/state/people/1/location/end_tick", json!(4), "Gehsegment"),
        ("/state/households/1", json!(4), "Haushalt"),
        ("/last_report/tick", json!(3), "Tickbericht"),
        (
            "/state/pending",
            json!({"4":{"1":{"kind":"rent_section","section":3}}}),
            "Ziel-Tick",
        ),
        ("/state/submitted_numbers", json!([2]), "Befehl 1"),
    ];
    for (pointer, replacement, expected_cause) in cases {
        let mut document: Value = serde_json::from_slice(&saved).unwrap();
        *document["payload"].pointer_mut(pointer).unwrap() = replacement;
        let encoded_payload = serde_json::to_vec(&document["payload"]).unwrap();
        let checksum: String = Sha256::digest(encoded_payload)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        document["checksum"] = json!(checksum);
        let failure = World::restore(scenario(), &serde_json::to_vec(&document).unwrap())
            .err()
            .unwrap();
        assert!(
            matches!(failure, SaveError::InvalidState(ref detail) if detail.contains(expected_cause)),
            "{pointer}: {failure}"
        );
    }
}
