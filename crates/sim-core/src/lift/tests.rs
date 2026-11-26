use super::*;

const LOWER: LiftStopId = LiftStopId(4);
const MIDDLE: LiftStopId = LiftStopId(2);
const UPPER: LiftStopId = LiftStopId(8);
const LIFT: LiftId = LiftId(1);
const NEIGHBOR: LiftId = LiftId(2);

fn system(capacity: u32) -> LiftSystem {
    let mut system = LiftSystem::default();
    system.insert(LIFT, line(capacity)).unwrap();
    system
}

fn line(capacity: u32) -> LiftLine {
    LiftLine::new(
        vec![LOWER, MIDDLE, UPPER],
        NonZeroU32::new(capacity).unwrap(),
        NonZeroU64::new(2).unwrap(),
        NonZeroU64::MIN,
    )
    .unwrap()
}

fn request(
    system: &mut LiftSystem,
    lift: LiftId,
    person: u32,
    from: LiftStopId,
    to: LiftStopId,
    tick: u64,
) {
    system
        .request(lift, PersonId(person), from, to, tick)
        .unwrap();
}

#[test]
fn serves_earlier_arrivals_then_lower_ids_and_clears_the_queue() {
    let mut system = system(1);
    request(&mut system, LIFT, 1, LOWER, UPPER, 2);
    request(&mut system, LIFT, 4, LOWER, UPPER, 1);
    request(&mut system, LIFT, 3, LOWER, UPPER, 1);
    let boarded: Vec<_> = (2..100)
        .flat_map(|tick| system.step(tick))
        .filter_map(|event| match event {
            LiftEvent::Boarded { person, .. } => Some(person.0),
            _ => None,
        })
        .collect();
    assert_eq!(boarded, [3, 4, 1]);
    assert_eq!(system.waiting_count(LIFT), 0);
    assert_eq!(system.rider_count(LIFT), 0);
}

#[test]
fn leaves_an_opposite_direction_call_until_the_car_turns() {
    let mut system = system(2);
    request(&mut system, LIFT, 1, LOWER, UPPER, 1);
    request(&mut system, LIFT, 2, MIDDLE, LOWER, 1);
    request(&mut system, LIFT, 3, MIDDLE, UPPER, 1);
    let events: Vec<_> = (1..60).flat_map(|tick| system.step(tick)).collect();
    let movements: Vec<_> = events
        .into_iter()
        .map(|event| match event {
            LiftEvent::Boarded { person, .. } => (true, person.0),
            LiftEvent::Exited { person, .. } => (false, person.0),
            _ => panic!("ordinary service must not reroute a passenger"),
        })
        .collect();
    assert_eq!(
        movements,
        [
            (true, 1),
            (true, 3),
            (false, 1),
            (false, 3),
            (true, 2),
            (false, 2)
        ]
    );
}

#[test]
fn shared_access_gives_exiting_passengers_priority_over_boarding() {
    let mut system = system(1);
    system.insert(NEIGHBOR, line(1)).unwrap();
    request(&mut system, LIFT, 1, LOWER, MIDDLE, 1);
    request(&mut system, NEIGHBOR, 2, LOWER, MIDDLE, 1);
    request(&mut system, LIFT, 3, MIDDLE, UPPER, 1);
    request(&mut system, NEIGHBOR, 4, MIDDLE, UPPER, 1);

    let mut at_middle = Vec::new();
    for tick in 1..40 {
        let events: Vec<_> = system
            .step(tick)
            .into_iter()
            .filter(|event| {
                matches!(
                    event,
                    LiftEvent::Boarded { stop: MIDDLE, .. }
                        | LiftEvent::Exited { stop: MIDDLE, .. }
                )
            })
            .collect();
        assert!(
            events.len() <= 1,
            "one access place was reused in tick {tick}"
        );
        at_middle.extend(events);
    }
    assert_eq!(
        at_middle.first(),
        Some(&LiftEvent::Exited {
            lift: LIFT,
            person: PersonId(1),
            stop: MIDDLE
        })
    );
    assert_eq!(at_middle.len(), 4);
    assert_eq!(
        system.waiting_count(LIFT) + system.waiting_count(NEIGHBOR),
        0
    );
}

