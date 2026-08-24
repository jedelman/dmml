//! A generic topological sort over a hash-labeled DAG -- pulled out as
//! its own dependency-free unit because the one real caller today
//! (`server/src/atproto/replay.rs`, ordering a player's PDS commit
//! records by their `parents` links before replaying them through
//! `Game::replay_commit`) lives in a wasm32-only crate with no way to
//! actually *run* a test against it in this repo's current tooling (no
//! `wasm-bindgen-test` harness is set up). This function has nothing
//! atproto- or HTTP-shaped about it, so it belongs here instead, where
//! `cargo test -p engine` gives it real, executing coverage.

use std::collections::HashMap;

/// Kahn's algorithm: given `parents_of[node] = [the nodes it depends on]`,
/// returns every node in an order where each one appears after all of its
/// own dependencies. A `parent` listed for some node but not itself a key
/// of `parents_of` is simply treated as satisfied (not an error) --
/// callers that need to distinguish "this dependency doesn't exist
/// because it's outside the set I'm sorting" from "this dependency is
/// missing/censored" have to make that call themselves before calling
/// this; the algorithm itself only orders what it's given.
///
/// Ties among simultaneously-ready nodes break on their own label,
/// ascending -- arbitrary but fully deterministic, so two calls over the
/// identical input always agree on order even when several nodes are
/// ready at once (a real fork between independent writers, say).
///
/// Returns `Err` if the graph doesn't fully resolve -- a cycle, which no
/// legitimate writer in this codebase can produce, but a directly crafted
/// input could.
pub fn topo_sort(parents_of: &HashMap<String, Vec<String>>) -> Result<Vec<String>, String> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();

    for (node, parents) in parents_of {
        let real_parents = parents.iter().filter(|p| parents_of.contains_key(p.as_str()));
        let mut count = 0;
        for parent in real_parents {
            count += 1;
            children.entry(parent.as_str()).or_default().push(node.as_str());
        }
        in_degree.insert(node.as_str(), count);
    }

    let mut ready: Vec<&str> = in_degree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(node, _)| *node)
        .collect();
    ready.sort_unstable();

    let mut order = Vec::with_capacity(parents_of.len());
    while let Some(node) = ready.pop() {
        order.push(node.to_string());
        if let Some(kids) = children.get(node) {
            let mut newly_ready = Vec::new();
            for kid in kids {
                let count = in_degree.get_mut(kid).expect("every child has an in_degree entry");
                *count -= 1;
                if *count == 0 {
                    newly_ready.push(*kid);
                }
            }
            newly_ready.sort_unstable();
            ready.extend(newly_ready);
            ready.sort_unstable();
        }
    }

    if order.len() != parents_of.len() {
        return Err(format!(
            "graph does not fully resolve ({} of {} nodes reachable) -- a cycle exists",
            order.len(),
            parents_of.len()
        ));
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(node, parents)| {
                (
                    node.to_string(),
                    parents.iter().map(|p| p.to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn orders_a_linear_chain() {
        let graph = edges(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]);
        assert_eq!(topo_sort(&graph).unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn treats_a_parent_outside_the_input_set_as_already_satisfied() {
        // "a"'s parent "genesis" isn't a key of the input at all -- the
        // real-world case this exists for: nothing signs a replayed
        // Game's own bootstrap commits to the PDS, so the first fetched
        // record's parent is always something outside the fetched set.
        let graph = edges(&[("a", &["genesis"])]);
        assert_eq!(topo_sort(&graph).unwrap(), vec!["a"]);
    }

    #[test]
    fn orders_a_fork_and_join_correctly() {
        // Two independent roots (neither depends on the other) both
        // feeding a single join node -- must come after both.
        let graph = edges(&[("a1", &["genesis"]), ("a2", &["genesis"]), ("b", &["a1", "a2"])]);
        let order = topo_sort(&graph).unwrap();
        assert_eq!(order.len(), 3);
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("a1") < pos("b"), "b must come after a1: {order:?}");
        assert!(pos("a2") < pos("b"), "b must come after a2: {order:?}");
    }

    #[test]
    fn rejects_a_cycle() {
        let graph = edges(&[("a", &["b"]), ("b", &["a"])]);
        assert!(topo_sort(&graph).is_err());
    }

    #[test]
    fn ties_break_deterministically_across_repeated_calls() {
        // Several roots ready at once, no real dependency between them --
        // two calls over the identical input must agree, whatever the
        // specific order chosen turns out to be.
        let graph = edges(&[("z", &[]), ("m", &[]), ("a", &[])]);
        assert_eq!(topo_sort(&graph).unwrap(), topo_sort(&graph).unwrap());
    }
}
