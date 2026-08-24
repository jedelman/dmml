//! Substrate-blind identity/rendering helpers: how a DMML predicate
//! renders to a real, namespaced IRI, and how `produces` triples render
//! to N-Quads text. No hashing, no CID computation, no wire-record shape
//! lives here — those are substrate-specific concerns (a CIDv1(dag-cbor,
//! sha2-256) atproto commit hashes the same content very differently
//! than a raw-BLAKE3 iroh-blobs hash would), and moved out to
//! `dmml-substrate-kit`'s `atproto_cid` module when this crate was
//! extracted from written-world (see that crate's own module doc
//! comment for what actually lives there and why).
//!
//! `StrongRef`/`ConsumeRef` (`crate::lower`) stay exactly what they were
//! in the monolithic version: opaque `{uri, cid: String}` pairs. This
//! crate never computes or verifies a `cid` value itself — it only ever
//! carries one through, the same way `dmml-runtime`'s `apply_commit`
//! referential-integrity check (written-world's engine crate, issue #53)
//! only ever compares `cid` strings for equality, never re-derives a
//! hash. That's the substrate-blindness property this whole extraction
//! exists to preserve.

/// Namespaces a predicate's local name the way a real materializer's own
/// dynamic-predicate vocabulary does: strip to `[A-Za-z0-9_-]`, prefix
/// with `http://ww/`. Kept `pub` specifically so a substrate-kit's own
/// wire-rendering code (which needs this to build a byte-compatible
/// wire record) can call it without duplicating the namespacing rule.
pub const WW_NS: &str = "http://ww/";

/// Real RDF `rdf:type` IRI -- the one predicate that is never
/// `ww`-namespaced.
pub const RDF_TYPE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Namespaces a predicate's local name: strip to `[A-Za-z0-9_-]`, prefix
/// with `WW_NS`.
pub fn predicate_iri(local: &str) -> String {
    let cleaned: String = local.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
    format!("{WW_NS}{cleaned}")
}

/// `predicate_iri`, except `rdf:type` maps to the real RDF IRI instead
/// of being `ww`-namespaced like every other predicate.
pub fn predicate_wire(local: &str) -> String {
    if local == "rdf:type" {
        RDF_TYPE_IRI.to_string()
    } else {
        predicate_iri(local)
    }
}

/// Renders a `crate::lower::TripleValue` as N-Quads object text.
/// **Open gap, not glossed over**: `Node` renders as a bare local
/// identifier (`<room/1>`), not a real namespaced or blank-node form --
/// a real, observed production record used blank nodes for its
/// subjects, but that was one sample, not a verified general rule.
pub fn render_object(v: &crate::lower::TripleValue) -> String {
    use crate::lower::TripleValue;
    match v {
        TripleValue::Node(s) => format!("<{s}>"),
        TripleValue::Number(s) => {
            format!("\"{s}\"^^<http://www.w3.org/2001/XMLSchema#decimal>")
        }
        TripleValue::Boolean(b) => {
            format!("\"{b}\"^^<http://www.w3.org/2001/XMLSchema#boolean>")
        }
        TripleValue::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
    }
}

/// Renders `triples` as N-Quads text, one line per triple, in order.
/// Predicates are real, namespaced IRIs (`predicate_wire`); subjects and
/// objects are not yet (see `render_object`'s own doc comment). `pub`
/// specifically so a substrate-kit's wire-record builder can reuse this
/// instead of re-deriving N-Quads rendering itself.
pub fn render_produces(triples: &[crate::lower::Triple]) -> String {
    triples
        .iter()
        .map(|t| {
            format!(
                "<{}> <{}> {} .\n",
                t.subject,
                predicate_wire(&t.predicate),
                render_object(&t.object)
            )
        })
        .collect()
}