#[test]
fn widening_shared_access_shortens_the_same_arrival_group_wait() {
    let total_wait = |width| {
        let mut system = system(6);
        system.insert(NEIGHBOR, line(6)).unwrap();
        system.set_access_width(LOWER, NonZeroU32::new(width).unwrap());
        for person in 1..=6 {
            let lift = if person % 2 == 0 { LIFT } else { NEIGHBOR };
            request(&mut system, lift, person, LOWER, UPPER, 1);
        }
        let mut total = 0;
        for tick in 1..100 {
            for event in system.step(tick) {
                if matches!(event, LiftEvent::Boarded { .. }) {
                    total += tick - 1;
                }
            }
        }
        assert_eq!(
            system.waiting_count(LIFT) + system.waiting_count(NEIGHBOR),
            0
        );
        assert_eq!(system.rider_count(LIFT) + system.rider_count(NEIGHBOR), 0);
        total
    };
    assert!(total_wait(4) < total_wait(1));
}

#[test]
fn neighboring_stops_use_the_same_access_slots() {
    let adjacent = LiftStopId(12);
    let mut system = system(2);
    let neighbor = LiftLine::new(
        vec![adjacent, MIDDLE, UPPER],
        NonZeroU32::new(2).unwrap(),
        NonZeroU64::new(2).unwrap(),
        NonZeroU64::MIN,
    )
    .unwrap();
    system.insert(NEIGHBOR, neighbor).unwrap();
    system.set_access_group(adjacent, LOWER);
    request(&mut system, LIFT, 2, LOWER, UPPER, 1);
    request(&mut system, NEIGHBOR, 1, adjacent, UPPER, 1);
    assert_eq!(
        system.step(1),
        [LiftEvent::Boarded {
            lift: NEIGHBOR,
            person: PersonId(1),
            stop: adjacent,
        }]
    );
    assert_eq!(
        system.step(2),
        [LiftEvent::Boarded {
            lift: LIFT,
            person: PersonId(2),
            stop: LOWER,
        }]
    );
}

#[test]
fn an_interruption_keeps_the_lower_prefix_and_releases_riders_at_valid_stops() {
    let mut system = system(2);
    request(&mut system, LIFT, 1, LOWER, UPPER, 1);
    request(&mut system, LIFT, 2, UPPER, LOWER, 1);
    assert_eq!(
        system.step(1),
        [LiftEvent::Boarded {
            lift: LIFT,
            person: PersonId(1),
            stop: LOWER
        }]
    );
    assert_eq!(
        system.affected_by_interrupt(LIFT, &BTreeSet::from([UPPER])),
        [
            InterruptedRide {
                person: PersonId(1),
                destination: UPPER,
                last_stop: LOWER,
                next_stop: Some(MIDDLE),
            },
            InterruptedRide {
                person: PersonId(2),
                destination: LOWER,
                last_stop: UPPER,
                next_stop: None,
            },
        ]
    );
    assert_eq!(
        system
            .lines()
            .next()
            .unwrap()
            .1
            .rider_destination(PersonId(1)),
        Some(UPPER)
    );
    let changes = system.interrupt(LIFT, &BTreeSet::from([UPPER]));
    assert_eq!(
        changes,
        [LiftEvent::Replan {
            lift: LIFT,
            person: PersonId(2),
            stop: None
        }]
    );
    assert_eq!(system.lines().next().unwrap().1.stops(), &[LOWER, MIDDLE]);
    assert_eq!(
        system
            .lines()
            .next()
            .unwrap()
            .1
            .rider_destination(PersonId(1)),
        Some(MIDDLE)
    );
    let rider_changes: Vec<_> = (2..30).flat_map(|tick| system.step(tick)).collect();
    assert_eq!(
        rider_changes,
        [LiftEvent::Replan {
            lift: LIFT,
            person: PersonId(1),
            stop: Some(MIDDLE)
        }]
    );
    assert_eq!(system.rider_count(LIFT), 0);
}

