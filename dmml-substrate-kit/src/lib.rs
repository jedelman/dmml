//! Substrate-specific strategies and a zero-network mock, kept out of
//! both `dmml` (the ontology, which should never need to know how a
//! `cid` is actually computed -- only that it's an opaque string) and
//! `dmml-runtime` (the materializer, which only ever compares `cid`
//! strings for equality, never derives one). See the workspace root
//! `ARCHITECTURE.md` for the full reasoning behind this 3-crate split.
//!
//! `atproto_cid`: the real `CIDv1(dag-cbor, sha2-256)` strategy,
//! extracted verbatim from written-world's original `dmml::identity`
//! module. A concrete `iroh_cid` module (wrapping iroh-blobs' raw
//! BLAKE3 hashes as CIDv1 under the registered BLAKE3 multicodec, per
//! written-world's `dev-journal/2026-08-24-multi-tenant-network-dmml-
//! iroh-substrate.md`) and an in-memory mock `Substrate` implementation
//! for testing `dmml-runtime` are both real, named next steps -- not yet
//! built here.

pub mod atproto_cid;
