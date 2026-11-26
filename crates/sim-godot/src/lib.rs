use godot::prelude::*;
use sim_core::{FloorId, Replay, ReplayError, Scenario, ViewRect, World};

struct SimExtension;

// SAFETY: Godot calls this entry point to register the classes defined in this library.
#[gdextension]
unsafe impl ExtensionLibrary for SimExtension {}

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct SimSession {
    world: Option<World>,
}

#[godot_api]
impl SimSession {
    #[func]
    fn load_partie(&mut self, scenario_json: GString, replay_json: GString) -> GString {
        match start(&scenario_json.to_string(), &replay_json.to_string()) {
            Ok(world) => {
                self.world = Some(world);
                GString::new()
            }
            Err(error) => GString::from(&error.to_string()),
        }
    }

    #[func]
    fn step(&mut self) -> bool {
        let Some(world) = &mut self.world else {
            return false;
        };
        world.step();
        true
    }

    #[func]
    fn checkpoint_json(&self) -> GString {
        self.world
            .as_ref()
            .map_or_else(GString::new, |world| to_json(&world.checkpoint()))
    }

    #[func]
    fn view_json(&self, first_floor: i32, last_floor: i32) -> GString {
        let (Some(world), Ok(first_floor), Ok(last_floor)) = (
            &self.world,
            u32::try_from(first_floor),
            u32::try_from(last_floor),
        ) else {
            return GString::new();
        };
        to_json(&world.view(ViewRect {
            first_floor: FloorId(first_floor),
            last_floor: FloorId(last_floor),
        }))
    }
}

fn start(scenario: &str, replay: &str) -> Result<World, ReplayError> {
    let scenario = Scenario::from_json(scenario.as_bytes())?;
    Replay::from_json(replay.as_bytes())?.start(scenario)
}

fn to_json(value: &impl serde::Serialize) -> GString {
    match serde_json::to_string(value) {
        Ok(json) => GString::from(&json),
        Err(error) => {
            godot_error!("Simulationsdaten konnten nicht serialisiert werden: {error}");
            GString::new()
        }
    }
}
