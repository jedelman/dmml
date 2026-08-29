//! Datalog-based spike for the referential-integrity checks in
//! `WorldGraph::apply_commit` (Strong-ref and Fact-ref admissibility).
//!
//! This is a standalone spike, proven by its own equivalence tests only.
//! It does NOT mutate the graph and is NOT wired into `apply_commit`.
//!
//! First drafted by `z-ai/glm-5.3-flash` (dev-tooling dispatch pipeline,
//! see written-world's CLAUDE.md) and corrected by hand before this file
//! would even compile: a stray token, a `use` of a module (`crepe_output`)
//! that doesn't exist -- `crepe!`'s generated types live directly in this
//! module, same as `datalog_guard.rs` -- a `.run()` call destructured as a
//! 2-tuple against a block that declared seven `@output` relations, and
//! every reference to `oxigraph::model::Subject`, which does not exist in
//! this oxigraph version at all (the real type, used everywhere else in
//! this crate, is `NamedOrBlankNode`). The four unused `@output` relations
//! (`StrongNodeKnown`, `StrongNodeUnknown`, `StrongCidMismatch`,
//! `FactQuadMatches`, `FactQuadMismatch`) were never actually consumed by
//! the driver, which re-derives "known"/"matched" imperatively for its
//! error messages instead -- trimmed to plain (non-`@output`) derived
//! relations, keeping only `Admissible`/`Inadmissible` as real outputs.
//!
//! Real, load-bearing finding from this spike: unlike `datalog_guard.rs`
//! (which only ever needed `WorldGraph::all_with_predicate`), this check
//! genuinely needs "does this node appear anywhere in the store, as
//! subject or object, under any predicate" -- a query `WorldGraph` had no
//! method for at all. `apply_commit` itself answers it via direct
//! `self.store` access (a private field), which a sibling module can't
//! reach. `WorldGraph::all_quads` (`graph.rs`) was added specifically to
//! close that gap -- a minimal, crate-visible accessor, not a spike-side
//! guess.

use std::collections::HashMap;

use crepe::crepe;
use oxigraph::model::{NamedNode, NamedOrBlankNode, Term};

use crate::graph::{Commit, ConsumeRef, FactRef, WorldGraph};
use crate::vocab;

// ---------------------------------------------------------------------------
// Symbol table (same pattern as datalog_guard.rs): crepe facts need Copy
// fields, so IRI strings and CID strings are interned as u32 symbols.
// ---------------------------------------------------------------------------

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

/// The comparison key for a Fact-ref's `object` string against a quad's
/// actual object term -- mirrors `apply_commit`'s own (private)
/// `term_matches_fact_object` exactly: a `NamedNode` object only matches
/// via its *decoded* foreign-URI form (`vocab::foreign_uri_from_node`),
/// never its raw encoded IRI string, and a `Literal` matches by value.
/// `None` means "this object can never satisfy a closed-object FactRef" --
/// an ordinary same-repo internal node has no foreign-URI decoding and so
/// (correctly) never matches a specific expected string, only the
/// open-object (`fr.object: None`) form, which doesn't consult this at all.
///
/// Getting this wrong doesn't fail to compile either: the naive first
/// draft (`n.as_str()`, the node's own raw IRI) made every closed-object
/// FactRef against a foreign node fail, silently, because the expected
/// string (`"urn:test:obj"`) never matches the encoded form
/// (`<ns>foreign/urn%3Atest%3Aobj`) -- caught only by actually running the
/// `fact_ref_matching_triple_accepted` test, not by the equivalence
/// helper alone (the naive oracle reimplementation made the identical
/// mistake, so the two agreed with each other while both being wrong).
fn fact_object_key(term: &Term) -> Option<String> {
    match term {
        Term::NamedNode(n) => vocab::foreign_uri_from_node(n),
        Term::Literal(l) => Some(l.value().to_string()),
        _ => None,
    }
}

