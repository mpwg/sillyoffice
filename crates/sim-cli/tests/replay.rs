use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

#[test]
fn first_walk_is_repeatable() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../godot/content");
    let output = tempfile::tempdir().unwrap();
    let run = |name: &str| {
        let report_path = output.path().join(name);
        let process = Command::new(env!("CARGO_BIN_EXE_sim-cli"))
            .arg("replay")
            .arg(content.join("zwei-etagen.json"))
            .arg(content.join("erster-weg.json"))
            .arg("--out")
            .arg(&report_path)
            .output()
            .unwrap();
        assert!(
            process.status.success(),
            "{}",
            String::from_utf8_lossy(&process.stderr)
        );
        let bytes = fs::read(&report_path).expect("replay must write a checkpoint report");
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        (bytes, report)
    };

    let (first, report) = run("first.json");
    let (second, _) = run("second.json");
    assert_eq!(first, second);
    let checkpoints = report["checkpoints"].as_array().unwrap();
    assert_eq!(checkpoints.len(), 3);
    let [before, rent, arrived] = checkpoints.as_slice() else {
        unreachable!()
    };
    assert_eq!(before["tick"], 4);
    assert_eq!(before["state"]["rented_sections"], json!([1, 2]));
    assert_eq!(before["state"]["balance"], 100);
    assert_eq!(before["results"], json!([]));
    assert_eq!(rent["tick"], 5);
    assert_eq!(rent["state"]["rented_sections"], json!([1, 2, 3]));
    assert_eq!(rent["state"]["balance"], 75);
    assert_eq!(
        rent["results"],
        json!([
            {"number": 1, "outcome": {"kind": "applied"}},
            {"number": 2, "outcome": {"kind": "rejected", "reason": "already_rented"}}
        ])
    );
    assert_eq!(arrived["tick"], 12);
    assert_eq!(
        arrived["state"]["people"]["1"]["location"],
        json!({"kind": "at", "place": 2})
    );
    for checkpoint in checkpoints {
        assert_eq!(checkpoint["checksum"].as_str().unwrap().len(), 64);
    }
    assert_ne!(before["checksum"], rent["checksum"]);
    assert_ne!(rent["checksum"], arrived["checksum"]);
}

#[test]
fn prototype_day_covers_tenancy_lift_initiative_and_confirmed_return() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../godot/content");
    let output = tempfile::tempdir().unwrap();
    let report_path = output.path().join("day.json");
    let process = Command::new(env!("CARGO_BIN_EXE_sim-cli"))
        .arg("replay")
        .arg(content.join("erstes-haus.json"))
        .arg(content.join("spieltag.json"))
        .arg("--out")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        process.status.success(),
        "{}",
        String::from_utf8_lossy(&process.stderr)
    );
    let report: Value = serde_json::from_slice(&fs::read(report_path).unwrap()).unwrap();
    let points = report["checkpoints"].as_array().unwrap();
    for point in points {
        for result in point["results"].as_array().unwrap() {
            assert_eq!(
                result["outcome"]["kind"], "applied",
                "{result} at tick {}",
                point["tick"]
            );
        }
    }
    let point = |tick: u64| points.iter().find(|point| point["tick"] == tick).unwrap();
    assert_eq!(point(23)["state"]["people"]["1"]["location"]["place"], 7);
    assert_eq!(
        point(35)["state"]["people"]["1"]["location"]["kind"],
        "riding_lift"
    );
    assert!(point(50)["state"]["lift_boardings"].as_u64().unwrap() > 0);
    assert!(
        point(89)["state"]["initiatives"]["10"]["3"]["ready_tick"]
            .as_u64()
            .unwrap()
            <= 89
    );
    assert_eq!(point(100)["state"]["people"]["1"]["office"], 11);
    assert_eq!(point(100)["state"]["rented_sections"], json!([1, 2, 4]));
}
