use serde_json::{Value, json};
use sim_core::{Action, Outcome, Scenario, World};

fn action(value: Value) -> Action {
    serde_json::from_value(value).expect("supported return action")
}

fn at_work(home_in_wing: bool) -> World {
    let mut raw: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    raw["rules"]["day_ticks"] = json!(1000);
    if home_in_wing {
        raw["initial"]["applicants"][0]["size"] = json!(2);
    }
    let scenario = Scenario::from_json(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let mut world = World::new(scenario, 42).unwrap();
    let mut actions = vec![
        json!({"kind":"rent_section","section":4}),
        json!({"kind":"designate","place":12,"use":if home_in_wing {"home"} else {"office"}}),
        json!({"kind":"designate","place":if home_in_wing {7} else {6},"use":if home_in_wing {"office"} else {"home"}}),
    ];
    if home_in_wing {
        actions.push(json!({"kind":"designate","place":11,"use":"office"}));
    }
    for command in actions {
        world.submit_next(action(command)).unwrap();
    }
    while world.tick() < 30 {
        world.step();
    }
    world
}

fn confirmation(world: &World) -> String {
    world
        .preview(&action(
            json!({"kind":"return_section","section":4,"confirmation":""}),
        ))
        .unwrap()
        .confirmation
        .unwrap()
}

#[test]
fn returning_a_workplace_keeps_the_home_and_reassigns_an_available_job() {
    let mut world = at_work(false);
    let preview = world
        .preview(&action(
            json!({"kind":"return_section","section":4,"confirmation":""}),
        ))
        .unwrap();
    assert_eq!(
        preview.jobs_lost.iter().map(|id| id.0).collect::<Vec<_>>(),
        [1]
    );
    assert!(preview.households_leaving.is_empty());

    world
        .submit_next(action(json!({"kind":"designate","place":7,"use":"office"})))
        .unwrap();
    world
        .submit_next(action(
            json!({"kind":"return_section","section":4,"confirmation":preview.confirmation}),
        ))
        .unwrap();
    let results = world.step().results;
    assert_eq!(results[1].outcome, Outcome::Applied);
    let state: Value = serde_json::to_value(world.checkpoint()).unwrap();
    assert_eq!(state["state"]["households"]["1"], 6);
    assert_eq!(state["state"]["people"]["1"]["office"], 7);
    assert_eq!(state["state"]["designations"]["12"], "vacant");
}

#[test]
fn stale_confirmation_preserves_a_household_until_its_return_is_confirmed() {
    let mut world = at_work(true);
    let before = serde_json::to_value(world.checkpoint()).unwrap();
    world
        .submit_next(action(
            json!({"kind":"return_section","section":4,"confirmation":"obsolete"}),
        ))
        .unwrap();
    let result = world.step().results[0].outcome;
    assert!(matches!(result, Outcome::Rejected { .. }));
    assert_eq!(world.explain(sim_core::PersonId(1)).unwrap().home.0, 12);
    assert_eq!(world.explain(sim_core::PersonId(2)).unwrap().home.0, 12);
    let now = serde_json::to_value(world.checkpoint()).unwrap();
    assert_eq!(before["state"]["balance"], now["state"]["balance"]);

    let token = confirmation(&world);
    world
        .submit_next(action(
            json!({"kind":"return_section","section":4,"confirmation":token}),
        ))
        .unwrap();
    assert_eq!(world.step().results[0].outcome, Outcome::Applied);
    assert!(world.explain(sim_core::PersonId(1)).is_none());
    assert!(world.explain(sim_core::PersonId(2)).is_none());
}
