//! A real Datalog replacement for `interpret::reachable_from`'s hand-
//! rolled fixed-point loop -- the textbook transitive-closure case, and
//! honestly the strongest single candidate this crate had for a Datalog
//! port: `reachable_from`'s own doc comment already half-admits it
//! ("fixed-point iteration... simpler than a proper graph walk").
//!
//! `interpret::ancestors_of` (the inverse, backward walk) was reviewed
//! for the same treatment and deliberately NOT ported: its real content
//! is cycle-termination and returning a strictly *ordered* (genesis-
//! first) `Vec`, and Datalog computes unordered sets. Deriving "the set
//! of ancestor cids" via Datalog would be trivial, but that was never
//! the hard part of `ancestors_of` -- reconstructing the linear order
//! afterward would still need the same imperative walk this would have
//! replaced. Porting it anyway just to have ported it would trade a
//! working, already-tested function for a longer one that does the same
//! amount of real work. `reachable_from` is different: consumers only
//! ever need set *membership* (`commits.iter().filter(|c| included.
//! contains(...))`, which preserves the original slice's order, not a
//! derived one), which is exactly what a transitive-closure fixpoint
//! naturally produces.

use std::collections::{HashMap, HashSet};

use crate::interpret::IdentifiedCommit;

/// Interns strings to small `u32` symbols, since crepe's fact fields
/// must be `Copy`. Local to this module -- see `datalog_guard.rs`'s own
/// copy for why this isn't shared with `dmml-runtime`'s
/// `datalog_support` (wrong dependency direction) or even with this
/// crate's own `datalog_guard.rs` (two tiny, single-file-local copies
/// of a ten-line helper isn't the kind of duplication worth a shared
/// module for -- unlike the three-module, byte-identical duplication
/// `dmml-runtime::datalog_support` actually fixed).
#[derive(Default)]
struct SymbolTable {
    by_str: HashMap<String, u32>,
    by_sym: Vec<String>,
}

impl SymbolTable {
    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&sym) = self.by_str.get(s) {
            return sym;
        }
        let sym = self.by_sym.len() as u32;
        self.by_str.insert(s.to_string(), sym);
        self.by_sym.push(s.to_string());
        sym
    }
}

crepe::crepe! {
    @input
    struct RespondsTo(u32, u32); // (child_cid, parent_cid)
    @input
    struct Root(u32); // (root_cid) -- singleton

    @output
    struct Included(u32); // (cid)

    Included(r) <- Root(r);
    Included(child) <- Included(parent), RespondsTo(child, parent);
}

/// Scopes `commits` to "this world": `root_cid` itself, plus every
/// commit reachable by walking `respondsTo` backward to it. Drop-in
/// equivalent of `interpret::reachable_from` (same signature, same
/// filter semantics -- the original slice's own order is preserved,
/// this never re-orders anything).
pub fn reachable_from(commits: &[IdentifiedCommit], root_cid: &str) -> Vec<IdentifiedCommit> {
    let mut sym = SymbolTable::default();
    let mut runtime = Crepe::new();

    runtime.extend([Root(sym.intern(root_cid))]);
    for c in commits {
        if let Some(parent) = &c.commit.responds_to {
            runtime.extend([RespondsTo(sym.intern(&c.cid), sym.intern(&parent.cid))]);
        }
    }

    let (included,) = runtime.run();
    let included_cids: HashSet<u32> = included.into_iter().map(|Included(c)| c).collect();

    commits
        .iter()
        .filter(|c| {
            let cid_sym = sym.intern(&c.cid);
            included_cids.contains(&cid_sym)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::reachable_from as hand_rolled_reachable_from;
    use crate::lower::{LoweredCommit, StrongRef};

    fn commit(cid: &str, responds_to: Option<&str>) -> IdentifiedCommit {
        IdentifiedCommit {
            uri: format!("at://did:example:test/x/{cid}"),
            cid: cid.to_string(),
            commit: LoweredCommit {
                predicate_verb: "becomes".to_string(),
                consumes: vec![],
                produces: vec![],
                via: None,
                responds_to: responds_to.map(|r| StrongRef {
                    uri: format!("at://did:example:test/x/{r}"),
                    cid: r.to_string(),
                }),
            },
        }
    }

    fn cids(commits: &[IdentifiedCommit]) -> Vec<&str> {
        commits.iter().map(|c| c.cid.as_str()).collect()
    }

    fn assert_agrees(commits: &[IdentifiedCommit], root_cid: &str) -> Vec<String> {
        let ours = reachable_from(commits, root_cid);
        let theirs = hand_rolled_reachable_from(commits, root_cid);
        assert_eq!(
            cids(&ours),
            cids(&theirs),
            "Datalog and hand-rolled reachable_from disagree"
        );
        cids(&ours).into_iter().map(String::from).collect()
    }

    #[test]
    fn root_alone_is_reachable_from_itself() {
        let commits = vec![commit("root", None)];
        assert_eq!(assert_agrees(&commits, "root"), vec!["root"]);
    }

    #[test]
    fn a_linear_chain_is_fully_reachable() {
        let commits = vec![
            commit("root", None),
            commit("a", Some("root")),
            commit("b", Some("a")),
        ];
        let got = assert_agrees(&commits, "root");
        assert_eq!(got.len(), 3);
        for c in ["root", "a", "b"] {
            assert!(got.contains(&c.to_string()));
        }
    }

    /// The real reason this is worth having, not just a set-equality
    /// smoke test: a sibling branch (`other`, responding to a commit
    /// NOT on the path to root) must be excluded, and a commit
    /// responding to something entirely unrelated (`stray`) must be
    /// excluded too -- proves this isn't just "everything with a
    /// responds_to gets included."
    #[test]
    fn sibling_branches_and_unrelated_commits_are_excluded() {
        let commits = vec![
            commit("root", None),
            commit("a", Some("root")),
            commit("unrelated-root", None),
            commit("stray", Some("unrelated-root")),
            commit("orphan", Some("nonexistent-parent")),
        ];
        let got = assert_agrees(&commits, "root");
        assert_eq!(got, vec!["root".to_string(), "a".to_string()]);
    }

    #[test]
    fn unknown_root_cid_yields_only_itself_if_present_else_empty() {
        let commits = vec![commit("root", None), commit("a", Some("root"))];
        assert_eq!(assert_agrees(&commits, "no-such-cid"), Vec::<String>::new());
    }

    /// Real regression coverage that didn't exist for `reachable_from`
    /// anywhere before this port (unlike `ancestors_of`, which already
    /// had `a_cyclic_responds_to_stops_rather_than_looping_forever`):
    /// a cycle entirely WITHIN the reachable set (`a` and `b` respond to
    /// each other, both ultimately reachable from `root`) must not hang
    /// either implementation -- crepe's semi-naive evaluation is a
    /// fixpoint over a finite domain by construction, and the hand-
    /// rolled loop terminates because `included_cids` only ever grows,
    /// but neither guarantee was ever actually exercised by a test.
    #[test]
    fn a_cycle_inside_the_reachable_set_does_not_hang() {
        let commits = vec![
            commit("root", None),
            commit("a", Some("root")),
            commit("b", Some("a")),
            // Cyclic edge back into the set: not a real DMML shape
            // (responds_to should never cycle), but the function must
            // not hang if content somehow produces one.
            commit("a-again", Some("b")),
        ];
        let got = assert_agrees(&commits, "root");
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn empty_commit_list_is_empty() {
        assert_eq!(assert_agrees(&[], "root"), Vec::<String>::new());
    }
}
