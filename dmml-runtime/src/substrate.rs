//! The trait boundary a concrete backend must satisfy to host a DMML
//! world: a sovereignty root, an opaque identity, and reads -- writes
//! are deliberately NOT on this base trait. `WorldGraph::apply_commit`
//! (`crate::graph`) never depends on this trait directly today -- it
//! only ever compares `cid: String` values it's handed for equality
//! (written-world's issue #53) -- so this module doesn't change that
//! function's behavior. It names the real contract two independently-
//! verified backends need, honestly split rather than unified:
//!
//! - **atproto**, via written-world's `server/src/atproto/
//!   commit_write.rs`: every write is gated by a real `swapCommit`
//!   compare-and-swap against a live PDS, verified live, not simulated
//!   (`dev-journal/2026-08-18-real-pds-validation.md` in written-world).
//!   [`CasSubstrate`] names this shape.
//! - **iroh**, via written-world's `spikes/iroh-chain-integrity/`
//!   research: writes are author-partitioned (iroh-docs keys entries by
//!   `(namespace, author, key)`), so concurrent writers from different
//!   authors never collide at the storage layer at all -- there is
//!   nothing for a compare-and-swap to gate. [`AppendSubstrate`] names
//!   this shape, and its own doc comment explains why conflict
//!   detection has to be the caller's job instead.
//!
//! Design history: `dmml/ARCHITECTURE.md`'s "Live deployment shape" and
//! "Open design work" sections carry the full decision trail this trait
//! implements (client split, why the only real conflict shape is two
//! commits `consumes`-citing the same prior fact, why resolution is
//! `disputes` rather than arbitration, why the conflict check reuses
//! `appview`'s `getResolved` rather than needing a new primitive). This
//! module is that decision trail made real, not a fresh design.
//!
//! One trait, or capability traits split by write shape? Capability
//! traits, deliberately: an enum-shaped single write method would force
//! the iroh side to carry a `Conflict` variant it can never actually
//! produce (author-partitioned writes cannot collide) and the atproto
//! side to accept an `expected` parameter that means nothing without a
//! real swap underneath it -- "does this backend do CAS" would become a
//! runtime question instead of a type-level fact. [`Substrate`] carries
//! only what both backends genuinely share (identity, the sovereignty
//! root, reads); [`CasSubstrate`] and [`AppendSubstrate`] each carry
//! exactly the write contract its backend can actually honor, and
//! implement neither on the wrong type.
//!
//! Methods return `impl Future<...> + Send` rather than being declared
//! `async fn` directly: stable native async-in-traits (Rust 1.75+, this
//! workspace is on a newer toolchain already) desugars `async fn` in a
//! trait to exactly this shape but without a way to name the `Send`
//! bound, which the compiler flags as a real, worth-fixing lint for a
//! public trait -- confirmed by actually building this and reading the
//! warning, not assumed. No `async-trait` crate dependency either way:
//! nothing elsewhere in this workspace holds a `Box<dyn Substrate>` or
//! needs trait-object dispatch across the two backends, so paying that
//! crate's boxing cost for hypothetical future flexibility isn't
//! warranted yet. Revisit if a caller ever genuinely needs to hold
//! "whichever substrate" behind one pointer.

use oxigraph::model::{NamedNode, Term};

use crate::graph::{Commit, FactRef};

/// Proof that a commit is durably published, common to both backends.
/// Deliberately just the CID: a backend that needs more (an atproto
/// rev, an iroh entry hash) exposes it on its own concrete type, not
/// here -- the trait boundary only ever needs the opaque, comparable
/// string every other part of this system already treats CIDs as
/// (`dmml::identity`'s own doc comment makes the same choice for the
/// same reason).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceipt {
    pub cid: String,
}

/// A commit read back from the substrate, with the CID it's stored
/// under -- what `get_commit` returns, and what a caller resolving a
/// `consumes` reference (a `StrongRef` or a `FactRef.commit`) needs to
/// inspect what that reference actually produced.
#[derive(Debug, Clone)]
pub struct CommitRecord {
    pub cid: String,
    pub commit: Commit,
}

/// One assertion of `predicate` on `subject`, tagged with the commit
/// that produced it. `Substrate::assertions` returns every one of
/// these ever produced for a given `(subject, predicate)` pair -- a
/// bare `produces` (no `consumes`) never overwrites anything, so
/// multiple independent assertions genuinely coexist (`pantheon.rs`'s
/// Helios/Selene/Eos), and the caller (not this trait) folds them into
/// a current value by walking commit-log order.
#[derive(Debug, Clone)]
pub struct Assertion {
    pub commit: String,
    pub object: Term,
}

