use crate::{Checkpoint, DataError, Scenario, ScheduledCommand, SubmitError, World};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayFile {
    version: u32,
    seed: u64,
    checkpoints: Vec<u64>,
    commands: Vec<ScheduledCommand>,
}

pub struct Replay {
    seed: u64,
    checkpoints: Vec<u64>,
    commands: Vec<ScheduledCommand>,
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("ungültiges Ablauf-JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unbekannte Ablaufversion {0}")]
    Version(u32),
    #[error("Kontrollticks müssen vorhanden sein und streng aufsteigen")]
    Checkpoints,
    #[error(transparent)]
    Scenario(#[from] DataError),
    #[error(transparent)]
    Command(#[from] SubmitError),
}

#[derive(Debug, Serialize)]
pub struct ReplayReport {
    pub checkpoints: Vec<Checkpoint>,
}

impl Replay {
    pub fn from_json(bytes: &[u8]) -> Result<Self, ReplayError> {
        let ReplayFile {
            version,
            seed,
            checkpoints,
            commands,
        } = serde_json::from_slice(bytes)?;
        if version != 1 {
            return Err(ReplayError::Version(version));
        }
        if checkpoints.is_empty() || !checkpoints.is_sorted_by(|a, b| a < b) {
            return Err(ReplayError::Checkpoints);
        }
        Ok(Self {
            seed,
            checkpoints,
            commands,
        })
    }

    pub fn start(&self, scenario: Scenario) -> Result<World, ReplayError> {
        let mut world = World::new(scenario, self.seed)?;
        for command in &self.commands {
            world.submit(command.clone())?;
        }
        Ok(world)
    }

    pub fn run(&self, scenario: Scenario) -> Result<ReplayReport, ReplayError> {
        let mut world = self.start(scenario)?;
        let checkpoints = self
            .checkpoints
            .iter()
            .map(|&tick| {
                while world.tick() < tick {
                    world.step();
                }
                world.checkpoint()
            })
            .collect();
        Ok(ReplayReport { checkpoints })
    }
}
