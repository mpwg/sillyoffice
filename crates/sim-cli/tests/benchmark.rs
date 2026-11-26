use serde_json::{Value, json};
use sim_core::{FloorId, Scenario};
use std::{fs, path::Path, process::Command};

#[test]
fn rush_hour_has_a_real_transfer_and_is_repeatable() {
    let output = tempfile::tempdir().unwrap();
    let config_path = output.path().join("kurzer-berufsverkehr.json");
    let mut config: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/berufsverkehr.json"))
            .unwrap();
    // Keep the workload and geometry. Move the first wave forward for this functional test.
    config["wave_tick"] = json!(25);
    config["quiet_from_tick"] = json!(250);
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();

    let run = |name: &str| {
        let report = output.path().join(format!("{name}-bericht.json"));
        let scenario = output.path().join(format!("{name}-haus.json"));
        let process = Command::new(env!("CARGO_BIN_EXE_sim-cli"))
            .arg("bench")
            .arg(&config_path)
            .args(["--ticks", "320", "--out"])
            .arg(&report)
            .arg("--scenario-out")
            .arg(&scenario)
            .output()
            .unwrap();
        assert!(
            process.status.success(),
            "{}",
            String::from_utf8_lossy(&process.stderr)
        );
        (read_json(&report), fs::read(scenario).unwrap())
    };
    let (first, house) = run("eins");
    let (second, repeated_house) = run("zwei");
    assert_eq!(house, repeated_house);
    let scenario = Scenario::from_json(&house).unwrap();
    assert_eq!(scenario.geometry().floors.len(), 100);
    assert_eq!(scenario.initial().people.len(), 5000);
    let express_floors: Vec<FloorId> = scenario
        .geometry()
        .lift_stops
        .iter()
        .filter(|stop| stop.column == 6)
        .map(|stop| stop.floor)
        .collect();
    assert_eq!(
        express_floors,
        (0..100).step_by(10).map(FloorId).collect::<Vec<_>>()
    );

    let deterministic = &first["deterministic"];
    assert_eq!(deterministic, &second["deterministic"]);
    assert_eq!(deterministic["ticks"], 320);
    assert_eq!(deterministic["checksum"].as_str().unwrap().len(), 64);
    assert!(deterministic["route_searches"].as_u64().unwrap() > 0);
    assert!(deterministic["lift_boardings"].as_u64().unwrap() > 0);
    assert!(deterministic["lift_transfers"].as_u64().unwrap() > 0);
    assert!(deterministic["longest_queue"].as_u64().unwrap() > 0);
    let transfer = &deterministic["cohort_transfer"];
    assert!((1..=5).contains(&transfer["person"].as_u64().unwrap()));
    assert!((25..=320).contains(&transfer["tick"].as_u64().unwrap()));
    let mut lines = [
        transfer["from_kind"].as_str().unwrap(),
        transfer["to_kind"].as_str().unwrap(),
    ];
    lines.sort_unstable();
    assert_eq!(lines, ["express", "feeder"]);
    let phases = first["phases"].as_array().unwrap();
    assert_eq!(
        phases
            .iter()
            .map(|phase| phase["ticks"].as_u64().unwrap())
            .sum::<u64>(),
        320
    );
    assert_eq!(
        phases
            .iter()
            .map(|phase| phase["route_searches"].as_u64().unwrap())
            .sum::<u64>(),
        deterministic["route_searches"].as_u64().unwrap()
    );
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
