use crate::{PlaceId, Scenario, WalkSegment};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};

pub(crate) type WalkingEdges = BTreeMap<PlaceId, Vec<(PlaceId, u64)>>;

pub(crate) fn walking_edges(
    scenario: &Scenario,
    accessible: impl Fn(PlaceId) -> bool,
) -> WalkingEdges {
    let geometry = scenario.geometry();
    let mut edges = WalkingEdges::new();
    for walkway in geometry
        .walkways
        .iter()
        .filter(|edge| accessible(edge.from) && accessible(edge.to))
    {
        edges
            .entry(walkway.from)
            .or_default()
            .push((walkway.to, walkway.ticks.get()));
        if walkway.bidirectional {
            edges
                .entry(walkway.to)
                .or_default()
                .push((walkway.from, walkway.ticks.get()));
        }
    }
    for stair in geometry
        .stairs
        .iter()
        .filter(|edge| accessible(edge.from) && accessible(edge.to))
    {
        edges
            .entry(stair.from)
            .or_default()
            .push((stair.to, stair.ticks.get()));
        edges
            .entry(stair.to)
            .or_default()
            .push((stair.from, stair.ticks.get()));
    }
    for outgoing in edges.values_mut() {
        outgoing.sort_unstable();
    }
    edges
}

pub(crate) fn walk(
    edges: &WalkingEdges,
    start: PlaceId,
    destination: PlaceId,
    tick: u64,
) -> Option<VecDeque<WalkSegment>> {
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
        for &(to, duration) in edges.get(&from).into_iter().flatten() {
            if let Some(end_tick) = cost.checked_add(duration)
                && costs.get(&to).is_none_or(|&known| end_tick < known)
            {
                costs.insert(to, end_tick);
                previous.insert(
                    to,
                    WalkSegment {
                        from,
                        to,
                        start_tick: cost,
                        end_tick,
                    },
                );
                frontier.push(Reverse((end_tick, to)));
            }
        }
    }
    if !costs.contains_key(&destination) {
        return None;
    }
    let mut walk = VecDeque::new();
    let mut place = destination;
    while let Some(segment) = previous.get(&place) {
        walk.push_front(*segment);
        place = segment.from;
    }
    Some(walk)
}
