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
