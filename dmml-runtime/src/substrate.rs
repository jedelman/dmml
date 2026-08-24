//! The trait boundary a concrete backend must satisfy to host a DMML
//! world: write-with-admission-gate, a sovereignty root, and an opaque
//! identity/CID representation. `WorldGraph::apply_commit`
//! (`crate::graph`) never depends on this trait directly today -- it
//! only ever compares `cid: String` values it's handed for equality
//! (written-world's issue #53) -- so this module doesn't yet change
//! that function's behavior. It exists to name, in one place, the
//! contract two real, independently-verified backends already
//! informally satisfy:
//!
//! - **atproto**, via written-world's `server/src/atproto/
//!   commit_write.rs`: `write_commit`/`write_dmml_commit` gate every
//!   write on a real `swapCommit` compare-and-swap against a live PDS,
//!   returning a `StrongRef {uri, cid}` on success and a distinct
//!   `Conflict` error on a stale write -- verified live, not simulated
//!   (`dev-journal/2026-08-18-real-pds-validation.md` in written-world).
//! - **iroh-docs**, via written-world's `spikes/iroh-chain-integrity/`
//!   research: no native compare-and-swap, so the gate has to be
//!   application code (`gated_chain_append.rs`'s `ChainHead`), and real
//!   multi-writer fork resolution needs an explicit `mergeable`/
//!   `arbitrated` policy per consume-kind, not a single uniform rule
//!   (`dev-journal/2026-08-24-multi-tenant-network-dmml-iroh-substrate.md`).
//!
//! **Deliberately not fleshed out yet** -- this is a named stub, not a
//! finished design. Filling in the real method signatures (what a write
//! call actually looks like, how `isCanonicalLeaf()` and the mergeable/
//! arbitrated declaration plug in, how a sovereignty root is
//! represented across an atproto DID and an iroh `NamespaceSecret`
//! without leaking either one's shape into this trait) is real,
//! separate design work -- see the workspace root `ARCHITECTURE.md` for
//! what's still open.

/// Placeholder for the write-admission contract every concrete backend
/// must provide. Intentionally empty beyond a marker method for now --
/// see this module's own doc comment for why the real shape isn't
/// designed yet.
pub trait Substrate {
    /// A human-readable name for this substrate, for logging/debugging
    /// only -- not part of the real contract yet.
    fn name(&self) -> &'static str;
}