/// Whether a specific prior fact -- the exact `(commit, subject,
/// predicate[, object])` a `FactRef` cites as a `consumes` base -- has
/// been retracted, and if so by which commit(s).
///
/// This is the trait-level equivalent of written-world's live, deployed
/// `appview` service (`org.jason-edelman.writtenworld.getResolved`),
/// which already computes exactly this across repos via Jetstream --
/// see `dmml/ARCHITECTURE.md`'s "conflict check reuses `getResolved`"
/// finding. `resolve_fact` exists so a caller can run the same check
/// locally against whichever substrate hosts the world, without a hard
/// runtime dependency on the AppView being reachable; the two are meant
/// to agree, not compete.
///
/// `by` holding more than one CID is the real, checkable signature of
/// a genuine conflict: two commits, unaware of each other, both citing
/// this exact fact as their base. That's the input a `disputes` commit
/// needs -- resolution is never picking a winner, per
/// `dmml/ARCHITECTURE.md`'s "Resolution is `disputes`, not arbitration."
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetractionStatus {
    /// No commit's `consumes` has resolved this fact yet.
    Live,
    /// Retracted by at least one commit. More than one entry in `by`
    /// is the concurrent-base conflict signature above.
    Retracted { by: Vec<String> },
}

/// Result of a compare-and-swap write attempt (`CasSubstrate::
/// commit_with_cas`).
///
/// `RootMoved` is a success-path outcome, not an error: a rejected swap
/// is expected control flow the caller converts into a `disputes`
/// commit, never a failure to propagate or retry blindly. Putting it in
/// an error type would invite exactly that bug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapOutcome {
    /// The commit is published; the substrate's root now points at it.
    Committed(CommitReceipt),
    /// The swap was rejected: between the caller reading the root and
    /// this call, it moved from `expected` to `observed`. Nothing was
    /// written.
    ///
    /// Invariant: `observed` is never `None` here. A rejection means
    /// something else committed first, so the root can only have
    /// *advanced* from what the caller expected (`None` -> some commit,
    /// or one commit -> a later one) -- it never regresses to empty.
    /// `expected: None` is the one legitimate `None`: the caller
    /// believed the repo was still empty and someone beat them to the
    /// first commit. (An impl that somehow observes a genuine
    /// empty-to-empty non-move should return [`SwapOutcome::Committed`],
    /// not this variant -- that's not a rejection, nothing to reject.)
    RootMoved {
        expected: Option<String>,
        observed: String,
    },
}

/// The substrate boundary a concrete backend must satisfy to host a
/// DMML world.
///
/// A `Substrate` is bound at construction to exactly one sovereignty
/// root: one owner identity, one namespace/repo. Every method answers
/// only within that root; an impl must reject anything outside it
/// rather than silently answering a cross-repo query. Both backends
/// already rely on "only ever check one repo" as a real, load-bearing
/// simplification (`dmml/ARCHITECTURE.md`), not an incidental one.
///
/// Deliberately no write method here -- see this module's own doc
/// comment for why a unified write shape would misrepresent at least
/// one backend's real guarantees. A type is writable only via whichever
/// of [`CasSubstrate`]/[`AppendSubstrate`] it separately implements.
pub trait Substrate: Send + Sync + 'static {
    /// The backend's opaque author identity, and wherever the backend
    /// needs it for writes, the credential material that goes with it
    /// (an atproto DID plus its auth session; an iroh `AuthorId` plus
    /// its signing key). Treated as fully opaque here -- handed to a
    /// write method, never inspected -- so neither backend's identity
    /// shape leaks into the other. The DID-to-`AuthorId` *binding*
    /// itself is an ordinary DMML record (`dmml/ARCHITECTURE.md`'s
    /// "Cross-substrate identity" section), not this trait's concern.
    type Identity: Send + Sync + 'static;

    /// Error type for every fallible operation. An expected control-flow
    /// outcome (a rejected CAS) is never an error -- it lives in
    /// [`SwapOutcome`] instead -- so anything surfaced here genuinely
    /// means the operation didn't complete.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Human-readable substrate name, for logging/debugging only.
    fn name(&self) -> &'static str;

    /// The single owner this instance is bound to. All writes are made
    /// as this identity; all reads are scoped to its repo.
    fn owner(&self) -> &Self::Identity;

    /// The single namespace/repo this instance is bound to (an atproto
    /// DID's own repo; an iroh world's `NamespaceId`).
    fn namespace(&self) -> &str;

    /// Fetch a commit by CID. `Ok(None)` means "not in this world's
    /// repo" -- needed to inspect what a `consumes` reference (a
    /// `StrongRef`, or a `FactRef.commit`) actually produced.
    fn get_commit(
        &self,
        cid: &str,
    ) -> impl std::future::Future<Output = Result<Option<CommitRecord>, Self::Error>> + Send;

    /// Resolve whether the specific prior fact `fact` cites has been
    /// retracted, and if so by which commit(s). See [`RetractionStatus`]
    /// for the full contract.
    ///
    /// A genuine read, on the base trait deliberately: any caller
    /// materializing a world's current state, or validating an incoming
    /// commit's citations, needs this regardless of which write
    /// capability the substrate has -- a `CasSubstrate`-hosted world
    /// still has readers who need to know whether a cited fact is live.
    /// It's *additionally* load-bearing, specifically, as
    /// [`AppendSubstrate::append_commit`]'s required pre-write check,
    /// since nothing at that write path can detect a conflict on its
    /// own the way a CAS write detects one at write time -- but that's
    /// one caller of a shared read, not a reason to scope it narrower.
    fn resolve_fact(
        &self,
        fact: &FactRef,
    ) -> impl std::future::Future<Output = Result<RetractionStatus, Self::Error>> + Send;

    /// Every assertion ever produced for `(subject, predicate)`,
    /// including ones later superseded -- see [`Assertion`]'s own doc
    /// comment for why bare `produces` entries all coexist here rather
    /// than being pre-folded to one current value.
    fn assertions(
        &self,
        subject: &NamedNode,
        predicate: &NamedNode,
    ) -> impl std::future::Future<Output = Result<Vec<Assertion>, Self::Error>> + Send;
}

