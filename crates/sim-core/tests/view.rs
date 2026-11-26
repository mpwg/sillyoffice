use sim_core::{FloorId, Scenario, ViewRect, World};

#[test]
fn moving_person_appears_only_on_the_floors_of_the_current_segment() {
    let scenario =
        Scenario::from_json(include_bytes!("../../../godot/content/zwei-etagen.json")).unwrap();
    let mut world = World::new(scenario, 42).unwrap();
    let reception = ViewRect {
        first_floor: FloorId(0),
        last_floor: FloorId(0),
    };
    let office = ViewRect {
        first_floor: FloorId(1),
        last_floor: FloorId(1),
    };
    let initial = world.checkpoint();
    let start = world.view(reception);
    assert_eq!(start.tick, 0);
    assert_eq!(start.people.len(), 1);
    assert!(world.view(office).people.is_empty());
    assert_eq!(initial.checksum, world.checkpoint().checksum);

    for _ in 0..4 {
        world.step();
    }
    let lower = world.view(reception);
    let upper = world.view(office);
    assert_eq!(lower.tick, 4);
    assert_eq!(lower.people, upper.people);
    assert_eq!(lower.people.len(), 1);

    for _ in 4..12 {
        world.step();
    }
    assert!(world.view(reception).people.is_empty());
    let end = world.view(office);
    assert_eq!(end.people.len(), 1);
    assert_eq!(start.people[0].id, end.people[0].id);
}
