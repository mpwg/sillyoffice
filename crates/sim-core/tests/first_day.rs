use serde_json::{Value, json};
use sim_core::{CommandNumber, Scenario, ScheduledCommand, World};

fn prototype() -> World {
    let scenario = Scenario::from_json(include_bytes!("../../../godot/content/erstes-haus.json"))
        .expect("the prototype scenario loads");
    World::new(scenario, 42).unwrap()
}

fn schedule(world: &mut World, tick: u64, number: u64, action: Value) {
    world
        .submit(ScheduledCommand {
            tick,
            number: CommandNumber(number),
            action: serde_json::from_value(action).unwrap(),
        })
        .unwrap();
}

#[test]
fn a_household_moves_in_only_when_it_can_reach_a_free_job() {
    let mut world = prototype();
    schedule(
        &mut world,
        1,
        1,
        json!({"kind":"designate","place":6,"use":"home"}),
    );
    schedule(&mut world, 2, 2, json!({"kind":"rent_section","section":4}));
    schedule(
        &mut world,
        3,
        3,
        json!({"kind":"designate","place":8,"use":"office"}),
    );
    schedule(
        &mut world,
        4,
        4,
        json!({"kind":"designate","place":7,"use":"office"}),
    );
    for _ in 0..3 {
        world.step();
    }
    let empty = serde_json::to_value(world.checkpoint()).unwrap();
    assert_eq!(empty["state"]["people"], json!({}));

    for _ in 3..12 {
        world.step();
    }
    let on_stairs = serde_json::to_value(world.checkpoint()).unwrap();
    assert_eq!(
        on_stairs["state"]["people"]["1"]["location"]["kind"],
        "walking"
    );
    assert_eq!(on_stairs["state"]["people"]["1"]["location"]["from"], 3);

    for _ in 12..30 {
        world.step();
    }
    let at_work = serde_json::to_value(world.checkpoint()).unwrap();
    assert_eq!(
        at_work["state"]["people"]["1"]["location"],
        json!({"kind":"at","place":7})
    );
    assert_eq!(at_work["state"]["development_points"], 1);

    for _ in 30..60 {
        world.step();
    }
    let accounting = serde_json::to_value(world.checkpoint()).unwrap();
    assert_eq!(
        accounting["state"]["balance"],
        150 - 6 - 3 * 4 + 18 + 12 - 3 - 3 - 4 - 6
    );
}