crepe! {
    @input
    struct NodeAppears(u32);
    // A node (by IRI-string symbol) appears somewhere in the store, as
    // subject or object.

    @input
    struct RecordedCid(u32, u32);
    // (node-sym, cid-sym): every foreignCid ever recorded in the store for
    // the node, PLUS the candidate commit's own via/responds_to cids when
    // their uri matches this node.

    @input
    struct StrongRefCheck(u32, u32, u32);
    // (consume-ref-id, node-sym, cid-sym)

    @input
    struct SubjPred(u32, u32);
    // (subject-sym, predicate-sym): exists for EVERY quad regardless of
    // whether its object decodes to anything comparable -- feeds the
    // open-object (`fr.object: None`) FactRef case, which doesn't care
    // what the object is at all.

    @input
    struct Quad(u32, u32, u32);
    // (subject-sym, predicate-sym, object-sym): only emitted when the
    // object has a `fact_object_key` -- feeds the closed-object FactRef
    // case, which needs the object to actually match.

    @input
    struct FactRefCheckAny(u32, u32, u32);
    // (consume-ref-id, subject-sym, predicate-sym) -- fr.object is None.

    @input
    struct FactRefCheckObj(u32, u32, u32, u32);
    // (consume-ref-id, subject-sym, predicate-sym, object-sym)

    // The store (or this commit's own via/responds_to) has at least one
    // recorded cid for this node.
    struct HasRecordedCid(u32);
    HasRecordedCid(node) <- RecordedCid(node, _cid);

    // This Strong ref's cid is one of the recorded cids for its node.
    struct StrongCidMatches(u32);
    StrongCidMatches(id) <- StrongRefCheck(id, node, cid), RecordedCid(node, cid);

    struct StrongNodeUnknown(u32);
    StrongNodeUnknown(id) <- StrongRefCheck(id, node, _cid), !NodeAppears(node);

    struct StrongCidMismatch(u32);
    StrongCidMismatch(id) <- StrongRefCheck(id, node, _cid), NodeAppears(node), HasRecordedCid(node), !StrongCidMatches(id);

    // Fact refs: the store contains a matching quad.
    struct FactQuadMatches(u32);
    FactQuadMatches(id) <- FactRefCheckAny(id, s, p), SubjPred(s, p);
    FactQuadMatches(id) <- FactRefCheckObj(id, s, p, o), Quad(s, p, o);

    struct FactQuadMismatch(u32);
    FactQuadMismatch(id) <- FactRefCheckAny(id, _s, _p), !FactQuadMatches(id);
    FactQuadMismatch(id) <- FactRefCheckObj(id, _s, _p, _o), !FactQuadMatches(id);

    // Top-level verdicts, stratified over the mismatch predicates above.
    @output
    struct Admissible(u32);
    Admissible(id) <- StrongRefCheck(id, _node, _cid), !StrongNodeUnknown(id), !StrongCidMismatch(id);
    Admissible(id) <- FactRefCheckAny(id, _s, _p), !FactQuadMismatch(id);
    Admissible(id) <- FactRefCheckObj(id, _s, _p, _o), !FactQuadMismatch(id);

    @output
    struct Inadmissible(u32);
    Inadmissible(id) <- StrongNodeUnknown(id);
    Inadmissible(id) <- StrongCidMismatch(id);
    Inadmissible(id) <- FactQuadMismatch(id);
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

enum RefInfo {
    Strong { uri: String, cid: String },
    Fact(FactRef),
}

/// Datalog reimplementation of the referential-integrity *checks* of
/// `WorldGraph::apply_commit` (mutation excluded). Returns `Ok(())` if every
/// `ConsumeRef` in `commit.consumes` is admissible against the graph
/// snapshot, otherwise one reason string per inadmissible ref, mirroring
/// `apply_commit`'s own error text.
pub fn consumes_admissible(graph: &WorldGraph, commit: &Commit) -> Result<(), Vec<String>> {
    let mut sym = SymbolTable::default();
    let mut runtime = Crepe::new();

    // Scan the store once: every quad contributes to (a) the "node
    // appears as subject or object" relation and (b) the full quad
    // relation used for Fact-ref matching; every foreignCid literal
    // additionally contributes to the recorded-cid relation.
    for quad in graph.all_quads() {
        let subject_str = match &quad.subject {
            NamedOrBlankNode::NamedNode(n) => n.as_str().to_string(),
            NamedOrBlankNode::BlankNode(b) => b.as_str().to_string(),
        };
        let sub = sym.intern(&subject_str);
        let pred = sym.intern(quad.predicate.as_str());
        runtime.extend([SubjPred(sub, pred)]);
        runtime.extend([NodeAppears(sub)]);
        if let Some(key) = fact_object_key(&quad.object) {
            let obj = sym.intern(&key);
            runtime.extend([Quad(sub, pred, obj)]);
        }
        // `NodeAppears` also needs to know about NamedNode objects (for
        // the Strong-ref existence check, which cares about raw node
        // identity, not the decoded FactRef key).
        if let Term::NamedNode(n) = &quad.object {
            let obj_node = sym.intern(n.as_str());
            runtime.extend([NodeAppears(obj_node)]);
        }

        // `foreignCid`'s object is a `NamedNode` (`vocab::foreign_cid_node`),
        // not a literal -- see `vocab::foreign_cid_node`/`foreign_cid_from_node`'s
        // own doc comments; `apply_commit` decodes it the same way.
        if quad.predicate == vocab::foreign_cid() {
            if let NamedOrBlankNode::NamedNode(subject_nn) = &quad.subject {
                if let Term::NamedNode(cid_node) = &quad.object {
                    if let Some(cid) = vocab::foreign_cid_from_node(cid_node) {
                        let node_sym = sym.intern(subject_nn.as_str());
                        let cid_sym = sym.intern(&cid);
                        runtime.extend([RecordedCid(node_sym, cid_sym)]);
                    }
                }
            }
        }
    }

    // The candidate commit's own via/responds_to cids count as genuine
    // observations even though they have not been written to the store
    // yet -- same self-consistency rule `apply_commit` itself applies.
    if let Some(via) = &commit.via {
        let node = vocab::foreign_uri_node(&via.uri);
        let node_sym = sym.intern(node.as_str());
        let cid_sym = sym.intern(&via.cid);
        runtime.extend([RecordedCid(node_sym, cid_sym)]);
    }
    if let Some(rt) = &commit.responds_to {
        let node = vocab::foreign_uri_node(&rt.uri);
        let node_sym = sym.intern(node.as_str());
        let cid_sym = sym.intern(&rt.cid);
        runtime.extend([RecordedCid(node_sym, cid_sym)]);
    }

    // Build one check per ConsumeRef.
    let mut infos: Vec<RefInfo> = Vec::with_capacity(commit.consumes.len());
    for (idx, r) in commit.consumes.iter().enumerate() {
        let id = idx as u32;
        match r {
            ConsumeRef::Strong(sr) => {
                let node = vocab::foreign_uri_node(&sr.uri);
                let node_sym = sym.intern(node.as_str());
                let cid_sym = sym.intern(&sr.cid);
                runtime.extend([StrongRefCheck(id, node_sym, cid_sym)]);
                infos.push(RefInfo::Strong {
                    uri: sr.uri.clone(),
                    cid: sr.cid.clone(),
                });
            }
            ConsumeRef::Fact(fr) => {
                let subject_node = vocab::foreign_uri_node(&fr.subject);
                let sub = sym.intern(subject_node.as_str());
                let pred = sym.intern(&fr.predicate);
                match &fr.object {
                    None => {
                        runtime.extend([FactRefCheckAny(id, sub, pred)]);
                    }
                    Some(obj) => {
                        let obj_sym = sym.intern(obj);
                        runtime.extend([FactRefCheckObj(id, sub, pred, obj_sym)]);
                    }
                }
                infos.push(RefInfo::Fact(fr.clone()));
            }
        }
    }

    let (_admissible, inadmissible) = runtime.run();

    if inadmissible.is_empty() {
        return Ok(());
    }

    // Mirror apply_commit's error text. For Strong cid mismatches the
    // message needs the recorded-cid list; recompute it from the same
    // sources (store + via + responds_to) rather than threading it back
    // out of the Datalog symbols.
    let mut reasons = Vec::new();
    for Inadmissible(id) in inadmissible {
        let id = id as usize;
        match &infos[id] {
            RefInfo::Strong { uri, cid } => {
                let node = vocab::foreign_uri_node(uri);
                let known = node_in_store(graph, &node);
                if !known {
                    reasons.push(format!("consumes references unknown node: {uri}"));
                    continue;
                }
                let mut recorded: Vec<String> = graph
                    .all_with_predicate(&vocab::foreign_cid())
                    .into_iter()
                    .filter(|(s, _)| s == &node)
                    .filter_map(|(_, o)| match o {
                        Term::NamedNode(n) => vocab::foreign_cid_from_node(&n),
                        _ => None,
                    })
                    .collect();
                if let Some(via) = &commit.via {
                    if &via.uri == uri {
                        recorded.push(via.cid.clone());
                    }
                }
                if let Some(rt) = &commit.responds_to {
                    if &rt.uri == uri {
                        recorded.push(rt.cid.clone());
                    }
                }
                reasons.push(format!(
                    "consumes cid does not match any cid ever recorded for {uri}: got {cid}, recorded {recorded:?}"
                ));
            }
            RefInfo::Fact(fr) => {
                reasons.push(format!(
                    "consumes references unknown fact: ({}, {}, {:?})",
                    fr.subject, fr.predicate, fr.object
                ));
            }
        }
    }
    Err(reasons)
}

fn node_in_store(graph: &WorldGraph, node: &NamedNode) -> bool {
    graph.all_quads().iter().any(|q| {
        matches!(&q.subject, NamedOrBlankNode::NamedNode(s) if s == node)
            || matches!(&q.object, Term::NamedNode(o) if o == node)
    })
}

// ---------------------------------------------------------------------------
// Tests: equivalence of the Datalog decision against the imperative
// referential-integrity checks of apply_commit, checked on real WorldGraph
// fixtures (same style as datalog_guard.rs's tests).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Delta, StrongRef};

    fn base_commit() -> Commit {
        Commit {
            consumes: Vec::new(),
            produces: String::new(),
            predicate: "urn:test:pred".to_string(),
            via: None,
            responds_to: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn strong(uri: &str, cid: &str) -> ConsumeRef {
        ConsumeRef::Strong(StrongRef {
            uri: uri.to_string(),
            cid: cid.to_string(),
        })
    }

    fn fact(subject: &str, predicate: &str, object: Option<&str>) -> ConsumeRef {
        ConsumeRef::Fact(FactRef {
            commit: StrongRef {
                uri: "urn:test:src".to_string(),
                cid: "cid-src".to_string(),
            },
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.map(|o| o.to_string()),
        })
    }

    /// `WorldGraph` enforces a closed predicate vocabulary -- an ad-hoc
    /// IRI like `urn:test:p` is rejected outright ("not a recognized
    /// predicate") unless self-declared first, same mechanism
    /// `datalog_guard.rs`'s own tests use for its `strength` Attribute.
    /// Every fixture below self-declares this one Relation once per graph
    /// and reuses it for all triples that need a predicate at all.
    const TEST_REL: &str = "urn:test:rel";

    fn declare_test_rel(g: &mut WorldGraph) {
        g.commit(
            "test",
            Delta::new().assert(
                NamedNode::new(TEST_REL).unwrap(),
                vocab::rdf_type(),
                vocab::class_relation(),
            ),
        )
        .expect("declaring the test relation is always valid");
    }

    /// `foreignCid`'s object is a `NamedNode` (`vocab::foreign_cid_node`),
    /// never a literal -- see that function's own doc comment. Getting
    /// this wrong doesn't fail to compile, it fails at commit time
    /// (`WorldGraph`'s validator rejects it: "foreignCid must be a node
    /// reference, not a literal"), which is exactly what the first draft
    /// of this fixture did.
    fn graph_with_node() -> WorldGraph {
        let mut g = WorldGraph::new();
        g.commit(
            "test",
            Delta::new().assert(
                vocab::foreign_uri_node("urn:test:node-a"),
                vocab::foreign_cid(),
                vocab::foreign_cid_node("cid-original"),
            ),
        )
        .expect("fixture commit");
        g
    }

    fn graph_with_triple() -> WorldGraph {
        let mut g = WorldGraph::new();
        declare_test_rel(&mut g);
        g.commit(
            "test",
            Delta::new().assert(
                vocab::foreign_uri_node("urn:test:subj"),
                NamedNode::new(TEST_REL).unwrap(),
                vocab::foreign_uri_node("urn:test:obj"),
            ),
        )
        .expect("fixture commit");
        g
    }

    /// Hand-rolled oracle: the exact imperative checks from apply_commit,
    /// reimplemented inline for comparison.
    fn oracle(graph: &WorldGraph, commit: &Commit) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();
        for r in &commit.consumes {
            match r {
                ConsumeRef::Strong(r) => {
                    let node = vocab::foreign_uri_node(&r.uri);
                    let known = node_in_store(graph, &node);
                    if !known {
                        errs.push(format!("consumes references unknown node: {}", r.uri));
                        continue;
                    }
                    let mut recorded_cids: Vec<String> = graph
                        .all_with_predicate(&vocab::foreign_cid())
                        .into_iter()
                        .filter(|(s, _)| s == &node)
                        .filter_map(|(_, o)| match o {
                            Term::NamedNode(n) => vocab::foreign_cid_from_node(&n),
                            _ => None,
                        })
                        .collect();
                    if let Some(via) = &commit.via {
                        if via.uri == r.uri {
                            recorded_cids.push(via.cid.clone());
                        }
                    }
                    if let Some(rt) = &commit.responds_to {
                        if rt.uri == r.uri {
                            recorded_cids.push(rt.cid.clone());
                        }
                    }
                    if !recorded_cids.is_empty() && !recorded_cids.iter().any(|c| c == &r.cid) {
                        errs.push(format!(
                            "consumes cid does not match any cid ever recorded for {}: got {}, recorded {:?}",
                            r.uri, r.cid, recorded_cids
                        ));
                    }
                }
                ConsumeRef::Fact(fr) => {
                    let subject = vocab::foreign_uri_node(&fr.subject);
                    let predicate = NamedNode::new(&fr.predicate).unwrap();
                    let matches = graph.all_quads().iter().any(|q| {
                        matches!(&q.subject, NamedOrBlankNode::NamedNode(s) if s == &subject)
                            && q.predicate == predicate
                            && match &fr.object {
                                None => true,
                                // Mirrors apply_commit's own (private)
                                // term_matches_fact_object: a NamedNode
                                // object matches via its *decoded*
                                // foreign-URI form, never its raw IRI --
                                // reimplemented independently here, not
                                // delegated to fact_object_key, so this
                                // oracle stays a genuinely separate check.
                                Some(want) => match &q.object {
                                    Term::NamedNode(n) => {
                                        vocab::foreign_uri_from_node(n).as_deref() == Some(want.as_str())
                                    }
                                    Term::Literal(l) => l.value() == want,
                                    _ => false,
                                },
                            }
                    });
                    if !matches {
                        errs.push(format!(
                            "consumes references unknown fact: ({}, {}, {:?})",
                            fr.subject, fr.predicate, fr.object
                        ));
                    }
                }
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    fn assert_agrees(graph: &WorldGraph, commit: &Commit) {
        let ours = consumes_admissible(graph, commit);
        let theirs = oracle(graph, commit);
        match (&ours, &theirs) {
            (Ok(()), Ok(())) => {}
            (Err(a), Err(b)) => {
                assert_eq!(a.len(), b.len(), "verdict counts disagree: {a:?} vs {b:?}");
            }
            _ => panic!("verdicts disagree: ours={ours:?} theirs={theirs:?}"),
        }
    }

    // (a) Strong ref to a genuinely existing node with no recorded cid:
    // node exists via a plain triple, never via foreignCid -> accepted.
    #[test]
    fn strong_existing_node_no_recorded_cid_accepted() {
        let g = graph_with_triple();
        let mut c = base_commit();
        c.consumes.push(strong("urn:test:subj", "cid-anything"));
        assert_agrees(&g, &c);
        assert!(consumes_admissible(&g, &c).is_ok());
    }

    // (b) Strong ref to an unknown node -> rejected.
    #[test]
    fn strong_unknown_node_rejected() {
        let g = WorldGraph::new();
        let mut c = base_commit();
        c.consumes.push(strong("urn:test:nowhere", "cid-x"));
        assert_agrees(&g, &c);
        let errs = consumes_admissible(&g, &c).unwrap_err();
        assert_eq!(errs, vec!["consumes references unknown node: urn:test:nowhere"]);
    }

    // (c) Strong ref whose cid matches a previously-recorded foreignCid -> accepted.
    #[test]
    fn strong_cid_matches_recorded_cid_accepted() {
        let g = graph_with_node();
        let mut c = base_commit();
        c.consumes.push(strong("urn:test:node-a", "cid-original"));
        assert_agrees(&g, &c);
        assert!(consumes_admissible(&g, &c).is_ok());
    }

    // (d) Strong ref whose cid does NOT match any recorded cid (one WAS
    // recorded) -> rejected.
    #[test]
    fn strong_cid_mismatch_rejected() {
        let g = graph_with_node();
        let mut c = base_commit();
        c.consumes.push(strong("urn:test:node-a", "cid-wrong"));
        assert_agrees(&g, &c);
        let errs = consumes_admissible(&g, &c).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].starts_with(
            "consumes cid does not match any cid ever recorded for urn:test:node-a: got cid-wrong"
        ));
    }

    // (e) The subtle self-consistency case: the cid matches only the
    // candidate commit's OWN `via.cid` (nothing in the store yet) -> accepted.
    #[test]
    fn strong_cid_matches_own_via_cid_accepted() {
        let mut g = WorldGraph::new();
        declare_test_rel(&mut g);
        g.commit(
            "test",
            Delta::new().assert(
                vocab::foreign_uri_node("urn:test:node-b"),
                NamedNode::new(TEST_REL).unwrap(),
                vocab::foreign_uri_node("urn:test:other"),
            ),
        )
        .expect("fixture commit");

        let mut c = base_commit();
        c.via = Some(StrongRef {
            uri: "urn:test:node-b".to_string(),
            cid: "cid-from-via".to_string(),
        });
        c.consumes.push(strong("urn:test:node-b", "cid-from-via"));
        assert_agrees(&g, &c);
        assert!(consumes_admissible(&g, &c).is_ok());

        // ...but a *different* cid in the same shape is rejected, because
        // via's cid was recorded and does not match.
        let mut c2 = base_commit();
        c2.via = Some(StrongRef {
            uri: "urn:test:node-b".to_string(),
            cid: "cid-from-via".to_string(),
        });
        c2.consumes.push(strong("urn:test:node-b", "cid-other"));
        assert_agrees(&g, &c2);
        assert!(consumes_admissible(&g, &c2).is_err());
    }

    // (e2) Same subtlety via responds_to.
    #[test]
    fn strong_cid_matches_own_responds_to_cid_accepted() {
        let mut g = WorldGraph::new();
        declare_test_rel(&mut g);
        g.commit(
            "test",
            Delta::new().assert(
                vocab::foreign_uri_node("urn:test:node-c"),
                NamedNode::new(TEST_REL).unwrap(),
                vocab::foreign_uri_node("urn:test:other"),
            ),
        )
        .expect("fixture commit");

        let mut c = base_commit();
        c.responds_to = Some(StrongRef {
            uri: "urn:test:node-c".to_string(),
            cid: "cid-from-responds".to_string(),
        });
        c.consumes.push(strong("urn:test:node-c", "cid-from-responds"));
        assert_agrees(&g, &c);
        assert!(consumes_admissible(&g, &c).is_ok());
    }

    // (f) Fact ref matching an existing triple -> accepted (both the
    // open-object and closed-object forms).
    #[test]
    fn fact_ref_matching_triple_accepted() {
        let g = graph_with_triple();
        let mut c = base_commit();
        c.consumes.push(fact("urn:test:subj", TEST_REL, None));
        c.consumes.push(fact("urn:test:subj", TEST_REL, Some("urn:test:obj")));
        assert_agrees(&g, &c);
        assert!(consumes_admissible(&g, &c).is_ok());
    }

    // (g) Fact ref matching no triple -> rejected (wrong subject, wrong
    // predicate, and wrong object each produce a reason).
    #[test]
    fn fact_ref_no_matching_triple_rejected() {
        let g = graph_with_triple();
        let mut c = base_commit();
        c.consumes.push(fact("urn:test:missing", TEST_REL, None));
        c.consumes.push(fact("urn:test:subj", "urn:test:q", None));
        c.consumes.push(fact("urn:test:subj", TEST_REL, Some("urn:test:not-the-object")));
        assert_agrees(&g, &c);
        let errs = consumes_admissible(&g, &c).unwrap_err();
        assert_eq!(errs.len(), 3);
        assert!(errs.iter().all(|e| e.starts_with("consumes references unknown fact:")));
    }

    // Mixed batch: one admissible Strong ref and one inadmissible Fact ref
    // yields exactly one reason naming the failing ref only.
    #[test]
    fn mixed_batch_reports_only_failures() {
        let g = graph_with_node();
        let mut c = base_commit();
        c.consumes.push(strong("urn:test:node-a", "cid-original")); // ok
        c.consumes.push(fact("urn:test:node-a", "urn:test:nope", None)); // fails
        assert_agrees(&g, &c);
        let errs = consumes_admissible(&g, &c).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].starts_with("consumes references unknown fact: (urn:test:node-a, urn:test:nope"));
    }

    // Vacuous-cid check is per-node: an empty recorded set for one node must
    // not relax the check for another node with a recorded cid.
    #[test]
    fn vacuous_cid_check_is_per_node() {
        let mut g = graph_with_node(); // node-a has cid-original recorded
        declare_test_rel(&mut g);
        g.commit(
            "test",
            Delta::new().assert(
                vocab::foreign_uri_node("urn:test:node-d"),
                NamedNode::new(TEST_REL).unwrap(),
                vocab::foreign_uri_node("urn:test:other"),
            ),
        )
        .expect("fixture commit"); // node-d has no recorded cid

        let mut c = base_commit();
        c.consumes.push(strong("urn:test:node-d", "anything-goes")); // vacuous: ok
        c.consumes.push(strong("urn:test:node-a", "anything-goes")); // must fail
        assert_agrees(&g, &c);
        let errs = consumes_admissible(&g, &c).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("urn:test:node-a"));
    }
}
