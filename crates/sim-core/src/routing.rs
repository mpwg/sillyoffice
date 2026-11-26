use crate::{LiftId, LiftStopId, PlaceId, Scenario};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum RouteLeg {
    Walk {
        from: PlaceId,
        to: PlaceId,
        duration: u64,
    },
    Lift {
        lift: LiftId,
        from: LiftStopId,
        to: LiftStopId,
    },
}

pub(crate) type Routes = BTreeMap<PlaceId, Vec<(PlaceId, u64, RouteLeg)>>;

pub(crate) fn add_walk(edges: &mut Routes, from: PlaceId, to: PlaceId, duration: u64) {
    edges
        .entry(from)
        .or_default()
        .push((to, duration, RouteLeg::Walk { from, to, duration }));
}

pub(crate) fn walking_edges(scenario: &Scenario, accessible: impl Fn(PlaceId) -> bool) -> Routes {
    let geometry = scenario.geometry();
    let mut edges = Routes::new();
    for walkway in geometry
        .walkways
        .iter()
        .filter(|edge| accessible(edge.from) && accessible(edge.to))
    {
        add_walk(&mut edges, walkway.from, walkway.to, walkway.ticks.get());
        if walkway.bidirectional {
            add_walk(&mut edges, walkway.to, walkway.from, walkway.ticks.get());
        }
    }
    for stair in geometry
        .stairs
        .iter()
        .filter(|edge| accessible(edge.from) && accessible(edge.to))
    {
        add_walk(&mut edges, stair.from, stair.to, stair.ticks.get());
        add_walk(&mut edges, stair.to, stair.from, stair.ticks.get());
    }
    for outgoing in edges.values_mut() {
        outgoing.sort_unstable();
    }
    edges
}

pub(crate) fn walk(
    edges: &Routes,
    start: PlaceId,
    destination: PlaceId,
    tick: u64,
) -> Option<VecDeque<RouteLeg>> {
    let mut frontier = BinaryHeap::from([Reverse((tick, start))]);
    let mut costs = BTreeMap::from([(start, tick)]);
    let mut previous = BTreeMap::new();
    while let Some(Reverse((cost, from))) = frontier.pop() {
        if from == destination {
            break;
        }
        if costs[&from] != cost {
            continue;
        }
        for &(to, duration, leg) in edges.get(&from).into_iter().flatten() {
            if let Some(end_tick) = cost.checked_add(duration)
                && costs.get(&to).is_none_or(|&known| end_tick < known)
            {
                costs.insert(to, end_tick);
                previous.insert(to, (from, leg));
                frontier.push(Reverse((end_tick, to)));
            }
        }
    }
    if !costs.contains_key(&destination) {
        return None;
    }
    let mut walk = VecDeque::new();
    let mut place = destination;
    while let Some(&(from, leg)) = previous.get(&place) {
        walk.push_front(leg);
        place = from;
    }
    Some(walk)
}
