use godot::prelude::*;
use serde_json::json;
use sim_core::{Action, FloorId, PersonId, Replay, ReplayError, Scenario, ViewRect, World};

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
    fn start_prototype(&mut self, scenario_json: GString, seed: i64) -> GString {
        let result = Scenario::from_json(scenario_json.to_string().as_bytes())
            .and_then(|scenario| World::new(scenario, seed as u64));
        match result {
            Ok(world) => {
                self.world = Some(world);
                GString::new()
            }
            Err(error) => message(error),
        }
    }

    #[func]
    fn preview_json(&self, action_json: GString) -> GString {
        let Some(world) = &self.world else {
            return to_json(&json!({"ok": false, "error": "Es ist keine Partie geladen."}));
        };
        let action = match serde_json::from_str::<Action>(&action_json.to_string()) {
            Ok(action) => action,
            Err(error) => return to_json(&json!({"ok": false, "error": error.to_string()})),
        };
        match world.preview(&action) {
            Ok(impact) => to_json(&json!({"ok": true, "impact": impact})),
            Err(error) => to_json(&json!({
                "ok": false,
                "error": error.to_string(),
                "first_blocked_stop": world.first_blocked_lift_stop(&action),
            })),
        }
    }

    #[func]
    fn submit_action(&mut self, action_json: GString) -> GString {
        let Some(world) = &mut self.world else {
            return GString::from("Es ist keine Partie geladen.");
        };
        let action = match serde_json::from_str(&action_json.to_string()) {
            Ok(action) => action,
            Err(error) => return message(error),
        };
        world
            .submit_next(action)
            .map_or_else(message, |()| GString::new())
    }

    #[func]
    fn results_json(&self) -> GString {
        self.world
            .as_ref()
            .map_or_else(GString::new, |world| to_json(world.tick_report()))
    }

    #[func]
    fn inspect_json(&self, person_id: i64) -> GString {
        let info = u32::try_from(person_id)
            .ok()
            .and_then(|id| self.world.as_ref()?.explain(PersonId(id)));
        to_json(&info)
    }

    #[func]
    fn save_json(&self) -> GString {
        let Some(world) = &self.world else {
            return GString::new();
        };
        match world.save().map(String::from_utf8) {
            Ok(Ok(content)) => GString::from(&content),
            Ok(Err(error)) => {
                godot_error!("Der Spielstand ist kein UTF-8: {error}");
                GString::new()
            }
            Err(error) => {
                godot_error!("{error}");
                GString::new()
            }
        }
    }

    #[func]
    fn validate_save(&self, scenario_json: GString, save_json: GString) -> bool {
        restored_world(scenario_json, save_json).is_ok()
    }

    #[func]
    fn restore_json(&mut self, scenario_json: GString, save_json: GString) -> GString {
        match restored_world(scenario_json, save_json) {
            Ok(world) => {
                self.world = Some(world);
                GString::new()
            }
            Err(error) => message(error),
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

fn restored_world(scenario: GString, state: GString) -> Result<World, String> {
    let scenario =
        Scenario::from_json(scenario.to_string().as_bytes()).map_err(|error| error.to_string())?;
    World::restore(scenario, state.to_string().as_bytes()).map_err(|error| error.to_string())
}

fn message(error: impl std::fmt::Display) -> GString {
    GString::from(&error.to_string())
}
