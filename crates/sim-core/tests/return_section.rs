use serde_json::{Value, json};
use sim_core::{Action, Outcome, Replay, Scenario, World};

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

#[test]
fn returning_an_active_coffee_destination_keeps_the_game_restorable() {
    let mut house: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    house["geometry"]["sections"][0]["end"] = json!(8);
    house["geometry"]["sections"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":5,"floor":0,"start":8,"end":10,"rent_cost":3}));
    let kiosk = house["geometry"]["places"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|place| place["id"] == 9)
        .unwrap();
    kiosk["section"] = json!(5);
    house["initial"]["rented_sections"]
        .as_array_mut()
        .unwrap()
        .push(json!(5));
    let scenario = Scenario::from_json(&serde_json::to_vec(&house).unwrap()).unwrap();

    let replay = active_coffee_replay();
    for (cut_after, expected_movement) in [(24, "walking"), (30, "riding_lift")] {
        let mut world = replay.start(scenario.clone()).unwrap();
        while world.tick() < cut_after {
            world.step();
        }
        let mut return_action = json!({"kind":"return_section","section":5,"confirmation":""});
        return_action["confirmation"] = json!(
            world
                .preview(&action(return_action.clone()))
                .unwrap()
                .confirmation
                .unwrap()
        );
        world.submit_next(action(return_action)).unwrap();
        assert_eq!(world.step().results[0].outcome, Outcome::Applied);
        let checkpoint = serde_json::to_value(world.checkpoint()).unwrap();
        assert_eq!(
            checkpoint["state"]["people"]["1"]["location"]["kind"],
            expected_movement
        );

        let saved = world.save().unwrap();
        let restored = World::restore(scenario.clone(), &saved).unwrap_or_else(|error| {
            panic!(
                "Spielstand nach Rückgabe bei Tick {} ist unladbar: {error}",
                world.tick()
            )
        });
        assert_eq!(
            serde_json::to_value(restored.checkpoint()).unwrap(),
            checkpoint
        );
    }
}

#[test]
fn returning_the_only_access_to_an_active_coffee_trip_keeps_the_game_restorable() {
    let mut house: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    house["geometry"]["sections"][0]["end"] = json!(5);
    house["geometry"]["sections"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!({"id":5,"floor":0,"start":5,"end":6,"rent_cost":3}),
            json!({"id":6,"floor":0,"start":6,"end":10,"rent_cost":3}),
        ]);
    for place in house["geometry"]["places"].as_array_mut().unwrap() {
        match place["id"].as_u64() {
            Some(3) => place["section"] = json!(5),
            Some(9) => place["section"] = json!(6),
            _ => {}
        }
    }
    house["initial"]["rented_sections"]
        .as_array_mut()
        .unwrap()
        .extend([json!(5), json!(6)]);
    let scenario = Scenario::from_json(&serde_json::to_vec(&house).unwrap()).unwrap();
    let replay = active_coffee_replay();

    for (cut_after, expected_anchor) in [(35, 4), (36, 9)] {
        let mut world = replay.start(scenario.clone()).unwrap();
        while world.tick() < cut_after {
            world.step();
        }
        let mut return_action = json!({"kind":"return_section","section":5,"confirmation":""});
        return_action["confirmation"] = json!(
            world
                .preview(&action(return_action.clone()))
                .unwrap()
                .confirmation
                .unwrap()
        );
        world.submit_next(action(return_action)).unwrap();
        assert_eq!(world.step().results[0].outcome, Outcome::Applied);
        assert_eq!(
            world.person_location(sim_core::PersonId(1)),
            Some(sim_core::Location::At {
                place: sim_core::PlaceId(expected_anchor),
            })
        );

        let saved = world.save().unwrap();
        let restored = World::restore(scenario.clone(), &saved).unwrap_or_else(|error| {
            panic!(
                "Spielstand nach Rückgabe bei Tick {} ist unladbar: {error}",
                world.tick()
            )
        });
        assert_eq!(
            serde_json::to_value(restored.checkpoint()).unwrap(),
            serde_json::to_value(world.checkpoint()).unwrap()
        );
    }
}

fn active_coffee_replay() -> Replay {
    let mut day: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/spieltag.json")).unwrap();
    day["commands"]
        .as_array_mut()
        .unwrap()
        .retain(|command| command["tick"].as_u64().unwrap() <= 24);
    Replay::from_json(&serde_json::to_vec(&day).unwrap()).unwrap()
}

#[test]
fn one_rider_leaves_the_building_while_the_other_gets_one_safe_stop() {
    use sim_core::{HouseholdId, LiftStopId, Location, PersonId, PlaceId};

    let mut house: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    house["initial"]["applicants"] = json!([]);
    house["initial"]["rented_sections"] = json!([1, 2, 3, 4]);
    house["initial"]["people"] = json!([
        {"id":1,"home":7,"office":9,"location":9},
        {"id":2,"home":12,"office":9,"location":9}
    ]);
    for place in house["geometry"]["places"].as_array_mut().unwrap() {
        match place["id"].as_u64() {
            Some(7 | 12) => place["kind"] = json!("home"),
            Some(9) => place["kind"] = json!("office"),
            _ => {}
        }
    }
    let scenario = Scenario::from_json(&serde_json::to_vec(&house).unwrap()).unwrap();
    let mut world = World::new(scenario.clone(), 42).unwrap();
    world.step();
    for value in [
        json!({"kind":"unlock_lift"}),
        json!({"kind":"build_lift","stops":[1,2,3]}),
    ] {
        world.submit_next(action(value)).unwrap();
    }
    world.step();
    for person in [1, 2] {
        world
            .submit_next(action(
                json!({"kind":"commute","person":person,"toward":"home"}),
            ))
            .unwrap();
    }
    let in_car = (0..20).any(|_| {
        world.step();
        [PersonId(1), PersonId(2)].into_iter().all(|person| {
            matches!(
                world.person_location(person),
                Some(Location::RidingLift { .. })
            )
        })
    });
    assert!(in_car);

    let mut command = json!({"kind":"return_section","section":3});
    let preview = world.preview(&action(command.clone())).unwrap();
    assert_eq!(preview.households_leaving, [HouseholdId(1)]);
    assert_eq!(preview.affected_people, [PersonId(1), PersonId(2)]);
    assert_eq!(preview.remaining_stops, [LiftStopId(1), LiftStopId(2)]);
    command["confirmation"] = json!(preview.confirmation);
    world.submit_next(action(command)).unwrap();
    assert_eq!(world.step().results[0].outcome, Outcome::Applied);
    assert!(world.person_location(PersonId(1)).is_none());

    for _ in 0..15 {
        world.step();
    }
    assert!(matches!(
        world.person_location(PersonId(2)),
        Some(Location::At { place: PlaceId(4) })
    ));
    assert_eq!(world.transport_stats().waiting_people, 0);
    let restored = World::restore(scenario, &world.save().unwrap()).unwrap();
    assert_eq!(world.checkpoint().checksum, restored.checkpoint().checksum);
}
