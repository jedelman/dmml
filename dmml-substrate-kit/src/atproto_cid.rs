//! The atproto-specific CID strategy, extracted verbatim from written-
//! world's original `dmml::identity` module when this repo was split
//! out of it. Computes the real `CIDv1(dag-cbor, sha2-256)` of a commit,
//! over the exact wire shape atproto's own `com.atproto.repo.
//! createRecord` expects for `org.jason-edelman.writtenworld.commit`
//! records -- this is one CONCRETE strategy `dmml-runtime`'s `Substrate`
//! trait can be satisfied by, not something `dmml` itself needs to know
//! about. An iroh-docs substrate would use a different module in this
//! same crate (raw BLAKE3, wrapped as a CIDv1 under the registered
//! BLAKE3 multicodec per `dev-journal/2026-08-24-multi-tenant-network-
//! dmml-iroh-substrate.md`'s proposal -- not yet built here).
//!
//! **Honest limit, carried over unchanged from the original module**:
//! the computed CID is only byte-identical to what a real atproto PDS
//! would compute for "the same" logical commit if `produces`' rendered
//! text ALSO matches byte-for-byte at the subject/object level -- true
//! for predicates (`dmml::identity::predicate_wire`, cross-checked
//! against written-world's own `engine::vocab::dynamic_predicate` in
//! that crate's own test suite), not yet true for subjects/objects
//! (`dmml::identity::render_object`'s own doc comment: they render as
//! bare local identifiers, not confirmed against production's real
//! blank-node convention beyond one observed sample).

use dmml::identity::render_produces;
use dmml::lower::{ConsumeRef, LoweredCommit, StrongRef, Triple};
use cid::Cid;
use multihash::Multihash;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Real atproto record type NSID this wire shape targets.
const COMMIT_NSID: &str = "org.jason-edelman.writtenworld.commit";

/// The `dag-cbor` multicodec code (IPLD's registered value).
const DAG_CBOR_CODEC: u64 = 0x71;
/// The `sha2-256` multihash code (the multiformats registered value).
const SHA2_256_CODE: u64 = 0x12;

#[derive(Serialize)]
struct WireStrongRef<'a> {
    uri: &'a str,
    cid: &'a str,
}

impl<'a> From<&'a StrongRef> for WireStrongRef<'a> {
    fn from(r: &'a StrongRef) -> Self {
        WireStrongRef {
            uri: &r.uri,
            cid: &r.cid,
        }
    }
}

#[derive(Serialize)]
struct WireFactRef<'a> {
    commit: WireStrongRef<'a>,
    subject: &'a str,
    predicate: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    object: Option<String>,
}

/// Mirrors written-world's `server/src/atproto/commit_write.rs::
/// WireConsumeRef` exactly: `serde`'s internally-tagged representation
/// (`tag = "$type"`) matches the real atproto union wire convention
/// directly.
#[derive(Serialize)]
#[serde(tag = "$type")]
enum WireConsumeRef<'a> {
    #[serde(rename = "com.atproto.repo.strongRef")]
    Strong(WireStrongRef<'a>),
    #[serde(rename = "org.jason-edelman.writtenworld.commit#factRef")]
    Fact(WireFactRef<'a>),
}

#[derive(Serialize)]
struct WireCommit<'a> {
    #[serde(rename = "$type")]
    type_: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    consumes: Vec<WireConsumeRef<'a>>,
    #[serde(skip_serializing_if = "str::is_empty")]
    produces: &'a str,
    predicate: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    via: Option<WireStrongRef<'a>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "respondsTo")]
    responds_to: Option<WireStrongRef<'a>>,
    #[serde(rename = "createdAt")]
    created_at: &'a str,
}

fn wire_consume_ref(r: &ConsumeRef) -> WireConsumeRef<'_> {
    match r {
        ConsumeRef::Strong(s) => WireConsumeRef::Strong(WireStrongRef::from(s)),
        ConsumeRef::Fact(f) => WireConsumeRef::Fact(WireFactRef {
            commit: WireStrongRef::from(&f.commit),
            subject: &f.subject,
            predicate: &f.predicate,
            object: f.object.as_ref().map(dmml::identity::render_object),
        }),
    }
}

/// Computes the real `CIDv1(dag-cbor, sha2-256)` of `commit`, given the
/// externally-supplied `created_at` -- per written-world's `SPEC.md`,
/// not something DMML syntax produces at all; populated by the
/// authoring tool at commit time, same as production. See the module
/// doc comment for the honest limit on byte-compatibility with a real
/// PDS's computed CID for "the same" logical commit.
pub fn compute_cid(commit: &LoweredCommit, created_at: &str) -> Cid {
    let produces_text = render_produces(&commit.produces);
    let wire = WireCommit {
        type_: COMMIT_NSID,
        consumes: commit.consumes.iter().map(wire_consume_ref).collect(),
        produces: &produces_text,
        predicate: &commit.predicate_verb,
        via: commit.via.as_ref().map(WireStrongRef::from),
        responds_to: commit.responds_to.as_ref().map(WireStrongRef::from),
        created_at,
    };

    let bytes = serde_ipld_dagcbor::to_vec(&wire).expect("WireCommit is always serializable");
    let digest = Sha256::digest(&bytes);
    let mh = Multihash::<64>::wrap(SHA2_256_CODE, &digest)
        .expect("a 32-byte digest fits a 64-byte multihash");
    Cid::new_v1(DAG_CBOR_CODEC, mh)
}

/// Pairs a triple's pure-content CID with the DID of the repo treating
/// it as its own. Mirrors `StrongRef`'s own `{ uri, cid }` shape
/// (location + content-hash, kept deliberately separate) rather than
/// inventing a new pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripleRef {
    pub owner_did: String,
    pub triple: Cid,
}

/// `CIDv1(dag-cbor, sha2-256)` over `(subject, predicate, object)` alone
/// -- no DID, no commit context.
pub fn triple_cid(triple: &Triple) -> Cid {
    let bytes = serde_ipld_dagcbor::to_vec(triple).expect("Triple is always serializable");
    let digest = Sha256::digest(&bytes);
    let mh = Multihash::<64>::wrap(SHA2_256_CODE, &digest)
        .expect("a 32-byte digest fits a 64-byte multihash");
    Cid::new_v1(DAG_CBOR_CODEC, mh)
}

/// Builds a `TripleRef` pairing `triple_cid(triple)` with `owner_did`.
pub fn make_triple_ref(owner_did: &str, triple: &Triple) -> TripleRef {
    TripleRef {
        owner_did: owner_did.to_string(),
        triple: triple_cid(triple),
    }
}

/// The actual verification a resolver runs before honoring a
/// retraction: both the recomputed triple CID AND the owner DID must
/// match `reference` -- both must hold.
pub fn triple_ref_matches(reference: &TripleRef, owner_did: &str, triple: &Triple) -> bool {
    reference.owner_did == owner_did && reference.triple == triple_cid(triple)
}
