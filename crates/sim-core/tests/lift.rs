use serde_json::{Value, json};
use sim_core::{Action, CommandNumber, Outcome, Scenario, ScheduledCommand, World};

fn world() -> World {
    let bytes = include_bytes!("../../../godot/content/erstes-haus.json");
    World::new(Scenario::from_json(bytes).unwrap(), 4).unwrap()
}

fn action(value: Value) -> Action {
    serde_json::from_value(value).expect("supported building action")
}

fn submit(world: &mut World, number: u64, action: Action) {
    world
        .submit(ScheduledCommand {
            tick: world.tick() + 1,
            number: CommandNumber(number),
            action,
        })
        .unwrap();
}

#[test]
fn lift_requires_development_and_contiguous_halts() {
    let mut world = world();
    let build = action(json!({"kind":"build_lift","stops":[1,2,3]}));
    assert!(world.preview(&build).is_err());
    submit(
        &mut world,
        1,
        action(json!({"kind":"designate","place":6,"use":"home"})),
    );
    submit(
        &mut world,
        2,
        action(json!({"kind":"designate","place":7,"use":"office"})),
    );
    world.step();
    submit(&mut world, 3, action(json!({"kind":"unlock_lift"})));
    world.step();
    let gaps = action(json!({"kind":"build_lift","stops":[1,3]}));
    assert!(world.preview(&gaps).is_err());
    assert_eq!(world.preview(&build).unwrap().immediate_cost, 35);
}

#[test]
fn competing_lift_builds_charge_once() {
    let mut world = world();
    submit(
        &mut world,
        1,
        action(json!({"kind":"designate","place":6,"use":"home"})),
    );
    submit(
        &mut world,
        2,
        action(json!({"kind":"designate","place":7,"use":"office"})),
    );
    world.step();
    submit(&mut world, 3, action(json!({"kind":"unlock_lift"})));
    world.step();
    let build = action(json!({"kind":"build_lift","stops":[1,2,3]}));
    submit(&mut world, 5, build.clone());
    submit(&mut world, 4, build);
    let tick = world.step();
    assert_eq!(tick.results[0].outcome, Outcome::Applied);
    assert!(matches!(tick.results[1].outcome, Outcome::Rejected { .. }));
    let snapshot = serde_json::to_value(world.checkpoint()).unwrap();
    assert_eq!(snapshot["state"]["balance"], 150 - 8 - 35);
}
