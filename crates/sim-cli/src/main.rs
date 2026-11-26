use sim_core::{Replay, Scenario};
use std::{env, error::Error, ffi::OsStr, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [mode, scenario_path, replay_path, flag, output_path] = args.as_slice() else {
        return Err("Aufruf: sim-cli replay HAUS ABLAUF --out DATEI".into());
    };
    if mode != "replay" || flag != "--out" {
        return Err("Aufruf: sim-cli replay HAUS ABLAUF --out DATEI".into());
    }

    let scenario = Scenario::from_json(&read(scenario_path)?)
        .map_err(|error| format!("{}: {error}", Path::new(scenario_path).display()))?;
    let replay = Replay::from_json(&read(replay_path)?)
        .map_err(|error| format!("{}: {error}", Path::new(replay_path).display()))?;
    let report = replay
        .run(scenario)
        .map_err(|error| format!("{}: {error}", Path::new(replay_path).display()))?;
    let mut json = serde_json::to_vec_pretty(&report)?;
    json.push(b'\n');
    fs::write(output_path, json)
        .map_err(|error| format!("{}: {error}", Path::new(output_path).display()))?;
    Ok(())
}

fn read(path: &OsStr) -> Result<Vec<u8>, Box<dyn Error>> {
    fs::read(path).map_err(|error| format!("{}: {error}", Path::new(path).display()).into())
}
