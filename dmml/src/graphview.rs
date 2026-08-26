//! Generic graph export for visualizing any set of `IdentifiedCommit`s --
//! the "dmml browser"'s data layer. Deliberately substrate-blind and
//! content-agnostic: this module knows nothing about Benjamin, papers,
//! or autoregressive critique cycles specifically. It only knows the
//! shape every DMML commit already has (`consumes`/`produces`), and
//! turns that into a plain node/edge graph plus a computed generation
//! depth -- how many consumes-hops back to the nearest node with no
//! in-graph dependency -- which is exactly what makes an autoregressive
//! or recombinant structure visually legible: base facts sit at
//! generation 0, first-order readings of them at generation 1, a
//! critique of a critique at generation 2, and so on.
//!
//! A `consumes` entry that cites something outside the given commit set
//! (a foreign repository, or simply a commit the caller didn't include)
//! is preserved as an edge with no resolved source node -- this mirrors
//! `SPEC.md`'s own referential-integrity story (Section 1 of the
//! desiring-production paper): citation is checkable, not required to
//! resolve, and a dangling-outside-this-view edge is real information,
//! not an error.

use crate::interpret::IdentifiedCommit;
use crate::lower::{ConsumeRef, TripleValue};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct GraphTriple {
    pub subject: String,
    pub predicate: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub uri: String,
    pub cid: String,
    /// The DID segment of the URI (`at://did:plc:foo/...` -> `did:plc:foo`),
    /// or the raw URI's scheme-stripped prefix if it isn't `at://`-shaped --
    /// this is the closest thing to "author" a bare commit carries.
    pub author: String,
    pub verb: String,
    pub produces: Vec<GraphTriple>,
    /// 0 for a node with no in-graph consumes edge (a base fact); one
    /// more than the maximum generation of everything it consumes that
    /// IS in this graph, otherwise. A node that only cites outside this
    /// graph is also generation 0 -- it has nothing in-graph to build on.
    pub generation: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from: String,
    /// `Some(uri)` when the cited fact's producing commit is itself in
    /// this graph; `None` when the citation points outside it (a
    /// foreign repository, or a commit the caller didn't include).
    pub to: Option<String>,
    pub subject: String,
    pub predicate: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphExport {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

fn triple_value_to_string(v: &TripleValue) -> String {
    match v {
        TripleValue::Node(s) => s.clone(),
        TripleValue::Number(s) => s.clone(),
        TripleValue::Boolean(b) => b.to_string(),
        TripleValue::Str(s) => s.clone(),
    }
}

fn author_of(uri: &str) -> String {
    uri.strip_prefix("at://")
        .and_then(|rest| rest.split('/').next())
        .unwrap_or(uri)
        .to_string()
}

/// Builds a node/edge/generation view of `commits`. Order-independent:
/// generation is computed from the actual consumes/produces structure,
/// not from `commits`' input order, so passing the same set in a
/// different order produces the identical export.
pub fn export_graph(commits: &[IdentifiedCommit]) -> GraphExport {
    // Which commit (by uri) produces each (subject, predicate)? Last
    // writer wins for materialized VALUE (Materialized's own rule), but
    // for graph-edge purposes every producer is a candidate source --
    // an edge should point at the specific commit a consumes entry cites
    // by its own (uri, cid), not at whichever commit happens to be the
    // materializer's current winner for that key.
    let uri_in_graph: HashMap<&str, &IdentifiedCommit> =
        commits.iter().map(|c| (c.uri.as_str(), c)).collect();

    let mut edges = Vec::new();
    for commit in commits {
        for consume in &commit.commit.consumes {
            let (target_uri, subject, predicate) = match consume {
                ConsumeRef::Fact(f) => (
                    f.commit.uri.as_str(),
                    f.subject.clone(),
                    f.predicate.clone(),
                ),
                ConsumeRef::Strong(s) => (s.uri.as_str(), String::new(), String::new()),
            };
            let to = uri_in_graph
                .get(target_uri)
                .map(|c| c.uri.clone())
                .filter(|u| u != &commit.uri); // no self-loops from a commit citing its own prior fact
            edges.push(GraphEdge {
                from: commit.uri.clone(),
                to,
                subject,
                predicate,
            });
        }
    }

    // Generation: 0 if a node cites nothing in-graph; otherwise one more
    // than the max generation among what it cites in-graph. Commits here
    // are already causally ordered (a commit can only consume something
    // produced earlier), so a single forward pass suffices -- no cycle
    // handling needed, matching DMML's own no-in-place-edit guarantee
    // that consumes can never reach a commit not yet in the log.
    let mut generation: HashMap<&str, u32> = HashMap::new();
    for commit in commits {
        let in_graph_targets: Vec<&str> = edges
            .iter()
            .filter(|e| e.from == commit.uri)
            .filter_map(|e| e.to.as_deref())
            .collect();
        let gen = in_graph_targets
            .iter()
            .map(|t| *generation.get(t).unwrap_or(&0) + 1)
            .max()
            .unwrap_or(0);
        generation.insert(&commit.uri, gen);
    }

    let nodes = commits
        .iter()
        .map(|c| GraphNode {
            uri: c.uri.clone(),
            cid: c.cid.clone(),
            author: author_of(&c.uri),
            verb: c.commit.predicate_verb.clone(),
            produces: c
                .commit
                .produces
                .iter()
                .filter(|t| t.predicate != "rdf:type")
                .map(|t| GraphTriple {
                    subject: t.subject.clone(),
                    predicate: t.predicate.clone(),
                    value: triple_value_to_string(&t.object),
                })
                .collect(),
            generation: *generation.get(c.uri.as_str()).unwrap_or(&0),
        })
        .collect();

    GraphExport { nodes, edges }
}