#[test]
fn shutting_down_the_line_releases_a_rider_at_the_next_accessible_stop() {
    let mut system = system(1);
    request(&mut system, LIFT, 1, MIDDLE, UPPER, 1);
    system.step(1);
    system.step(2);
    assert_eq!(
        system.step(3),
        [LiftEvent::Boarded {
            lift: LIFT,
            person: PersonId(1),
            stop: MIDDLE,
        }]
    );
    assert_eq!(
        system.interrupt(LIFT, &BTreeSet::from([MIDDLE])),
        [LiftEvent::Replan {
            lift: LIFT,
            person: PersonId(1),
            stop: Some(UPPER),
        }]
    );
    assert!(system.lines().next().is_none());
}

#[test]
fn an_interruption_brings_a_returning_car_to_the_next_preserved_stop() {
    let mut system = system(1);
    request(&mut system, LIFT, 1, LOWER, UPPER, 1);
    request(&mut system, LIFT, 2, UPPER, LOWER, 7);
    for tick in 1..7 {
        system.step(tick);
    }
    assert_eq!(
        system.step(7),
        [LiftEvent::Boarded {
            lift: LIFT,
            person: PersonId(2),
            stop: UPPER,
        }]
    );
    assert_eq!(
        system.affected_by_interrupt(LIFT, &BTreeSet::from([UPPER])),
        [InterruptedRide {
            person: PersonId(2),
            destination: LOWER,
            last_stop: UPPER,
            next_stop: Some(MIDDLE),
        }]
    );
    assert!(system.interrupt(LIFT, &BTreeSet::from([UPPER])).is_empty());
    assert_eq!(system.lines().next().unwrap().1.car_stop(), MIDDLE);
    assert_eq!(system.rider_count(LIFT), 1);
    let arrivals: Vec<_> = (8..20).flat_map(|tick| system.step(tick)).collect();
    assert_eq!(
        arrivals,
        [LiftEvent::Exited {
            lift: LIFT,
            person: PersonId(2),
            stop: LOWER,
        }]
    );
    assert_eq!(system.rider_count(LIFT), 0);
}

#[test]
fn a_saved_cabin_and_queue_continue_in_the_same_order() {
    let mut system = system(1);
    request(&mut system, LIFT, 1, LOWER, UPPER, 1);
    request(&mut system, LIFT, 2, LOWER, UPPER, 1);
    system.step(1);
    system.step(2);
    assert_eq!(system.rider_count(LIFT), 1);
    assert_eq!(system.waiting_count(LIFT), 1);
    let mut restored: LiftSystem =
        serde_json::from_slice(&serde_json::to_vec(&system).unwrap()).unwrap();
    for tick in 3..50 {
        assert_eq!(restored.step(tick), system.step(tick));
    }
    assert_eq!(restored, system);
    assert_eq!(restored.waiting_count(LIFT), 0);
    assert_eq!(restored.rider_count(LIFT), 0);
}

#[test]
fn save_rejects_a_moving_cabin_with_no_direction() {
    let mut system = system(1);
    request(&mut system, LIFT, 1, LOWER, UPPER, 1);
    system.step(1);
    system.step(2);
    let mut saved = serde_json::to_value(system).unwrap();
    saved["lines"]["1"]["direction"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<LiftSystem>(saved).is_err());
}

#[test]
fn rejects_duplicate_reservations_and_cancels_the_original() {
    let mut system = system(2);
    system.insert(NEIGHBOR, line(2)).unwrap();
    request(&mut system, LIFT, 1, LOWER, UPPER, 1);
    assert_eq!(
        system.request(NEIGHBOR, PersonId(1), UPPER, LOWER, 1),
        Err(LiftError::DuplicatePerson)
    );
    system.cancel(PersonId(1));
    assert!(system.step(1).is_empty());
    request(&mut system, NEIGHBOR, 1, LOWER, UPPER, 2);
    assert_eq!(
        system.step(2),
        [LiftEvent::Boarded {
            lift: NEIGHBOR,
            person: PersonId(1),
            stop: LOWER
        }]
    );
}