/// Write capability for a substrate whose storage layer gates every
/// commit with a real compare-and-swap (atproto's `swapCommit` against
/// a PDS). The CAS *is* the conflict detector for this shape: two
/// commits unaware of each other cannot both advance the root, because
/// the second swap fails atomically. There is no pre-write check for
/// the caller to run here, and none is wanted -- detection happens at
/// write time, closing the check-then-write race a purely
/// author-partitioned substrate (`AppendSubstrate`) cannot close.
pub trait CasSubstrate: Substrate {
    /// The world's current root commit -- the value a subsequent
    /// `commit_with_cas` call swaps against. `None` means the repo is
    /// empty.
    ///
    /// Deliberately not on `Substrate`: "the world has one swappable
    /// root pointer" is an atproto-shaped claim, and requiring it of
    /// every substrate would be the exact dishonest unification this
    /// trait split exists to avoid.
    fn current_root(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<String>, Self::Error>> + Send;

    /// Publish `commit` atomically, iff the substrate's root is still
    /// `expected_root`. On success the root points at the new commit;
    /// on a moved root, nothing is written and [`SwapOutcome::
    /// RootMoved`] carries both CIDs -- enough for the caller to fetch
    /// the competing commit (`get_commit`) and build a `disputes`
    /// commit, per `dmml/ARCHITECTURE.md`'s "Resolution is `disputes`,
    /// not arbitration." `expected_root: None` asserts the repo is
    /// currently empty.
    fn commit_with_cas(
        &self,
        author: &Self::Identity,
        expected_root: Option<String>,
        commit: &Commit,
    ) -> impl std::future::Future<Output = Result<SwapOutcome, Self::Error>> + Send;
}

/// Write capability for a substrate whose writes are author-partitioned
/// at the storage layer (iroh-docs: entries keyed by `(namespace,
/// author, key)`), so concurrent writers from different authors never
/// collide and **no admission gate exists at the storage layer at all**.
///
/// Because nothing at this layer can detect a conflict, detection is
/// the caller's responsibility: before appending any commit whose
/// `consumes` cites a prior fact, the caller MUST check
/// [`Substrate::resolve_fact`] (or the deployed `getResolved` AppView)
/// and, if the base is already retracted by a different commit, append
/// a `disputes` commit instead of the ordinary one. This trait can't
/// enforce that ordering, but `append_commit`'s signature deliberately
/// takes no `expected` parameter to swap against, so no caller can
/// mistake this for a checked write.
///
/// Known, accepted limitation, inherent to partitioned storage, not
/// specific to this trait: the caller's check-then-append isn't atomic
/// across authors, so two authors can both observe `Live` and both
/// append. The backstop is the checkpoint-to-atproto path, which
/// re-runs the same resolved-status check against the real `getResolved`
/// AppView before the CAS-gated checkpoint commit goes out --
/// `dmml/ARCHITECTURE.md`'s staleness discussion covers why this window
/// is a non-issue outside real-time gaming, and names a content-level
/// (TTL-triple) fix for the one case it isn't, rather than a stronger
/// guarantee here.
pub trait AppendSubstrate: Substrate {
    /// Append `commit` under `author`'s partition. Never fails with a
    /// conflict -- there is none to fail with -- and never touches
    /// another author's entries. Distinct commits from the *same*
    /// author must never overwrite each other either (key by commit
    /// CID, or a monotonic per-author sequence); the concrete keying is
    /// `dmml-substrate-kit`'s decision, but the no-overwrite property is
    /// this trait's contract regardless of how a concrete adapter keys
    /// it.
    fn append_commit(
        &self,
        author: &Self::Identity,
        commit: &Commit,
    ) -> impl std::future::Future<Output = Result<CommitReceipt, Self::Error>> + Send;
}
