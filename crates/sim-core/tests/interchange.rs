use serde_json::{Value, json};
use sim_core::{Location, Outcome, PersonId, PlaceId, Scenario, World};

fn commuters() -> Scenario {
    let mut raw: Value =
        serde_json::from_slice(include_bytes!("../../../godot/content/erstes-haus.json")).unwrap();
    raw["rules"]["day_ticks"] = json!(1000);
    raw["initial"]["applicants"] = json!([]);
    raw["initial"]["people"] = json!(
        (1..=8)
            .map(|id| json!({"id":id,"home":6,"office":7,"location":7}))
            .collect::<Vec<_>>()
    );
    raw["geometry"]["places"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":13,"section":3,"column":6,"kind":"space"}));
    for place in raw["geometry"]["places"].as_array_mut().unwrap() {
        if place["id"] == 6 {
            place["kind"] = json!("home");
        }
        if place["id"] == 7 {
            place["kind"] = json!("office");
        }
    }
    Scenario::from_json(&serde_json::to_vec(&raw).unwrap()).unwrap()
}

fn order(world: &mut World, action: Value) {
    world
        .submit_next(serde_json::from_value(action).unwrap())
        .unwrap();
}

fn tick(world: &mut World) {
    for result in world.step().results {
        assert_eq!(result.outcome, Outcome::Applied);
    }
}

fn finish(world: &mut World) {
    while !arrived(world) && world.tick() < 120 {
        tick(world);
    }
    assert!(arrived(world), "finite demand must reach home");
    assert_eq!(world.transport_stats().waiting_people, 0);
}

fn arrived(world: &World) -> bool {
    (1..=8)
        .all(|id| world.person_location(PersonId(id)) == Some(Location::At { place: PlaceId(6) }))
}

fn rush_hour(scenario: &Scenario, wide: bool) -> (u64, u64) {
    let mut world = World::new(scenario.clone(), 42).unwrap();
    tick(&mut world);
    if wide {
        for room in [9, 13] {
            order(
                &mut world,
                json!({"kind":"designate","place":room,"use":"corridor"}),
            );
        }
    }
    order(&mut world, json!({"kind":"unlock_lift"}));
    order(&mut world, json!({"kind":"build_lift","stops":[1,2,3]}));
    tick(&mut world);

    for id in 1..=8 {
        order(
            &mut world,
            json!({"kind":"commute","person":id,"toward":"home"}),
        );
    }
    while world.transport_stats().lift_boardings == 0 && world.tick() < 25 {
        tick(&mut world);
    }
    assert!(matches!(
        world.person_location(PersonId(1)),
        Some(Location::RidingLift { .. })
    ));
    assert!(matches!(
        world.person_location(PersonId(8)),
        Some(Location::WaitingLift { .. })
    ));
    finish(&mut world);
    let stats = world.transport_stats();
    assert_eq!(stats.lift_boardings, 8);
    (stats.total_waited_ticks, world.tick())
}

#[test]
fn widening_the_shared_lift_access_shortens_the_wait_for_the_same_group() {
    let scenario = commuters();
    let (narrow_wait, _) = rush_hour(&scenario, false);
    let (wide_wait, _) = rush_hour(&scenario, true);
    assert!(wide_wait < narrow_wait, "{wide_wait} >= {narrow_wait}");
}

#[test]
fn residents_take_the_lift_when_it_beats_the_same_stair_route() {
    let scenario = commuters();
    let mut stairs = World::new(scenario.clone(), 42).unwrap();
    tick(&mut stairs);
    tick(&mut stairs);
    for id in 1..=8 {
        order(
            &mut stairs,
            json!({"kind":"commute","person":id,"toward":"home"}),
        );
    }
    finish(&mut stairs);
    assert_eq!(stairs.transport_stats().lift_boardings, 0);

    let (_, lift_arrival) = rush_hour(&scenario, true);
    assert!(
        lift_arrival < stairs.tick(),
        "{lift_arrival} >= {}",
        stairs.tick()
    );
}
