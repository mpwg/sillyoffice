use serde_json::{Value, json};
use sim_core::{Scenario, World};

fn state(world: &World) -> Value {
    serde_json::to_value(world.checkpoint()).unwrap()
}

fn until(world: &mut World, tick: u64) {
    while world.tick() < tick {
        world.step();
    }
}

fn queue(world: &mut World, json: Value) {
    world
        .submit_next(serde_json::from_value(json).expect("supported prototype action"))
        .unwrap();
}

#[test]
fn repeated_coffee_walks_build_one_downward_shortcut_and_survive_a_reload() {
    let mut raw: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    raw["rules"]["day_ticks"] = json!(1000);
    let scenario = Scenario::from_json(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let mut world = World::new(scenario.clone(), 42).unwrap();
    for (place, designation) in [(6, "home"), (7, "office"), (9, "kiosk")] {
        queue(
            &mut world,
            json!({"kind":"designate","place":place,"use":designation}),
        );
    }
    until(&mut world, 25);
    let coffee = json!({"kind":"coffee_trip","person":1,"kiosk":9});
    queue(&mut world, coffee.clone());
    until(&mut world, 75);
    assert_eq!(state(&world)["state"]["initiatives"], json!({}));
    queue(&mut world, coffee.clone());
    until(&mut world, 98);
    let building = state(&world);
    assert_eq!(
        building["state"]["initiatives"]["10"]["3"]["ready_tick"],
        103
    );
    let mut restored = World::restore(scenario, &world.save().unwrap()).unwrap();
    until(&mut world, 125);
    until(&mut restored, 125);
    assert_eq!(state(&world), state(&restored));

    queue(&mut world, coffee);
    until(&mut world, 130);
    assert_eq!(state(&world)["state"]["people"]["1"]["location"]["from"], 3);
    assert_eq!(state(&world)["state"]["people"]["1"]["location"]["to"], 9);
    until(&mut world, 136);
    let climbing = state(&world);
    assert_eq!(climbing["state"]["people"]["1"]["location"]["from"], 3);
    assert_eq!(climbing["state"]["people"]["1"]["location"]["to"], 4);
    assert_eq!(
        climbing["state"]["initiatives"]["10"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
}
