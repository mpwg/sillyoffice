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

#[test]
fn a_simple_lift_can_serve_fifteen_floors() {
    let mut raw: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    for floor in 3..16 {
        let section = 100 + floor;
        raw["geometry"]["floors"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":floor,"width":14}));
        raw["geometry"]["sections"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":section,"floor":floor,"start":0,"end":14,"rent_cost":0}));
        raw["geometry"]["places"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":200+floor,"section":section,"column":5,"kind":"landing"}));
        raw["geometry"]["lift_stops"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":floor+1,"floor":floor,"column":5}));
        raw["initial"]["rented_sections"]
            .as_array_mut()
            .unwrap()
            .push(json!(section));
    }
    let scenario = Scenario::from_json(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let mut world = World::new(scenario, 4).unwrap();
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
    let valid = action(json!({"kind":"build_lift","stops":(1..=15).collect::<Vec<_>>()}));
    let invalid = action(json!({"kind":"build_lift","stops":(1..=16).collect::<Vec<_>>()}));
    assert_eq!(world.preview(&valid).unwrap().stops.len(), 15);
    assert_eq!(
        world.preview(&invalid).unwrap_err(),
        sim_core::Rejection::InvalidLiftStops
    );
}

#[test]
fn lift_preview_identifies_the_first_unrented_halt_before_unlocking() {
    let mut raw: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    raw["initial"]["rented_sections"] = json!([1]);
    let world = World::new(
        Scenario::from_json(&serde_json::to_vec(&raw).unwrap()).unwrap(),
        4,
    )
    .unwrap();
    let build = action(json!({"kind":"build_lift","stops":[1,2,3]}));
    assert_eq!(
        world.first_blocked_lift_stop(&build),
        Some(sim_core::LiftStopId(2))
    );
    assert_eq!(
        world.preview(&build).unwrap_err(),
        sim_core::Rejection::LiftLocked
    );
}
