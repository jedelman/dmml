//! The committed world, now a single reified graph instead of separate
//! entity/relation/rule stores. A `Delta` is still the only way anything
//! changes — generator, player, demiurge alike — and `propose_and_commit`
//! is still the one gate everything passes through before it's real. What's
//! different from the hand-rolled version: attributes, relations, and rules
//! are no longer three different kinds of fact requiring three different
//! lookup functions — they're all just triples, and `quads_for_pattern` is
//! the one traversal primitive underneath nearly everything below. Real
//! SPARQL, via `oxigraph::sparql`, is reserved for the one query shape here
//! that's a genuine join rather than a single subject's own triples --
//! `query_bound`, used by `render::reachable_adjacent_rooms` to find which
//! rooms are reachable through an unlocked edge. What's actually perceived
//! there is never this query's concern -- that's still `perceive_room`'s
//! own senses-gated field logic, just handed a second subject to run on.
//! Everywhere else, a direct pattern lookup stays more reliable to get
//! right than a hand-built query string, and that's the honest reason for
//! the choice, not a philosophical one.

use std::collections::{HashMap, HashSet};

use oxigraph::model::vocab::xsd;
use oxigraph::model::{
    GraphNameRef, Literal, NamedNode, NamedNodeRef, NamedOrBlankNode, Quad, Term, TermRef,
};
use oxigraph::store::Store;

use crate::vocab;

/// Every graded (bounded-float) attribute's declared range -- the
/// AttrDomain-equivalent this prototype still hardcodes as a table rather
/// than a runtime-extensible domain registry, but a table now, not a
/// wear-only special case. Shared between validation and effect-firing
/// (`game.rs`'s increment clamp) so the two can't drift out of sync.
pub const GRADED_ATTRS: &[(fn() -> NamedNode, f32, f32)] = &[
    (vocab::wear, 0.0, 2.0),
    (vocab::dampness, 0.0, 1.0),
    (vocab::decay, 0.0, 1.0),
    (vocab::light, 0.0, 1.0),
];

pub fn graded_range(p: &NamedNode) -> Option<(f32, f32)> {
    GRADED_ATTRS
        .iter()
        .find(|(f, _, _)| &f() == p)
        .map(|(_, lo, hi)| (*lo, *hi))
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphError {
    Invalid(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::Invalid(s) => write!(f, "{s}"),
        }
    }
}

/// A batch of additions (and, for attribute updates, retractions) that
/// commits atomically: validated against the fixed vocabulary/shape rules
/// before any of it is applied.
#[derive(Default)]
pub struct Delta {
    pub add: Vec<Quad>,
    pub remove: Vec<Quad>,
}

impl Delta {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assert(mut self, s: NamedNode, p: NamedNode, o: impl Into<Term>) -> Self {
        self.add.push(Quad::new(
            s,
            p,
            o.into(),
            oxigraph::model::GraphName::DefaultGraph,
        ));
        self
    }

    pub fn retract(mut self, s: NamedNode, p: NamedNode, o: impl Into<Term>) -> Self {
        self.remove.push(Quad::new(
            s,
            p,
            o.into(),
            oxigraph::model::GraphName::DefaultGraph,
        ));
        self
    }

    /// The inverse of `TranscriptEntry::canonical_text`: parses the same
    /// `+ <nquad>` / `- <nquad>` lines back into a `Delta`. Exists for the
    /// pantheon work's PDS-as-source-of-truth arc
    /// ([jedelman/written-world#8](https://github.com/jedelman/written-world/issues/8)):
    /// once a commit record carries this text (not just its hash, as
    /// today), a fresh client can replay a player's own commit log to
    /// reconstruct their graph from nothing but their PDS -- this is what
    /// makes that replay possible. Each line is already standalone valid
    /// N-Quads (`Quad::to_string()`'s own output), so parsing reuses the
    /// identical `Store::load_from_slice` path `load_nquads` already
    /// trusts, rather than a second hand-rolled N-Quads reader -- one
    /// scratch in-memory store per side (add/remove can't share one, or
    /// they'd collapse into a single set and lose which line was which),
    /// discarded once its quads are read back out.
    pub fn from_canonical_text(text: &str) -> Result<Self, GraphError> {
        let mut add_lines = Vec::new();
        let mut remove_lines = Vec::new();
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("+ ") else {
                if let Some(rest) = line.strip_prefix("- ") {
                    remove_lines.push(rest);
                    continue;
                }
                if line.trim().is_empty() {
                    continue;
                }
                return Err(GraphError::Invalid(format!(
                    "malformed canonical-text line (must start with '+ ' or '- '): {line:?}"
                )));
            };
            add_lines.push(rest);
        }

        let parse_side = |lines: &[&str]| -> Result<Vec<Quad>, GraphError> {
            if lines.is_empty() {
                return Ok(Vec::new());
            }
            let store = Store::new().expect("in-memory store always constructs");
            // `Quad::to_string()` (what `canonical_text` renders each line
            // with) omits the terminating `.` the N-Quads grammar requires
            // per statement -- it's a bare Display impl, not a strict
            // serializer. Restore it before parsing; `load_from_slice`
            // rejects a statement missing one outright ("Quads must be
            // followed by a dot"), confirmed by this function's own test
            // failing before this line existed.
            let blob = lines
                .iter()
                .map(|l| format!("{l} ."))
                .collect::<Vec<_>>()
                .join("\n");
            store
                .load_from_slice(oxigraph::io::RdfFormat::NQuads, blob.as_bytes())
                .map_err(|e| GraphError::Invalid(format!("malformed canonical-text N-Quads: {e}")))?;
            Ok(store
                .quads_for_pattern(None, None, None, None)
                .filter_map(|q| q.ok())
                .collect())
        };

        Ok(Delta {
            add: parse_side(&add_lines)?,
            remove: parse_side(&remove_lines)?,
        })
    }
}

/// A strong reference to an atproto record: its `at://` URI plus the CID it
/// was observed at. Mirrors `com.atproto.repo.strongRef`, used by the
/// lexicon's `consumes`/`via`/`respondsTo` fields -- see
/// `lexicons/org/jason-edelman/writtenworld/commit.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrongRef {
    pub uri: String,
    pub cid: String,
}

/// One entry in a commit's `consumes` list: either a whole-record
/// `StrongRef` (unchanged -- foreign nodes, Bridge halves, Pentacle
/// grants) or a `FactRef` naming one triple within an already-addressable
/// commit's own `produces`, for same-repo, triple-granularity
/// retraction/supersession. See `SPEC.md` and the
/// lexicon's own `consumes`/`#factRef` doc comments
/// (`lexicons/org/jason-edelman/writtenworld/commit.json`) --
/// `consumes`'s item type is a union of these two exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumeRef {
    Strong(StrongRef),
    Fact(FactRef),
}

impl ConsumeRef {
    /// `Some(&StrongRef)` iff this is a whole-record reference -- the one
    /// case every call site that predates `FactRef` (Bridge acceptance,
    /// `consume_state`'s node-currency bookkeeping) still only ever cares
    /// about.
    pub fn as_strong(&self) -> Option<&StrongRef> {
        match self {
            ConsumeRef::Strong(r) => Some(r),
            ConsumeRef::Fact(_) => None,
        }
    }
}

/// Names one `(subject, predicate[, object])` triple within an already-
/// addressable commit's own `produces` -- `SPEC.md`'s
/// mechanism for same-repo, triple-grained retraction, finer than a whole-
/// record `StrongRef`. `subject` is the durable `at://` URI a node minted
/// by `commit` is identified by going forward (the same node-identity
/// convention `getResolved`'s own recursion already depends on -- see that
/// spec's "Node identity across commits" section), `predicate` is the raw
/// predicate IRI as it appeared in `commit`'s own `produces`. `object`
/// disambiguates when `commit`'s `produces` asserted more than one triple
/// for `(subject, predicate)`; omitted means every triple `commit`
/// asserted for that pair -- wildcard semantics, settled per `SPEC.md`
/// section 5 (the lexicon's own `object` field doc already commits to
/// that reading, so honoring it here is the consistent choice, not a
/// separately-argued one).
///
/// Same-repo scope (`SPEC.md` section 6): `commit` must be in the same repo as
/// whatever commit carries this `FactRef` -- enforced write-time,
/// best-effort, in `server/src/atproto/commit_write.rs`, and resolve-time,
/// authoritatively, in `appview`. Not enforced by this struct itself, or
/// by `WorldGraph::apply_commit` below, which has no notion of "repo" at
/// all (its `source` parameter is a free-text label like `"demiurge"`, not
/// a DID) -- see `apply_commit`'s own doc comment for the narrower,
/// best-effort-only guard it runs instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactRef {
    pub commit: StrongRef,
    pub subject: String,
    pub predicate: String,
    pub object: Option<String>,
}

/// One `apply_commit` call's outcome, kept only to answer
/// `WorldGraph::consume_state`'s supersession query -- not a general
/// audit log (`TranscriptEntry`, recorded for *every* commit regardless of
/// path, already is that). Distinct from `TranscriptEntry` deliberately:
/// this one is indexed by the `consumes`/`produces` node identities a
/// `Commit` actually reasons about, not by raw added/removed quads.
///
/// Not folded into `WorldGraph::dump_nquads`/`load_nquads` itself -- it
/// isn't RDF data belonging in the store's own dump, for the same reason
/// `transcript` isn't either (see that field's own doc comment). Unlike
/// `transcript`, though, this one *is* persistable and restorable: it's
/// exactly the ordering `current_value`/`current_subjects_with` need to
/// answer "what's current" at all, so losing it silently blanks every
/// `apply_commit`-sourced fact (`heldBy`, `locatedIn`) the instant a
/// session reloads from a snapshot -- confirmed by reproduction; see
/// `dump_commit_log`/`restore_commit_log`, the save/load pair a caller
/// (`Game::snapshot`/`Game::from_snapshot`) uses alongside `dump_nquads`/
/// `load_nquads` to carry this across a reload too.
#[derive(Debug, Clone)]
struct CommitRecord {
    /// This commit's position in `WorldGraph::transcript` -- what
    /// `ConsumeState::Retracted::superseded_by` reports back.
    seq: u64,
    /// Every `consumes` reference, resolved to the same node identity
    /// `apply_commit`'s existence check resolves it to
    /// (`vocab::foreign_uri_node(&r.uri)`).
    consumes: Vec<NamedNode>,
    /// Every `NamedNode` subject this commit's `produces` asserted at
    /// least one triple about. A node in `consumes` that reappears here
    /// was updated in place, not superseded -- see `consume_state`.
    produced_subjects: HashSet<NamedNode>,
    /// The actual triples this commit's `produces` asserted -- i.e. the
    /// `quads` parsed from `Commit::produces`, before `apply_commit` folds
    /// in its own `via`/`respondsTo` provenance triples. What
    /// `current_value`/`current_subjects_with` walk to answer "what's the
    /// latest value this specific predicate on this specific node was
    /// asserted to have" -- `produced_subjects` alone only knows *which*
    /// nodes were touched, not *what* was asserted about them, which is
    /// exactly what a per-predicate query needs.
    produced: Vec<Quad>,
}

/// The answer to "is this `consumes`-addressable node still current, or
/// has some commit superseded it?" -- see `WorldGraph::consume_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeState {
    /// No `apply_commit` call has ever consumed this node, or every call
    /// that did also re-asserted it in the same commit's `produces` (an
    /// in-place update, not a supersession).
    Current,
    /// Some commit consumed this node without re-asserting it. `seq` is
    /// that commit's position in the transcript -- render.rs (or whatever
    /// wants the successor state) can read `TranscriptEntry::added` at
    /// that seq to see what replaced it.
    Retracted { seq: u64 },
}

/// The lexicon's one record shape (`org.jason-edelman.writtenworld.commit`):
/// a single-authority production event over the world graph, differentiated
/// only by which vocabulary populates `predicate`/`produces` -- minting a
/// node, changing an attribute, retracting a fact (empty `produces`),
/// declaring a Type, granting a delegation are all this same shape. See the
/// lexicon file for the full field semantics. This is a new, PDS-record-
/// shaped path added alongside the existing hand-built `Delta`/`commit`
/// path below -- it does not replace it.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Nodes or single facts this commit consumes. Empty for a mint. See
    /// `ConsumeRef`'s own doc comment for the two reference kinds this can
    /// hold.
    pub consumes: Vec<ConsumeRef>,
    /// Subgraph this commit produces, serialized as N-Quads. Empty for a
    /// pure retraction.
    pub produces: String,
    /// Open-ontology verb naming this commit's operation -- e.g. `mints`,
    /// `becomes`, `divides`, `grants`. Not validated against a closed enum;
    /// the lexicon deliberately leaves this vocabulary open.
    pub predicate: String,
    /// Optional provenance: the Theos-operation or Pentacle grant that
    /// authorized this commit.
    pub via: Option<StrongRef>,
    /// Set only on the accepting half of a cross-repo Bridge.
    pub responds_to: Option<StrongRef>,
    pub created_at: String,
}

/// Parses `text` as standard N-Quads (oxigraph's own parser, via a scratch
/// in-memory store -- the same trick `Delta::from_canonical_text`'s
/// `parse_side` uses to get a real N-Quads reader without hand-rolling one)
/// into a flat `Vec<Quad>`. Unlike `canonical_text`'s `+ `/`- `-prefixed
/// lines, `Commit::produces` is expected to already be well-formed N-Quads
/// text straight from the lexicon (each line already dot-terminated), so no
/// line-prefix stripping or dot-repair happens here. Empty/whitespace-only
/// text parses to an empty vec, matching the lexicon's "empty produces = a
/// pure retraction" case.
fn parse_nquads(text: &str) -> Result<Vec<Quad>, GraphError> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let store = Store::new().expect("in-memory store always constructs");
    store
        .load_from_slice(oxigraph::io::RdfFormat::NQuads, text.as_bytes())
        .map_err(|e| GraphError::Invalid(format!("malformed produces N-Quads: {e}")))?;
    Ok(store
        .quads_for_pattern(None, None, None, None)
        .filter_map(|q| q.ok())
        .collect())
}

/// One commit, in order: who proposed it and what it changed. This is the
/// literal transcript of the demiurge's (and everyone else's) activity --
/// every piece of world content traces back to exactly one entry here,
/// which is the whole point of routing everything through one gate.
#[derive(Debug, Clone)]
pub struct TranscriptEntry {
    pub seq: u64,
    pub source: String,
    pub added: Vec<Quad>,
    pub removed: Vec<Quad>,
    /// ms-epoch, supplied by the caller at `commit()` time (see
    /// `WorldGraph::set_now`) -- what `creation_order` actually reads.
    /// Not part of `canonical_text`/its hash (see that method's own
    /// comment for why: wall-clock time says nothing about *what* was
    /// committed, only *when*, and two independently-reproducible commits
    /// of identical content shouldn't hash differently just because a
    /// clock disagreed about the moment).
    pub timestamp_ms: u64,
}

impl TranscriptEntry {
    /// A deterministic text rendering of what this commit changed: every
    /// added/removed quad's N-Quads form (oxigraph's own `Quad::to_string`),
    /// sorted so two commits touching the same triples produce identical
    /// text regardless of the order `Delta` happened to list them in. A
    /// caller that wants a stable, content-addressed id for a commit (see
    /// the atproto commit-signing flow this exists for) hashes this rather
    /// than hashing `seq` or wall-clock time, neither of which says
    /// anything about what was actually committed.
    pub fn canonical_text(&self) -> String {
        let mut added: Vec<String> = self.added.iter().map(|q| q.to_string()).collect();
        let mut removed: Vec<String> = self.removed.iter().map(|q| q.to_string()).collect();
        added.sort();
        removed.sort();
        let mut text = String::new();
        for line in &added {
            text.push_str("+ ");
            text.push_str(line);
            text.push('\n');
        }
        for line in &removed {
            text.push_str("- ");
            text.push_str(line);
            text.push('\n');
        }
        text
    }
}

pub struct WorldGraph {
    store: Store,
    /// A running, order-sensitive hash of everything committed so far --
    /// `commit()` folds each entry's `canonical_text()` (plus `source`)
    /// into it. Replaces a plain incrementing counter specifically so
    /// fresh-id minting survives commit-log replay
    /// ([jedelman/written-world#8](https://github.com/jedelman/written-world/issues/8)):
    /// replaying a player's own PDS commit log runs the identical ordered
    /// sequence of `commit()` calls this field already accumulates through
    /// live, so a replayed graph's `content_hash` naturally lands on the
    /// same value the original session would have had at that point --
    /// no separate counter needs to be carried alongside the replayed
    /// records at all. (A flat `dump_nquads()` snapshot is a different
    /// case: order is lost, so `load_nquads` still needs this handed back
    /// explicitly -- see its own doc comment.)
    content_hash: u64,
    /// Purely a same-process disambiguator for multiple `fresh()` calls
    /// within one not-yet-committed `Delta` (`content_hash` only advances
    /// *after* a commit lands, but bootstrap/generate_frontier routinely
    /// mint several nodes before their first commit). Never needs to
    /// match across processes or survive replay -- replay never calls
    /// `fresh()` at all, it only re-asserts already-concrete ids parsed
    /// from `canonical_text`.
    mint_counter: u64,
    /// ms-epoch the caller last handed us via `set_now` -- stamped onto
    /// every `TranscriptEntry` a subsequent `commit()` produces (see
    /// `TranscriptEntry::timestamp_ms`'s own doc comment for why this is
    /// what `creation_order` reads instead of anything derived from a
    /// minted id). `engine` has no clock of its own -- see this crate's
    /// "no I/O" rule, already the reason `commune::raise_petition` takes
    /// `now_ms` as an explicit parameter rather than reading one itself --
    /// so this starts at `0` and stays there until a caller sets it.
    /// Replay reconstructs identical `creation_order` results because a
    /// replayed session calls `set_now`/`commit` with the same recorded
    /// timestamps the original commits carried, in the same order.
    now_ms: u64,
    transcript: Vec<TranscriptEntry>,
    /// `apply_commit`-only bookkeeping for `consume_state` -- see
    /// `CommitRecord`'s own doc comment for why this is separate from
    /// `transcript`.
    commit_log: Vec<CommitRecord>,
}

impl Default for WorldGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldGraph {
    pub fn new() -> Self {
        WorldGraph {
            store: Store::new().expect("in-memory store always constructs"),
            // The same golden-ratio mixing constant `Rng::new` seeds with
            // -- not load-bearing which constant, just a fixed, non-zero
            // genesis value every fresh world starts from.
            content_hash: 0x9E3779B97F4A7C15,
            mint_counter: 0,
            now_ms: 0,
            transcript: Vec::new(),
            commit_log: Vec::new(),
        }
    }

    /// Reconstructs a graph from a `dump_nquads()` snapshot -- the other
    /// half of that primitive. `content_hash` has to travel alongside the
    /// dump (it isn't part of the RDF data, and a flat dump has already
    /// lost the commit ordering `content_hash` is derived from, so it
    /// can't be recomputed from the triples themselves) or freshly-minted
    /// nodes after reload could collide with ones already committed
    /// before the snapshot was taken. The transcript is deliberately NOT
    /// restored -- it's an audit/introspection log, not state gameplay
    /// depends on, and starting it empty on reload is a real, accepted
    /// limitation rather than an oversight; carrying it across restarts
    /// would mean persisting and replaying the full commit history, which
    /// this snapshot-based approach doesn't attempt (see
    /// `from_canonical_text`/the commit-log replay path for where that's
    /// actually built instead).
    pub fn load_nquads(content_hash: u64, bytes: &[u8]) -> Result<Self, GraphError> {
        let store = Store::new().expect("in-memory store always constructs");
        store
            .load_from_slice(oxigraph::io::RdfFormat::NQuads, bytes)
            .map_err(|e| GraphError::Invalid(e.to_string()))?;
        Ok(WorldGraph {
            store,
            content_hash,
            mint_counter: 0,
            now_ms: 0,
            transcript: Vec::new(),
            commit_log: Vec::new(),
        })
    }

    /// The value `load_nquads` needs to be handed back on reload -- see
    /// its doc comment for why this can't be recovered from the triples
    /// themselves.
    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    /// Sets the wall-clock time (ms-epoch) `commit()` stamps onto the
    /// `TranscriptEntry` it produces next. A caller invokes this once
    /// before whatever minting/committing it's about to do -- see
    /// `Game::set_now` for the one real entry point every native/wasm
    /// caller (server, cli, client) actually calls before a mutating
    /// operation, and `now_ms`'s own doc comment for why `engine` doesn't
    /// read a clock itself.
    pub fn set_now(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// The ms-epoch reading `set_now` last stamped -- what a caller
    /// building a `Commit`'s own `created_at` (a real-world timestamp
    /// string, not this crate's business to format -- see `now_ms`'s own
    /// doc comment) reads back when it isn't threading its own explicit
    /// `now_ms` parameter through, e.g. a verb reached via `Game::handle`,
    /// which relies on the caller having already called `Game::set_now`.
    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }

    /// A collision-avoiding hash of `content_hash` and `mint_counter` --
    /// mixed into every prior call this commit (`mint_counter` advances
    /// each call, resets to `0` on `commit()`, see its own doc comment),
    /// so several mints within one not-yet-committed `Delta` land on
    /// different ids even though they share the same `content_hash`.
    /// Reconstructed identically by replay for the same reason
    /// `content_hash` itself is: replay runs the same ordered `commit()`
    /// calls, so `mint_counter` resets in lockstep. Unlike the older
    /// scheme this replaced, nothing about creation order is packed in
    /// here anymore -- that's now `creation_order`'s job, reading
    /// `TranscriptEntry::timestamp_ms` instead of anything derived from a
    /// minted id (see that function's own doc comment).
    fn next_mint_id(&mut self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.content_hash.hash(&mut h);
        self.mint_counter.hash(&mut h);
        self.mint_counter += 1;
        h.finish()
    }

    pub fn fresh(&mut self, prefix: &str) -> NamedNode {
        let id = self.next_mint_id();
        vocab::fresh(prefix, id)
    }

    pub fn fresh_class(&mut self) -> NamedNode {
        let id = self.next_mint_id();
        vocab::class_iri(id)
    }

    /// The only way `store` is ever mutated. Validates the whole batch
    /// against the current state first; on any failure nothing is applied.
    /// `source` names who's proposing -- "demiurge", "player", eventually
    /// an NPC's own id -- and is recorded in the transcript, not checked;
    /// nothing here grants any source special trust, the validator is the
    /// same gate regardless of who's asking.
    pub fn commit(&mut self, source: &str, delta: Delta) -> Result<(), GraphError> {
        validate(&self.store, &delta)?;
        for q in &delta.remove {
            self.store.remove(q).expect("in-memory store never errors");
        }
        for q in &delta.add {
            self.store.insert(q).expect("in-memory store never errors");
        }
        let seq = self.transcript.len() as u64;
        let entry = TranscriptEntry {
            seq,
            source: source.to_string(),
            added: delta.add,
            removed: delta.remove,
            timestamp_ms: self.now_ms,
        };

        // Folds this commit into the running content hash -- see
        // `content_hash`'s own doc comment for why this is what makes
        // fresh-id minting survive commit-log replay: replaying applies
        // this exact same fold, in the same order, over the same text.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.content_hash.hash(&mut h);
        source.hash(&mut h);
        entry.canonical_text().hash(&mut h);
        self.content_hash = h.finish();
        // Resets the within-delta mint disambiguator too -- see
        // `next_mint_id`'s doc comment. Without this, two graphs that
        // reach equal `content_hash` by different total mint-call counts
        // (a live session's own history vs. that same point reached via
        // replay, which never calls `fresh()` at all) would mint
        // different-but-equally-valid next ids instead of the identical
        // one a matching `content_hash` implies they should.
        self.mint_counter = 0;

        self.transcript.push(entry);
        Ok(())
    }

    /// Applies a lexicon-shaped `Commit` (see `Commit`'s own doc comment)
    /// to this graph: validates that every `consumes` reference is at least
    /// known to the graph, parses `produces` as N-Quads, and inserts the
    /// result plus (if present) `via`/`respondsTo` provenance triples --
    /// same transcript/content-hash bookkeeping `commit()` does, just fed
    /// from a `Commit` record instead of a hand-built `Delta`.
    ///
    /// ## Consume/produce semantics ("no deletions, only a record that
    /// consumption happened")
    ///
    /// `consumes` never deletes anything from the store. A `Commit` is a
    /// production event, not an edit: what actually happened to a
    /// `consumes`-referenced node is *recorded*, by this call landing in
    /// `WorldGraph::commit_log`, not enacted by removing its triples. Two
    /// things fall out of that:
    ///
    /// - **Referential guard.** Every `consumes` reference must already be
    ///   known to the graph (appear as a subject or object of some triple,
    ///   addressed via `vocab::foreign_uri_node(&r.uri)`) -- this only
    ///   rejects consuming something the graph has never heard of, it does
    ///   not remove it.
    /// - **Current-vs-retracted query.** `consume_state`/`is_retracted`
    ///   answer "is this node still current?" by looking at whether *this*
    ///   commit's `produces` re-asserts the same node (i.e. still has a
    ///   triple with that node as subject). If it does, this was an
    ///   in-place update (an attribute-change commit consuming the fact it
    ///   supersedes, then re-stating the node with a new value) and the
    ///   node stays current. If it doesn't -- including the pure-retraction
    ///   case, empty `produces` -- the node is retracted/superseded as of
    ///   this commit: still physically present in the store (nothing here
    ///   ever calls `store.remove`), but callers that care about current
    ///   state should treat it as gone and, when `produces` was non-empty,
    ///   look at what *did* land in this commit's `produces` as its
    ///   successor. This is a monotonic, one-way state: once retracted by
    ///   some commit, a node stays retracted (a later commit re-consuming
    ///   it without re-asserting it is redundant, not a resurrection --
    ///   this design has no "undo a retraction" operation).
    ///
    /// Still doesn't run this module's `validate`: `validate` rejects
    /// blank-node subjects outright, but the lexicon's own `produces` doc
    /// explicitly allows blank nodes for intra-commit references (a minted
    /// room referencing a minted door produced in the same commit), so the
    /// closed-vocabulary shape checker above isn't the right gate for this
    /// path. `commune`/`demiurge`/hand-built `Delta`s remain the validated
    /// path; a `Commit`'s own guard is the `consumes` existence check plus
    /// (for a real caller) whatever invariants that caller's own domain
    /// logic upholds -- see `Game::conjure` for the one wired in so far.
    ///
    /// ## `FactRef` entries (`SPEC.md`)
    ///
    /// A `ConsumeRef::Fact` entry gets a narrower, best-effort version of
    /// the same referential guard, at triple granularity instead of node
    /// granularity: does *some* triple matching `(subject, predicate[,
    /// object])` currently exist anywhere in the store? This in-memory
    /// graph has no notion of "which specific commit produced which
    /// triple" the way `appview`'s URI-indexed commit log does (it never
    /// learns its own commit's `at://` identity at `apply_commit` time),
    /// so unlike `server/src/atproto/commit_write.rs`'s write-time check
    /// and `appview`'s resolve-time check, this can't verify that `fr.
    /// commit` *specifically* asserted the referenced triple, and doesn't
    /// enforce same-repo scope either (no DID is available here at all --
    /// see `FactRef`'s own doc comment). A `FactRef` entry is deliberately
    /// **not** added to `consumed_nodes`/`CommitRecord.consumes`: that
    /// bookkeeping is `consume_state`'s whole-node currency question,
    /// which a triple-level retraction doesn't answer (and per the spec,
    /// `FactRef` never marks a node itself as retracted -- only
    /// `getResolved`'s resolution-time filtering acts on it at all).
    pub fn apply_commit(&mut self, source: &str, commit: Commit) -> Result<(), GraphError> {
        let mut consumed_nodes = Vec::with_capacity(commit.consumes.len());
        for r in &commit.consumes {
            match r {
                ConsumeRef::Strong(r) => {
                    let node = vocab::foreign_uri_node(&r.uri);
                    let as_subject = self
                        .store
                        .quads_for_pattern(
                            Some(NamedOrBlankNode::from(node.clone()).as_ref()),
                            None,
                            None,
                            Some(GraphNameRef::DefaultGraph),
                        )
                        .next()
                        .is_some();
                    let object_term = Term::NamedNode(node.clone());
                    let as_object = self
                        .store
                        .quads_for_pattern(
                            None,
                            None,
                            Some(TermRef::from(&object_term)),
                            Some(GraphNameRef::DefaultGraph),
                        )
                        .next()
                        .is_some();
                    if !as_subject && !as_object {
                        return Err(GraphError::Invalid(format!(
                            "consumes references unknown node: {}",
                            r.uri
                        )));
                    }

                    // Issue #53: check the `cid` half, not just `uri`.
                    // `foreignCid` facts are the only record this graph
                    // ever keeps of a genuinely observed cid for a node
                    // (asserted by `via`/`respondsTo` below) -- existence,
                    // not currency (`SPEC.md` section 11): a cid matching
                    // ANY prior observation is accepted, not just the most
                    // recent one, since an older-but-real reference is a
                    // true statement about what it actually referenced,
                    // not a stale error. A node with no recorded cid yet
                    // has nothing to check against, so it's accepted the
                    // same as before this check existed -- #53 asks for
                    // "was this cid genuinely ever recorded," not "every
                    // node must always carry one."
                    let mut recorded_cids: Vec<String> = self
                        .store
                        .quads_for_pattern(
                            Some(NamedOrBlankNode::from(node.clone()).as_ref()),
                            Some(NamedNodeRef::from(&vocab::foreign_cid())),
                            None,
                            Some(GraphNameRef::DefaultGraph),
                        )
                        .flatten()
                        .filter_map(|q| match q.object {
                            Term::NamedNode(n) => vocab::foreign_cid_from_node(&n),
                            _ => None,
                        })
                        .collect();
                    // This SAME commit's own `via`/`respondsTo` (asserted
                    // further down, after this loop) also counts as a
                    // genuine observation, even though it hasn't landed in
                    // the store yet at this point in the function -- a
                    // commit that observes a node's new cid (`respondsTo`)
                    // and, in the same breath, acts on that new cid
                    // (`consumes`) is self-consistent, not a forward
                    // reference to a fact that doesn't exist yet. Without
                    // this, whether that legitimate pattern is accepted
                    // would depend on this loop running before the
                    // via/respondsTo block below -- an implementation
                    // detail, not something a caller should have to know
                    // or a future refactor should be free to break.
                    if let Some(via) = &commit.via {
                        if via.uri == r.uri {
                            recorded_cids.push(via.cid.clone());
                        }
                    }
                    if let Some(responds_to) = &commit.responds_to {
                        if responds_to.uri == r.uri {
                            recorded_cids.push(responds_to.cid.clone());
                        }
                    }
                    if !recorded_cids.is_empty() && !recorded_cids.iter().any(|c| c == &r.cid) {
                        return Err(GraphError::Invalid(format!(
                            "consumes cid does not match any cid ever recorded for {}: got {}, recorded {:?}",
                            r.uri, r.cid, recorded_cids
                        )));
                    }

                    consumed_nodes.push(node);
                }
                ConsumeRef::Fact(fr) => {
                    let subject_node = vocab::foreign_uri_node(&fr.subject);
                    let predicate_node = NamedNode::new(&fr.predicate).map_err(|e| {
                        GraphError::Invalid(format!(
                            "factRef predicate is not a valid IRI: {}: {e}",
                            fr.predicate
                        ))
                    })?;
                    let matches = self
                        .store
                        .quads_for_pattern(
                            Some(NamedOrBlankNode::from(subject_node.clone()).as_ref()),
                            Some(NamedNodeRef::from(&predicate_node)),
                            None,
                            Some(GraphNameRef::DefaultGraph),
                        )
                        .flatten()
                        .any(|q| match &fr.object {
                            None => true,
                            Some(expected) => term_matches_fact_object(&q.object, expected),
                        });
                    if !matches {
                        return Err(GraphError::Invalid(format!(
                            "consumes references unknown fact: ({}, {}, {:?})",
                            fr.subject, fr.predicate, fr.object
                        )));
                    }
                }
            }
        }

        let quads = parse_nquads(&commit.produces)?;

        let produced_subjects: HashSet<NamedNode> = quads
            .iter()
            .filter_map(|q| q.subject_named())
            .collect();
        // Kept for `commit_log` (see `CommitRecord::produced`'s own doc
        // comment) separately from `added` below, which goes on to be
        // extended with `via`/`respondsTo` provenance triples that aren't
        // part of what this commit actually *produced* in the lexicon
        // sense -- materialization should never mistake a `Commit`-node's
        // own bookkeeping triple for a fact about the caller's subject.
        let produced_for_log = quads.clone();

        // `via`/`respondsTo` provenance: only minted at all when at least
        // one is present, onto a fresh `Commit`-typed node -- see
        // `vocab::class_commit`'s own doc comment for why both point at
        // the same `foreign_uri_node` addressing `consumes` uses, and why
        // the referenced node's own `foreignCid` fact is asserted too.
        let mut extra_quads = Vec::new();
        if commit.via.is_some() || commit.responds_to.is_some() {
            let commit_node = self.fresh("commit/");
            extra_quads.push(Quad::new(
                commit_node.clone(),
                vocab::rdf_type(),
                Term::NamedNode(vocab::class_commit()),
                oxigraph::model::GraphName::DefaultGraph,
            ));
            extra_quads.push(Quad::new(
                commit_node.clone(),
                vocab::commit_predicate(),
                Term::Literal(lit_str(commit.predicate.clone())),
                oxigraph::model::GraphName::DefaultGraph,
            ));
            if let Some(via) = &commit.via {
                let via_node = vocab::foreign_uri_node(&via.uri);
                extra_quads.push(Quad::new(
                    commit_node.clone(),
                    vocab::via(),
                    Term::NamedNode(via_node.clone()),
                    oxigraph::model::GraphName::DefaultGraph,
                ));
                extra_quads.push(Quad::new(
                    via_node,
                    vocab::foreign_cid(),
                    Term::NamedNode(vocab::foreign_cid_node(&via.cid)),
                    oxigraph::model::GraphName::DefaultGraph,
                ));
            }
            if let Some(responds_to) = &commit.responds_to {
                let responds_node = vocab::foreign_uri_node(&responds_to.uri);
                extra_quads.push(Quad::new(
                    commit_node,
                    vocab::responds_to(),
                    Term::NamedNode(responds_node.clone()),
                    oxigraph::model::GraphName::DefaultGraph,
                ));
                extra_quads.push(Quad::new(
                    responds_node,
                    vocab::foreign_cid(),
                    Term::NamedNode(vocab::foreign_cid_node(&responds_to.cid)),
                    oxigraph::model::GraphName::DefaultGraph,
                ));
            }
        }

        let mut added = quads;
        added.extend(extra_quads);

        for q in &added {
            self.store.insert(q).expect("in-memory store never errors");
        }

        let seq = self.transcript.len() as u64;
        let entry = TranscriptEntry {
            seq,
            source: source.to_string(),
            added,
            removed: Vec::new(),
            timestamp_ms: self.now_ms,
        };

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.content_hash.hash(&mut h);
        source.hash(&mut h);
        entry.canonical_text().hash(&mut h);
        self.content_hash = h.finish();
        self.mint_counter = 0;

        self.commit_log.push(CommitRecord {
            seq,
            consumes: consumed_nodes,
            produced_subjects,
            produced: produced_for_log,
        });
        self.transcript.push(entry);
        Ok(())
    }

    /// Whether `uri` -- the same address space a `StrongRef.uri` lives in,
    /// resolved via `vocab::foreign_uri_node` -- is still current or has
    /// been superseded by some `apply_commit` call's `consumes`. See
    /// `apply_commit`'s own doc comment ("Consume/produce semantics") for
    /// the exact rule this implements, and `ConsumeState` for what's
    /// returned.
    pub fn consume_state(&self, uri: &str) -> ConsumeState {
        let node = vocab::foreign_uri_node(uri);
        for record in &self.commit_log {
            if record.consumes.contains(&node) && !record.produced_subjects.contains(&node) {
                return ConsumeState::Retracted { seq: record.seq };
            }
        }
        ConsumeState::Current
    }

    /// `true` iff `consume_state(uri)` is `Retracted` -- the common case a
    /// caller just wants a bool for.
    pub fn is_retracted(&self, uri: &str) -> bool {
        matches!(self.consume_state(uri), ConsumeState::Retracted { .. })
    }

    // -- Per-predicate materialization ---------------------------------
    //
    // `consume_state`/`is_retracted` answer "is this whole node still
    // current" for a `consumes`-addressable (`StrongRef`/`foreign_uri_node`)
    // reference -- the granularity a cross-repo/atproto consume/produce
    // event reasons about. A stateful verb needs a finer question: not
    // "is the *node* still current" but "what is the *current value of
    // this one predicate* on it right now" -- e.g. which room currently
    // contains a given item. `apply_commit` never deletes (see its own
    // doc comment, "no deletions, only a record that consumption
    // happened"), so once some predicate on a node is re-asserted by a
    // second `apply_commit` call, the store physically holds every prior
    // generation of that fact side by side with the newest one, and a
    // plain pattern lookup (`object`/`objects`) can't tell which one is
    // current -- the in-memory store's own iteration order is keyed by
    // encoded term id, not insertion order (same reason `creation_order`
    // can't read timestamps off the store either). Only `commit_log`'s
    // ordering can answer that, which is what the two queries below walk.
    //
    // Deliberately keyed by `&NamedNode`, not a `consumes`-style `&str`
    // URI resolved through `foreign_uri_node`: the nodes a stateful verb
    // re-asserts a predicate on (a locally minted `Item`, the one
    // `Player` node) are ordinary graph identities, the same kind every
    // other query helper in this module (`objects`, `object`, `subjects`,
    // ...) already takes -- they were never `consumes`-addressed
    // `StrongRef`s to begin with, only a real cross-repo/atproto reference
    // is. Forcing them through `foreign_uri_node` just to match
    // `consume_state`'s signature would be encoding a fact about them
    // that isn't true.

    /// The value of `predicate` on `subject` as of the most recent
    /// `apply_commit` call whose `produces` asserted a `(subject,
    /// predicate, _)` triple -- "later wins", where "later" means later in
    /// `commit_log`'s own order, not whatever order the store's pattern
    /// iteration happens to return. `None` if no `apply_commit` call ever
    /// asserted this predicate for this subject (never touched, as
    /// distinct from "touched and then explicitly cleared" -- a caller
    /// wanting that distinction needs its own sentinel value, the same way
    /// `vocab::held_by`/`vocab::nobody` do for "dropped").
    ///
    /// Only sees state produced via `apply_commit` -- a fact asserted
    /// purely through the older `Delta`/`commit` path (still how most of
    /// this crate's own state lives) never lands in `commit_log` at all,
    /// so this simply won't find it. That's the accepted boundary of this
    /// prototype's migration, not a bug: see `Game::take`/`Game::drop`'s
    /// own doc comments for which predicates actually moved onto this
    /// path and why the rest deliberately didn't yet.
    pub fn current_value(&self, subject: &NamedNode, predicate: &NamedNode) -> Option<Term> {
        self.commit_log.iter().rev().find_map(|record| {
            record.produced.iter().rev().find_map(|q| {
                if q.subject_named().as_ref() == Some(subject) && &q.predicate == predicate {
                    Some(q.object.clone())
                } else {
                    None
                }
            })
        })
    }

    /// The reverse-indexed counterpart to `current_value`: every subject
    /// whose *current* value of `predicate` equals `value`, for callers
    /// that want "who/what currently holds this value" rather than
    /// starting from a known subject (e.g. "which items does the player
    /// currently hold", where the player is the *value*, not the
    /// subject -- `heldBy` runs item -> holder). Built by folding
    /// `commit_log` oldest-to-newest into a `subject -> latest object` map
    /// scoped to `predicate` alone (equivalent to calling `current_value`
    /// for every subject that ever had `predicate` produced, just without
    /// re-walking the log once per candidate), then filtering by `value`.
    /// Sorted by IRI so callers get a deterministic order -- the
    /// intermediate map isn't insertion-ordered, and neither is the
    /// store's own iteration this replaces.
    pub fn current_subjects_with(&self, predicate: &NamedNode, value: &Term) -> Vec<NamedNode> {
        let mut latest: HashMap<NamedNode, Term> = HashMap::new();
        for record in &self.commit_log {
            for q in &record.produced {
                if &q.predicate == predicate {
                    if let Some(s) = q.subject_named() {
                        latest.insert(s, q.object.clone());
                    }
                }
            }
        }
        let mut out: Vec<NamedNode> = latest
            .into_iter()
            .filter(|(_, v)| v == value)
            .map(|(s, _)| s)
            .collect();
        out.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        out
    }

    // -- Query helpers -----------------------------------------------

    pub fn objects(&self, s: &NamedNode, p: &NamedNode) -> Vec<Term> {
        self.store
            .quads_for_pattern(
                Some(NamedOrBlankNode::from(s.clone()).as_ref()),
                Some(NamedNodeRef::from(p)),
                None,
                Some(GraphNameRef::DefaultGraph),
            )
            .filter_map(|q| q.ok().map(|q| q.object))
            .collect()
    }

    pub fn object(&self, s: &NamedNode, p: &NamedNode) -> Option<Term> {
        self.objects(s, p).into_iter().next()
    }

    pub fn subjects(&self, p: &NamedNode, o: &Term) -> Vec<NamedNode> {
        self.store
            .quads_for_pattern(
                None,
                Some(NamedNodeRef::from(p)),
                Some(TermRef::from(o)),
                Some(GraphNameRef::DefaultGraph),
            )
            .filter_map(|q| match q.ok()?.subject {
                NamedOrBlankNode::NamedNode(n) => Some(n),
                NamedOrBlankNode::BlankNode(_) => None,
            })
            .collect()
    }

    pub fn triples_with_subject(&self, s: &NamedNode) -> Vec<(NamedNode, Term)> {
        self.store
            .quads_for_pattern(
                Some(NamedOrBlankNode::from(s.clone()).as_ref()),
                None,
                None,
                Some(GraphNameRef::DefaultGraph),
            )
            .filter_map(|q| q.ok().map(|q| (q.predicate, q.object)))
            .collect()
    }

    pub fn triples_with_object(&self, o: &NamedNode) -> Vec<(NamedNode, NamedNode)> {
        let term = Term::NamedNode(o.clone());
        self.store
            .quads_for_pattern(
                None,
                None,
                Some(TermRef::from(&term)),
                Some(GraphNameRef::DefaultGraph),
            )
            .filter_map(|q| {
                let q = q.ok()?;
                match q.subject {
                    NamedOrBlankNode::NamedNode(s) => Some((q.predicate, s)),
                    NamedOrBlankNode::BlankNode(_) => None,
                }
            })
            .collect()
    }

    pub fn all_with_predicate(&self, p: &NamedNode) -> Vec<(NamedNode, Term)> {
        self.store
            .quads_for_pattern(
                None,
                Some(NamedNodeRef::from(p)),
                None,
                Some(GraphNameRef::DefaultGraph),
            )
            .filter_map(|q| {
                let q = q.ok()?;
                match q.subject {
                    NamedOrBlankNode::NamedNode(s) => Some((s, q.object)),
                    NamedOrBlankNode::BlankNode(_) => None,
                }
            })
            .collect()
    }

    /// Executes `prepared` (already parsed -- see `render::reachable_adjacent_rooms`)
    /// against this graph's own store, with `room` bound to a query
    /// variable named `room`. The one escape hatch into real SPARQL this
    /// crate uses -- see this module's doc comment. Returns no solutions
    /// (rather than erroring) if `prepared` isn't a SELECT query at all or
    /// the bind produces none; a malformed prepared query is a caller-side
    /// mistake in a hardcoded query string, not something this crate's own
    /// commit path could ever produce, so there's nothing a caller could
    /// usefully do with a Result here.
    pub fn query_bound(
        &self,
        prepared: oxigraph::sparql::PreparedSparqlQuery,
        room: NamedNode,
    ) -> Vec<oxigraph::sparql::QuerySolution> {
        let bound = prepared.substitute_variable(
            oxigraph::sparql::Variable::new("room").expect("\"room\" is a valid SPARQL variable name"),
            room,
        );
        match bound.on_store(&self.store).execute() {
            Ok(oxigraph::sparql::QueryResults::Solutions(solutions)) => {
                solutions.filter_map(Result::ok).collect()
            }
            _ => Vec::new(),
        }
    }

    pub fn types_of(&self, s: &NamedNode) -> Vec<NamedNode> {
        self.objects(s, &vocab::rdf_type())
            .into_iter()
            .filter_map(as_node)
            .collect()
    }

    pub fn has_type(&self, s: &NamedNode, class: &NamedNode) -> bool {
        self.types_of(s).contains(class)
    }

    pub fn transcript(&self) -> &[TranscriptEntry] {
        &self.transcript
    }

    /// Every entry committed at or after `since` (a prior call's
    /// `transcript().len()`), in order -- what a caller wanting "what
    /// happened during this one request" needs, without tracking any state
    /// of its own: capture the length before acting, call this with it
    /// after. `since` past the current length (nothing new happened, or a
    /// stale mark from a different graph) just yields an empty slice rather
    /// than panicking.
    pub fn transcript_since(&self, since: u64) -> &[TranscriptEntry] {
        let start = (since as usize).min(self.transcript.len());
        &self.transcript[start..]
    }

    /// The demiurge transcript, human-readable: every commit, in order,
    /// tagged with who proposed it and what changed. This is the literal
    /// audit trail the "creator equal to creation" design implies -- there
    /// is no world content that isn't traceable to exactly one entry here.
    pub fn render_transcript(&self) -> String {
        let mut out = String::new();
        for entry in &self.transcript {
            out.push_str(&format!("--- #{} [{}] ---\n", entry.seq, entry.source));
            for q in &entry.added {
                out.push_str(&format!(
                    "+ {} {} {}\n",
                    short_named(&q.subject_named()),
                    short(&q.predicate),
                    term_str(&q.object)
                ));
            }
            for q in &entry.removed {
                out.push_str(&format!(
                    "- {} {} {}\n",
                    short_named(&q.subject_named()),
                    short(&q.predicate),
                    term_str(&q.object)
                ));
            }
            out.push('\n');
        }
        out
    }

    /// The store's triples, serialized as N-Quads -- the substrate any
    /// persistence path needs regardless of where it ends up living (a
    /// session-snapshot blob, a Durable Object's SQL storage, ...).
    /// Deliberately just the store: `content_hash` and `transcript` are
    /// real state too but live only in this struct, not in the graph
    /// itself, so a full save/restore needs to carry those alongside this
    /// dump rather than trying to fold them into the RDF data -- out of
    /// scope here, this is the substrate that piece would be built on.
    pub fn dump_nquads(&self) -> Result<Vec<u8>, GraphError> {
        self.store
            .dump_to_writer(oxigraph::io::RdfFormat::NQuads, Vec::new())
            .map_err(|e| GraphError::Invalid(e.to_string()))
    }

    /// The other half of the save-side substrate `dump_nquads` provides,
    /// this one for `commit_log` -- the ordered, per-`apply_commit`-call
    /// structure `current_value`/`current_subjects_with` walk (see their
    /// own doc comments) that a flat store dump has no way to represent: a
    /// plain N-Quads dump is a set of triples, with no notion of "which
    /// commit, in what order, produced this one." Without this alongside
    /// `dump_nquads`, a caller reconstructing a graph from a snapshot has
    /// no way to answer "what is the *current* value of this predicate,"
    /// only "what values were *ever* asserted" -- which is exactly the bug
    /// `Game::snapshot`/`Game::from_snapshot`'s round trip used to have
    /// (see `restore_commit_log`'s doc comment for the other half of the
    /// fix, and `vocab::held_by`/`vocab::located_in`'s own doc comments for
    /// the read-side symptoms this caused before commit_log travelled
    /// alongside a snapshot).
    ///
    /// Deliberately its own text format, not reused N-Quads: each record
    /// needs `seq`/`consumes`/`produced` kept as three distinct groups
    /// **in their original order** (`current_value`'s "later wins" logic
    /// depends on `produced`'s own internal order too, not just which
    /// commit a triple came from), and folding all of that into one
    /// N-Quads graph loses exactly that grouping and ordering the moment
    /// it round-trips through a `Store` (an in-memory store's own
    /// iteration order isn't insertion order -- the same reason
    /// `current_value` can't just be a raw pattern lookup to begin with).
    /// A `Store`-based N-Quads *parser* is still reused per produced quad
    /// (via `oxigraph::io::RdfParser`, not a shared `Store`) purely to get
    /// a real N-Quads reader without hand-rolling one -- same trick
    /// `Delta::from_canonical_text`/`parse_nquads` already rely on --
    /// while still parsing one quad at a time so each retains its own
    /// position in `produced`'s sequence.
    pub fn dump_commit_log(&self) -> String {
        let mut out = String::new();
        for record in &self.commit_log {
            out.push_str(&format!(
                "--- seq={} consumes={} produced={}\n",
                record.seq,
                record.consumes.len(),
                record.produced.len(),
            ));
            for c in &record.consumes {
                out.push_str(c.as_str());
                out.push('\n');
            }
            for q in &record.produced {
                // Same gap `Delta::from_canonical_text`/`parse_nquads`
                // work around: `Quad::to_string()` omits the terminating
                // `.` the N-Quads grammar requires -- `restore_commit_log`
                // adds it back on the way in, so it's fine to omit it here.
                out.push_str(&q.to_string());
                out.push('\n');
            }
        }
        out
    }

    /// The load-side counterpart to `dump_commit_log`: parses its text
    /// back into `commit_log`, replacing whatever this graph's own
    /// `commit_log` currently holds (empty on a graph freshly built by
    /// `load_nquads`, which is the one real caller -- see `Game::
    /// from_snapshot`). An empty `text` (nothing to restore, e.g. a
    /// pre-existing snapshot taken before this fix existed) is not an
    /// error: it just leaves `commit_log` empty, the same accepted
    /// (if incomplete) behavior `load_nquads` always had -- a caller
    /// migrating old stored snapshots degrades to the pre-fix behavior for
    /// facts recorded before this existed, rather than failing to load
    /// the session at all.
    pub fn restore_commit_log(&mut self, text: &str) -> Result<(), GraphError> {
        let mut lines = text.lines();
        let mut records = Vec::new();
        while let Some(header) = lines.next() {
            let Some(header) = header.strip_prefix("--- ") else {
                return Err(GraphError::Invalid(format!(
                    "malformed commit-log record header: {header:?}"
                )));
            };
            let mut seq = None;
            let mut n_consumes = None;
            let mut n_produced = None;
            for field in header.split_whitespace() {
                let Some((key, value)) = field.split_once('=') else {
                    return Err(GraphError::Invalid(format!(
                        "malformed commit-log header field: {field:?}"
                    )));
                };
                let parsed: u64 = value.parse().map_err(|_| {
                    GraphError::Invalid(format!("malformed commit-log header value: {field:?}"))
                })?;
                match key {
                    "seq" => seq = Some(parsed),
                    "consumes" => n_consumes = Some(parsed as usize),
                    "produced" => n_produced = Some(parsed as usize),
                    other => {
                        return Err(GraphError::Invalid(format!(
                            "unknown commit-log header field: {other}"
                        )))
                    }
                }
            }
            let seq = seq.ok_or_else(|| {
                GraphError::Invalid("commit-log record header missing seq".into())
            })?;
            let n_consumes = n_consumes.ok_or_else(|| {
                GraphError::Invalid("commit-log record header missing consumes".into())
            })?;
            let n_produced = n_produced.ok_or_else(|| {
                GraphError::Invalid("commit-log record header missing produced".into())
            })?;

            let mut consumes = Vec::with_capacity(n_consumes);
            for _ in 0..n_consumes {
                let line = lines.next().ok_or_else(|| {
                    GraphError::Invalid("commit-log record truncated in consumes section".into())
                })?;
                consumes.push(
                    NamedNode::new(line)
                        .map_err(|e| GraphError::Invalid(format!("bad consumes IRI: {e}")))?,
                );
            }

            let mut produced = Vec::with_capacity(n_produced);
            for _ in 0..n_produced {
                let line = lines.next().ok_or_else(|| {
                    GraphError::Invalid("commit-log record truncated in produced section".into())
                })?;
                let dotted = format!("{line} .");
                let quad = oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::NQuads)
                    .for_slice(dotted.as_bytes())
                    .next()
                    .ok_or_else(|| {
                        GraphError::Invalid(format!("empty produced N-Quad line: {line:?}"))
                    })?
                    .map_err(|e| {
                        GraphError::Invalid(format!("malformed produced N-Quad {line:?}: {e}"))
                    })?;
                produced.push(quad);
            }

            let produced_subjects = produced.iter().filter_map(|q| q.subject_named()).collect();
            records.push(CommitRecord {
                seq,
                consumes,
                produced_subjects,
                produced,
            });
        }
        self.commit_log = records;
        Ok(())
    }
}

trait SubjectNamed {
    fn subject_named(&self) -> Option<NamedNode>;
}

impl SubjectNamed for Quad {
    fn subject_named(&self) -> Option<NamedNode> {
        match &self.subject {
            NamedOrBlankNode::NamedNode(n) => Some(n.clone()),
            NamedOrBlankNode::BlankNode(_) => None,
        }
    }
}

/// When `n` first appeared in `graph`'s transcript -- what callers sort by
/// when "which happened first" matters (a drift rule needing to apply
/// before the threshold rule that reads its result on the same turn, a
/// petition queue needing FIFO order, the map's room listing). Store
/// iteration order isn't creation order (in-memory oxigraph indexes by
/// encoded term id, not insertion), so this can't be read off the store
/// itself -- it scans the transcript instead, looking for the earliest
/// entry that mentions `n` as an added quad's subject or object.
///
/// Returns `(timestamp_ms, seq)`, ordered lexicographically -- deliberately
/// timestamp-primary rather than derived from any bits packed into a
/// minted id (the scheme this replaced): ids are now a pure
/// collision-avoiding hash (see `next_mint_id`) with no ordering
/// information of their own, because ordering needs to be indifferent to
/// *how* a node came to exist -- self-minted by this process, or asserted
/// by an external write path (another Theos, an invited agent) whose own
/// minting scheme this graph has no say over. A timestamp every commit
/// carries regardless of origin is the one ordering signal every write
/// path can supply the same way; see `WorldGraph::set_now`/
/// `TranscriptEntry::timestamp_ms`.
///
/// `seq` (this entry's position in the transcript) is the tie-break for
/// commits that share a timestamp -- routine in practice, since a caller
/// typically calls `set_now` once per player turn while `engine` itself
/// may fire several machines/commits within that one turn (the drift/
/// threshold pair above is exactly this case). Without it, same-millisecond
/// commits would fall back to whatever order the store's `equips()` query
/// happens to return, which isn't insertion order and isn't stable --
/// exactly the bug this tuple exists to close. A node the transcript has
/// no record of (nothing currently constructs one this way, but a caller
/// holding a stale/foreign id could) sorts as `(0, 0)`, i.e. first -- a
/// safe default for "unknown," not a claim about real order.
pub fn creation_order(graph: &WorldGraph, n: &NamedNode) -> (u64, u64) {
    for entry in &graph.transcript {
        let mentioned = entry.added.iter().any(|q| {
            q.subject_named().as_ref() == Some(n) || matches!(&q.object, Term::NamedNode(o) if o == n)
        });
        if mentioned {
            return (entry.timestamp_ms, entry.seq);
        }
    }
    (0, 0)
}

pub fn short(n: &NamedNode) -> String {
    n.as_str()
        .strip_prefix("http://ww/")
        .unwrap_or(n.as_str())
        .to_string()
}

fn short_named(n: &Option<NamedNode>) -> String {
    match n {
        Some(n) => short(n),
        None => "(blank)".to_string(),
    }
}

pub fn term_str(t: &Term) -> String {
    match t {
        Term::NamedNode(n) => short(n),
        Term::Literal(l) => l.value().to_string(),
        other => format!("{other:?}"),
    }
}

/// Does `object` match a `FactRef.object` value, per
/// `SPEC.md`'s "durable node identity" convention?
/// A `NamedNode` object is compared as the `at://` URI it decodes from
/// (`vocab::foreign_uri_from_node`) -- `FactRef.object` is always written
/// in that same durable-address form (never a raw local IRI), matching
/// `FactRef.subject`'s own convention. A `Literal` object (an Attribute's
/// value, e.g. a graded float) is compared by its plain string value.
/// Shared by `WorldGraph::apply_commit`'s FactRef guard and (as its own,
/// appview-local copy -- `appview` doesn't depend on `dmml_runtime::graph`'s
/// private items) `appview`'s resolve-time retraction filter.
fn term_matches_fact_object(object: &Term, expected: &str) -> bool {
    match object {
        Term::NamedNode(n) => vocab::foreign_uri_from_node(n).as_deref() == Some(expected),
        Term::Literal(l) => l.value() == expected,
        _ => false,
    }
}

pub fn as_node(t: Term) -> Option<NamedNode> {
    match t {
        Term::NamedNode(n) => Some(n),
        _ => None,
    }
}

pub fn as_string(t: &Term) -> Option<String> {
    match t {
        Term::Literal(l) => Some(l.value().to_string()),
        _ => None,
    }
}

pub fn as_bool(t: &Term) -> Option<bool> {
    as_string(t).and_then(|s| s.parse().ok())
}

pub fn as_float(t: &Term) -> Option<f32> {
    as_string(t).and_then(|s| s.parse().ok())
}

pub fn as_int(t: &Term) -> Option<i64> {
    as_string(t).and_then(|s| s.parse().ok())
}

pub fn lit_str(s: impl Into<String>) -> Literal {
    Literal::new_simple_literal(s)
}

pub fn lit_bool(b: bool) -> Literal {
    Literal::new_typed_literal(b.to_string(), xsd::BOOLEAN)
}

pub fn lit_float(f: f32) -> Literal {
    Literal::new_typed_literal(f.to_string(), xsd::FLOAT)
}

pub fn lit_int(i: u64) -> Literal {
    Literal::new_typed_literal(i.to_string(), xsd::INTEGER)
}

// -- Validation --------------------------------------------------------

/// The fixed shape rules: which base sorts a given predicate's subject/
/// object must have, and which XSD datatype a given predicate's literal
/// values must carry. This is `relation_allowed()` + `AttrDomain` from the
/// hand-rolled version, re-homed. Deliberately not exhaustive — it covers
/// the failure modes this prototype's own generator can actually produce,
/// not a general-purpose graph schema validator.
fn validate(store: &Store, delta: &Delta) -> Result<(), GraphError> {
    let mut pending_types: HashMap<NamedNode, HashSet<NamedNode>> = HashMap::new();
    for q in &delta.add {
        if q.predicate == vocab::rdf_type() {
            if let NamedOrBlankNode::NamedNode(s) = &q.subject {
                if let Term::NamedNode(class) = &q.object {
                    pending_types
                        .entry(s.clone())
                        .or_default()
                        .insert(class.clone());
                }
            }
        }
    }

    let kind_of = |n: &NamedNode| -> HashSet<NamedNode> {
        let mut kinds: HashSet<NamedNode> = pending_types.get(n).cloned().unwrap_or_default();
        for q in store
            .quads_for_pattern(
                Some(NamedOrBlankNode::from(n.clone()).as_ref()),
                Some(NamedNodeRef::from(&vocab::rdf_type())),
                None,
                Some(GraphNameRef::DefaultGraph),
            )
            .flatten()
        {
            if let Term::NamedNode(class) = q.object {
                kinds.insert(class);
            }
        }
        kinds
    };

    for q in &delta.add {
        let NamedOrBlankNode::NamedNode(subject) = &q.subject else {
            return Err(GraphError::Invalid(
                "blank node subjects unsupported".into(),
            ));
        };
        let p = &q.predicate;

        if *p == vocab::contains() {
            // Room, or a PetitionSnapshot's frozen copy of what a room
            // contained at raise time -- see `commune::freeze_room_snapshot`.
            require_object_kind(
                &kind_of(subject),
                &[vocab::class_room(), vocab::class_petition_snapshot()],
                "contains subject",
            )?;
            require_object_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &[
                    vocab::class_item(),
                    vocab::class_player(),
                    vocab::class_npc(),
                ],
                "contains object",
            )?;
        } else if *p == vocab::holds() {
            require_object_kind(
                &kind_of(subject),
                &[vocab::class_player(), vocab::class_npc()],
                "holds subject",
            )?;
            require_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &vocab::class_item(),
                "holds object",
            )?;
        } else if *p == vocab::connects_to() {
            // Room, or a PetitionSnapshot's frozen copy of what a room
            // connected to at raise time -- see
            // `commune::freeze_room_snapshot`.
            require_object_kind(
                &kind_of(subject),
                &[vocab::class_room(), vocab::class_petition_snapshot()],
                "connectsTo subject",
            )?;
            require_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &vocab::class_edge(),
                "connectsTo object",
            )?;
        } else if *p == vocab::to() {
            require_kind(&kind_of(subject), &vocab::class_edge(), "edge `to` subject")?;
            require_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &vocab::class_room(),
                "edge `to` object",
            )?;
        } else if *p == vocab::petition_concerns() {
            require_kind(
                &kind_of(subject),
                &vocab::class_petition(),
                "petitionConcerns subject",
            )?;
            require_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &vocab::class_room(),
                "petitionConcerns object",
            )?;
        } else if *p == vocab::petition_context() {
            // A PetitionSnapshot node reference, not a JSON string literal
            // -- see `commune::freeze_room_snapshot` and issue #15.
            require_kind(
                &kind_of(subject),
                &vocab::class_petition(),
                "petitionContext subject",
            )?;
            require_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &vocab::class_petition_snapshot(),
                "petitionContext object",
            )?;
        } else if *p == vocab::equips() {
            require_object_kind(
                &kind_of(subject),
                &[
                    vocab::class_room(),
                    vocab::class_item(),
                    vocab::class_player(),
                    vocab::class_npc(),
                ],
                "equips subject",
            )?;
            require_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &vocab::class_machine(),
                "equips object",
            )?;
        } else if *p == vocab::name()
            || *p == vocab::direction()
            || *p == vocab::petition_result()
            || *p == vocab::foreign_snapshot()
            || *p == vocab::render_kind()
        {
            expect_literal_type(&q.object, xsd::STRING)?;
        } else if *p == vocab::noticed_change() {
            require_kind(&kind_of(subject), &vocab::class_room(), "noticedChange subject")?;
            require_kind(
                &kind_of_object(&kind_of, &q.object)?,
                &vocab::class_drift(),
                "noticedChange object",
            )?;
        } else if *p == vocab::petition_status() {
            // A closed, three-valued enum expressed as node identity, not
            // a string tag -- see vocab::status_pending/resolved/expired.
            let allowed = [
                vocab::status_pending(),
                vocab::status_resolved(),
                vocab::status_expired(),
            ];
            match &q.object {
                Term::NamedNode(n) if allowed.contains(n) => {}
                _ => {
                    return Err(GraphError::Invalid(format!(
                        "petitionStatus must be one of {}",
                        allowed.iter().map(short).collect::<Vec<_>>().join(", ")
                    )))
                }
            }
        } else if *p == vocab::foreign_uri()
            || *p == vocab::foreign_cid()
            || *p == vocab::drift_old_cid()
            || *p == vocab::drift_new_cid()
        {
            // Real external identifiers (an at:// URI, a content-address
            // hash) or references to one -- stored as node references so
            // they're joinable/inspectable the same way any other graph
            // reference is, not opaque text.
            if !matches!(q.object, Term::NamedNode(_)) {
                return Err(GraphError::Invalid(format!(
                    "{} must be a node reference, not a literal",
                    short(p)
                )));
            }
        } else if *p == vocab::locked() || *p == vocab::portable() {
            expect_literal_type(&q.object, xsd::BOOLEAN)?;
        } else if let Some((lo, hi)) = graded_range(p) {
            expect_literal_type(&q.object, xsd::FLOAT)?;
            let v = as_float(&q.object)
                .ok_or_else(|| GraphError::Invalid(format!("{} must be a float", short(p))))?;
            if !(lo..=hi).contains(&v) {
                return Err(GraphError::Invalid(format!(
                    "{} {v} out of domain [{lo}, {hi}]",
                    short(p)
                )));
            }
        } else if *p == vocab::seen_count()
            || *p == vocab::visits()
            || *p == vocab::petition_expires_at()
            || *p == vocab::drift_observed_at()
        {
            expect_literal_type(&q.object, xsd::INTEGER)?;
            let v: i64 = as_int(&q.object)
                .ok_or_else(|| GraphError::Invalid(format!("{} must be an integer", short(p))))?;
            if v < 0 {
                return Err(GraphError::Invalid(format!(
                    "{} must not be negative",
                    short(p)
                )));
            }
        } else if *p != vocab::rdf_type() && !is_closed_vocabulary(p) {
            // Anything else is a genuinely novel predicate -- the
            // substrate the demiurge (or any source) uses to introduce a
            // relation type this schema didn't already fix in advance.
            // It must self-declare first: `<p> rdf:type ww:Relation` for a
            // node-to-node relation, or `ww:Attribute` for node-to-literal,
            // asserted earlier or within this same delta (the same
            // pending_types + store lookup `kind_of` already does for
            // every other kind check here). The declaration is just
            // another triple, so it's inspectable the same way
            // crystallized kinds are, not a side channel. We don't further
            // constrain *what* it relates -- open-ended by design -- only
            // that its shape (node vs. literal object) matches what it
            // declared.
            check_declared_shape(&kind_of(p), p, &q.object)?;
        }
        // Structural glue predicates (hasRequirement/hasEffect/senses/
        // requirementAttrPredicate/effectAttrPredicate/requirementRoom/
        // requirementEdge/requirementAttrNode/effectTargetNode/effectEdge)
        // and rdf:type itself are intentionally left unchecked beyond
        // referential shape — see module doc. Note this final `else if`'s
        // guard is `is_closed_vocabulary`, not just `is_structural_glue`:
        // every dedicated branch above it (`contains`, `holds`,
        // `petitionStatus`, the graded-range attrs, etc.) already matches
        // earlier in this same if/else chain and so never reaches here
        // regardless -- `is_closed_vocabulary` is the *complete* closed set
        // (dedicated branches ∪ `is_structural_glue`), kept as one function
        // so `validate_self_declared` below, which has no `Store` to run
        // the dedicated branches' own shape/range checks, can still ask
        // this same function "does this predicate need self-declaration"
        // and get an answer that can't drift from this one's (see that
        // function's own doc comment, and jedelman/written-world PR #37's
        // bug 1).
    }
    Ok(())
}

/// Single-predicate half of the self-declaration mechanism: does `object`'s
/// shape (node vs. literal) match what `kinds` says `p` was declared as?
/// Factored out of `validate`'s own novel-predicate branch so
/// `validate_self_declared` below -- the generalized, domain-agnostic
/// version `appview` uses (see its own doc comment for why it can't reuse
/// `validate` wholesale) -- can share the exact same shape-checking logic
/// rather than a hand-copied duplicate that could drift.
fn check_declared_shape(
    kinds: &HashSet<NamedNode>,
    p: &NamedNode,
    object: &Term,
) -> Result<(), GraphError> {
    if kinds.contains(&vocab::class_relation()) {
        if !matches!(object, Term::NamedNode(_)) {
            return Err(GraphError::Invalid(format!(
                "{} is declared a Relation and must take a node object",
                short(p)
            )));
        }
    } else if kinds.contains(&vocab::class_attribute()) {
        if !matches!(object, Term::Literal(_)) {
            return Err(GraphError::Invalid(format!(
                "{} is declared an Attribute and must take a literal object",
                short(p)
            )));
        }
    } else {
        return Err(GraphError::Invalid(format!(
            "{} is not a recognized predicate -- declare it `rdf:type` \
             ww:Relation or ww:Attribute before using it",
            short(p)
        )));
    }
    Ok(())
}

/// The generalized, domain-agnostic version of `validate`'s *full*
/// predicate-acceptance logic -- not just the self-declaration half.
/// Factored out so a consumer that carries none of `validate`'s
/// `Store`-backed machinery (no in-memory graph, no commit history) can
/// still reach the exact same verdict `validate` would for "is this
/// predicate usage valid," on a plain batch of quads. This is what
/// `SPEC.md` settled on for `appview`: "no
/// new structural-glue vocabulary gets added to appview" -- the AppView
/// doesn't know what a `Room` or an `Item` is, and shouldn't, so it can't
/// run `validate`'s dedicated per-predicate shape/range branches (the
/// `contains`-subject-must-be-a-Room kind of check) -- but it doesn't need
/// to: those branches, and `is_structural_glue`'s fully-unchecked
/// predicates, are *exempt from self-declaration* in `validate` regardless
/// of what deeper check (if any) they get there, and `is_closed_vocabulary`
/// is the single function both `validate` and this one consult to agree on
/// that exemption set. The open self-declaration half of the mechanism
/// generalizes with no changes needed beyond that, per the spec's own
/// wording.
///
/// Before this function skipped closed-vocabulary predicates too, it only
/// exempted `rdf:type` itself, so a commit asserting a perfectly ordinary
/// closed-vocabulary predicate `validate` accepts unconditionally --
/// `hasRequirement`, `petitionStatus`, `locked`, and roughly two dozen
/// others -- got wrongly rejected here with "not a recognized predicate,"
/// since none of them are ever self-declared as `Relation`/`Attribute`
/// (jedelman/written-world PR #37 code review, bug 1). Both of this
/// function's call sites -- `server/src/atproto/commit_write.rs`'s
/// write-time best-effort check and `appview`'s resolve-time check -- call
/// this same function, so they can't independently drift the way the
/// write-time check drifted from `validate` before this fix.
///
/// `quads` is the batch being checked -- a single commit's `produces`, at
/// `server/src/atproto/commit_write.rs`'s write-time best-effort call site,
/// or a resolved multi-commit union at `appview`'s resolve-time call site.
/// `already_declared` supplies whatever "known beyond `quads` itself"
/// context the caller has: `validate`'s own call site below threads in a
/// Store-backed lookup (unioned with the same in-delta `pending_types` it
/// already computes); `commit_write.rs` has no such context and passes a
/// closure that always returns an empty set (this call is inherently
/// single-commit and best-effort -- see that module's own doc comment for
/// why it can't do better without a network round trip it doesn't make);
/// `appview` passes a declaration index built from the whole indexed
/// corpus (see `appview::build_declared_kinds`). A predicate's own
/// `rdf:type` triple *within* `quads` always counts regardless of what
/// `already_declared` reports, mirroring `pending_types`' role in
/// `validate` itself.
///
/// Declaration order is deliberately not enforced between `already_declared`
/// and `quads` -- unlike a single `Store`'s "already committed" semantics,
/// a federated multi-repo commit DAG has no single global "earlier": two
/// repos can each write a commit referencing a shape the other declared,
/// with neither well-ordered relative to the other from a resolving
/// AppView's vantage point. Membership in the reachable/known set is what's
/// checked here, not temporal precedence.
pub fn validate_self_declared<'a>(
    quads: impl IntoIterator<Item = &'a Quad> + Clone,
    already_declared: impl Fn(&NamedNode) -> HashSet<NamedNode>,
) -> Result<(), GraphError> {
    let mut pending: HashMap<NamedNode, HashSet<NamedNode>> = HashMap::new();
    for q in quads.clone() {
        if q.predicate == vocab::rdf_type() {
            if let (NamedOrBlankNode::NamedNode(s), Term::NamedNode(c)) = (&q.subject, &q.object) {
                pending.entry(s.clone()).or_default().insert(c.clone());
            }
        }
    }

    for q in quads {
        let NamedOrBlankNode::NamedNode(_) = &q.subject else {
            return Err(GraphError::Invalid(
                "blank node subjects unsupported".into(),
            ));
        };
        let p = &q.predicate;
        if *p == vocab::rdf_type() || is_closed_vocabulary(p) {
            // `rdf:type`: the declaration triple is the mechanism, not
            // something the mechanism checks itself -- same exemption
            // `validate` itself grants.
            //
            // `is_closed_vocabulary(p)`: this predicate is part of
            // `validate`'s own fixed vocabulary (a dedicated shape/range
            // branch there, or `is_structural_glue`), so `validate` itself
            // never demands self-declaration for it either -- regardless of
            // whatever deeper shape/range check `validate` separately runs
            // for it there, a check this function has no `Store` to run and
            // was never meant to replicate (see doc comment above).
            continue;
        }
        let mut kinds = already_declared(p);
        if let Some(local) = pending.get(p) {
            kinds.extend(local.iter().cloned());
        }
        check_declared_shape(&kinds, p, &q.object)?;
    }
    Ok(())
}

/// Parses `text` as standard N-Quads -- the public entry point onto
/// `parse_nquads` below for a caller outside this module (`appview`,
/// `commit_write.rs`) that needs the same `Commit::produces` parsing this
/// module already does, rather than a second hand-rolled copy. See
/// `parse_nquads`'s own doc comment for the parsing details.
pub fn parse_produces(text: &str) -> Result<Vec<Quad>, GraphError> {
    parse_nquads(text)
}

/// The fixed shape-glue predicates that structure a desiring-machine's own
/// anatomy (`Requirement`/`Effect` nodes) rather than describing world
/// content. Deliberately exempt from the novel-predicate self-declaration
/// requirement below -- they're part of the closed vocabulary, just not
/// worth a dedicated kind-checked branch above (see the pre-existing
/// comment at the end of `validate`).
fn is_structural_glue(p: &NamedNode) -> bool {
    *p == vocab::has_requirement()
        || *p == vocab::has_effect()
        || *p == vocab::senses()
        || *p == vocab::requirement_attr_predicate()
        || *p == vocab::effect_attr_predicate()
        || *p == vocab::requirement_room()
        || *p == vocab::requirement_edge()
        || *p == vocab::requirement_attr_node()
        || *p == vocab::effect_target_node()
        || *p == vocab::effect_edge()
        // `via`/`respondsTo`/`commitPredicate` structure a `Commit`-
        // provenance node's own anatomy (`WorldGraph::apply_commit`'s
        // `extra_quads`, minted whenever `Commit.via`/`responds_to` is
        // `Some`) rather than describing world content -- same category
        // as `has_requirement` et al. above. Found missing here by
        // `demiurge::bootstrap`'s own seed-node migration (#50 Tier 1
        // item 2): nothing had exercised `via` through the validated
        // `replay_commit` path before (every prior caller either never
        // set `via` at all, or never replayed the resulting commit), so
        // this gap existed but had never actually been hit until a real
        // caller (genesis content, now `via` the world's seed) needed it
        // to survive a validated re-run.
        || *p == vocab::via()
        || *p == vocab::responds_to()
        || *p == vocab::commit_predicate()
        || *p == vocab::unlocks_field()
}

/// The complete closed-vocabulary predicate set: every predicate `validate`
/// gives a dedicated shape/range branch to, unioned with
/// `is_structural_glue`'s fully-unchecked set. None of these ever reach
/// `validate`'s self-declaration branch -- by construction, since each one
/// matches an earlier arm of `validate`'s own `if`/`else if` chain (or, for
/// `is_structural_glue`'s predicates, is explicitly excluded from the final
/// arm) -- so none of them require a predicate to self-declare via
/// `rdf:type ww:Relation`/`ww:Attribute` before use.
///
/// This is the single source of truth `validate`'s own final-branch guard
/// and `validate_self_declared` (write-time and resolve-time, neither of
/// which has `Store` access to run the dedicated branches' actual
/// shape/range checks) both consult for "does this predicate need
/// self-declaration." Before this function existed, `validate_self_declared`
/// only knew about `is_structural_glue`'s narrower ten-predicate list,
/// silently rejecting the rest of this set at write/resolve time even
/// though `validate` itself accepted them unconditionally
/// (jedelman/written-world PR #37 code review, bug 1). Keeping this as one
/// function both call sites share is what closes that drift, per the fix
/// direction in `dev-journal/2026-08-12-pr37-review-findings-and-fix-
/// direction.md`: "the fix should be to use the same validation code in
/// each place."
fn is_closed_vocabulary(p: &NamedNode) -> bool {
    is_structural_glue(p)
        || *p == vocab::contains()
        || *p == vocab::holds()
        || *p == vocab::connects_to()
        || *p == vocab::to()
        || *p == vocab::petition_concerns()
        || *p == vocab::petition_context()
        || *p == vocab::equips()
        || *p == vocab::name()
        || *p == vocab::direction()
        || *p == vocab::petition_result()
        || *p == vocab::foreign_snapshot()
        || *p == vocab::render_kind()
        || *p == vocab::noticed_change()
        || *p == vocab::petition_status()
        || *p == vocab::foreign_uri()
        || *p == vocab::foreign_cid()
        || *p == vocab::drift_old_cid()
        || *p == vocab::drift_new_cid()
        || *p == vocab::locked()
        || *p == vocab::portable()
        || graded_range(p).is_some()
        || *p == vocab::seen_count()
        || *p == vocab::visits()
        || *p == vocab::petition_expires_at()
        || *p == vocab::drift_observed_at()
}

fn kind_of_object(
    kind_of: &impl Fn(&NamedNode) -> HashSet<NamedNode>,
    object: &Term,
) -> Result<HashSet<NamedNode>, GraphError> {
    match object {
        Term::NamedNode(n) => Ok(kind_of(n)),
        _ => Err(GraphError::Invalid(
            "expected a node reference, found a literal".into(),
        )),
    }
}

fn require_kind(
    kinds: &HashSet<NamedNode>,
    expected: &NamedNode,
    context: &str,
) -> Result<(), GraphError> {
    if kinds.contains(expected) {
        Ok(())
    } else {
        Err(GraphError::Invalid(format!(
            "{context} must have kind {expected}"
        )))
    }
}

fn require_object_kind(
    kinds: &HashSet<NamedNode>,
    expected: &[NamedNode],
    context: &str,
) -> Result<(), GraphError> {
    if expected.iter().any(|e| kinds.contains(e)) {
        Ok(())
    } else {
        Err(GraphError::Invalid(format!(
            "{context} must have one of {expected:?}"
        )))
    }
}

fn expect_literal_type(t: &Term, datatype: NamedNodeRef<'_>) -> Result<(), GraphError> {
    match t {
        // Simple literals (`Literal::new_simple_literal`) already report
        // xsd:string as their datatype in RDF 1.1, so this single check
        // covers both typed and simple string literals.
        Term::Literal(l) if l.datatype() == datatype => Ok(()),
        _ => Err(GraphError::Invalid(format!(
            "expected literal of type {datatype}, got {t:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders `quads` as `Commit::produces` wants them: standard,
    /// dot-terminated N-Quads text -- see `parse_nquads`'s own doc comment
    /// for why `Quad::to_string()` alone isn't enough.
    fn produces_text(quads: &[Quad]) -> String {
        quads
            .iter()
            .map(|q| format!("{q} ."))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A commit consuming/producing nothing but the node this test cares
    /// about, addressed the same way a real `consumes` reference would be
    /// (`vocab::foreign_uri_node(uri)`) -- `apply_commit`'s `consumes`
    /// existence guard only ever recognizes a node that's *already*
    /// appeared in the store under that same address, so a mint that wants
    /// to be consumable later has to assert triples about the encoded
    /// address itself, not some unrelated locally-minted `NamedNode`. This
    /// is exactly the shape a real cross-repo mint would have: the graph
    /// only ever comes to know a foreign record by its `StrongRef.uri`.
    fn mint(graph: &mut WorldGraph, uri: &str, extra: Vec<Quad>) {
        let node = vocab::foreign_uri_node(uri);
        let mut quads = vec![Quad::new(
            node.clone(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_room()),
            oxigraph::model::GraphName::DefaultGraph,
        )];
        quads.extend(extra);
        let commit = Commit {
            consumes: Vec::new(),
            produces: produces_text(&quads),
            predicate: "mints".to_string(),
            via: None,
            responds_to: None,
            created_at: "0".to_string(),
        };
        graph.apply_commit("test", commit).expect("mint is valid");
    }

    #[test]
    fn apply_commit_mint_then_retract_marks_node_retracted_without_deleting_it() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/1";
        mint(&mut graph, uri, Vec::new());

        // Nothing has consumed it yet -- current by default.
        assert_eq!(graph.consume_state(uri), ConsumeState::Current);
        assert!(!graph.is_retracted(uri));

        // A pure retraction: consumes the node, produces nothing.
        let retract = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef {
                uri: uri.to_string(),
                cid: "cid-at-mint".to_string(),
            })],
            produces: String::new(),
            predicate: "retracts".to_string(),
            via: None,
            responds_to: None,
            created_at: "1".to_string(),
        };
        graph
            .apply_commit("test", retract)
            .expect("retracting a node the graph already knows about is valid");

        assert!(graph.is_retracted(uri));
        match graph.consume_state(uri) {
            ConsumeState::Retracted { seq } => assert_eq!(seq, 1),
            ConsumeState::Current => panic!("expected the room to be retracted"),
        }

        // "No deletions": the originally-minted triple is still physically
        // in the store -- only the query-level state flipped.
        let node = vocab::foreign_uri_node(uri);
        assert!(graph.has_type(&node, &vocab::class_room()));
    }

    #[test]
    fn apply_commit_consume_with_reassertion_stays_current() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/2";
        mint(&mut graph, uri, Vec::new());
        let node = vocab::foreign_uri_node(uri);

        // An attribute-change commit: consumes the node's old state, but
        // its `produces` re-asserts the same node -- an in-place update,
        // not a supersession.
        let update = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef {
                uri: uri.to_string(),
                cid: "cid-at-mint".to_string(),
            })],
            produces: produces_text(&[Quad::new(
                node.clone(),
                vocab::name(),
                Term::Literal(lit_str("a freshly renamed room")),
                oxigraph::model::GraphName::DefaultGraph,
            )]),
            predicate: "becomes".to_string(),
            via: None,
            responds_to: None,
            created_at: "1".to_string(),
        };
        graph
            .apply_commit("test", update)
            .expect("updating a known node in place is valid");

        assert_eq!(graph.consume_state(uri), ConsumeState::Current);
        assert!(!graph.is_retracted(uri));
        assert_eq!(
            graph.object(&node, &vocab::name()),
            Some(Term::Literal(lit_str("a freshly renamed room")))
        );
    }

    #[test]
    fn apply_commit_rejects_consuming_an_unknown_node() {
        let mut graph = WorldGraph::new();
        let bogus = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef {
                uri: "at://did:example:test/org.jason-edelman.writtenworld.room/never-minted"
                    .to_string(),
                cid: "cid".to_string(),
            })],
            produces: String::new(),
            predicate: "retracts".to_string(),
            via: None,
            responds_to: None,
            created_at: "0".to_string(),
        };
        assert!(graph.apply_commit("test", bogus).is_err());
    }

    /// Issue #53: `consumes` must check the `cid` half of a `StrongRef`,
    /// not just that the `uri` is known to the graph. Once a node has a
    /// `foreignCid` fact on record (asserted by some earlier `via`/
    /// `respondsTo`, the only mechanism that records one today -- see
    /// `apply_commit`'s own doc comment), a later `consumes` claiming a
    /// *different* cid for that same uri is a fabricated/stale reference
    /// and must be rejected, existence-not-currency (`SPEC.md` section 11):
    /// this only ever checks "was this cid genuinely ever recorded for
    /// this uri," never "is it the latest one."
    #[test]
    fn apply_commit_rejects_consuming_a_node_with_a_cid_that_was_never_recorded() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/reach";
        mint(&mut graph, uri, Vec::new());

        // Record a real observed cid for `uri` the only way `apply_commit`
        // knows how today: a `respondsTo` provenance triple.
        let observe = Commit {
            consumes: Vec::new(),
            produces: String::new(),
            predicate: "notices".to_string(),
            via: None,
            responds_to: Some(StrongRef {
                uri: uri.to_string(),
                cid: "cid-genuinely-observed".to_string(),
            }),
            created_at: "1".to_string(),
        };
        graph
            .apply_commit("test", observe)
            .expect("recording an observed cid via respondsTo is valid");

        // A later commit claims to consume the SAME uri but a cid that was
        // never actually recorded for it -- fabricated or stale, and must
        // be rejected now that a real cid is on record to check against.
        let forged = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef {
                uri: uri.to_string(),
                cid: "cid-never-actually-observed".to_string(),
            })],
            produces: String::new(),
            predicate: "unlocks".to_string(),
            via: None,
            responds_to: None,
            created_at: "2".to_string(),
        };
        let err = graph
            .apply_commit("test", forged)
            .expect_err("a consumes cid that was never recorded for this uri must be rejected");
        assert!(
            matches!(err, GraphError::Invalid(ref s) if s.contains("cid")),
            "expected a cid-mismatch error, got: {err}"
        );
    }

    /// The positive case: a `consumes` cid that genuinely matches a
    /// recorded `foreignCid` observation is accepted, same as before #53.
    #[test]
    fn apply_commit_accepts_consuming_a_node_with_its_genuinely_recorded_cid() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/reach2";
        mint(&mut graph, uri, Vec::new());

        let observe = Commit {
            consumes: Vec::new(),
            produces: String::new(),
            predicate: "notices".to_string(),
            via: None,
            responds_to: Some(StrongRef {
                uri: uri.to_string(),
                cid: "cid-genuinely-observed".to_string(),
            }),
            created_at: "1".to_string(),
        };
        graph
            .apply_commit("test", observe)
            .expect("recording an observed cid via respondsTo is valid");

        let legitimate = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef {
                uri: uri.to_string(),
                cid: "cid-genuinely-observed".to_string(),
            })],
            produces: String::new(),
            predicate: "unlocks".to_string(),
            via: None,
            responds_to: None,
            created_at: "2".to_string(),
        };
        assert!(
            graph.apply_commit("test", legitimate).is_ok(),
            "a consumes cid matching a genuinely recorded observation must be accepted"
        );
    }

    /// A node that has never had ANY cid recorded against it (the common
    /// case for a purely local mint that no `via`/`respondsTo` has ever
    /// pointed at) has nothing to check a `consumes` cid against yet --
    /// #53 is existence-of-a-real-record, not "every node must always
    /// have carried a cid": accept, same as before #53, rather than
    /// inventing a requirement the issue never asked for.
    #[test]
    fn apply_commit_accepts_consuming_a_node_with_no_recorded_cid_yet() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/never-observed";
        mint(&mut graph, uri, Vec::new());

        let commit = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef {
                uri: uri.to_string(),
                cid: "whatever-the-caller-claims".to_string(),
            })],
            produces: String::new(),
            predicate: "retracts".to_string(),
            via: None,
            responds_to: None,
            created_at: "1".to_string(),
        };
        assert!(
            graph.apply_commit("test", commit).is_ok(),
            "a node with no recorded cid yet has nothing to check against"
        );
    }

    /// Real drift, real coverage: after a node's cid has drifted from `A`
    /// to `B` (two separate `respondsTo` observations), a `consumes`
    /// entry may legitimately reference EITHER one -- #53's whole point.
    /// A regression that only checked the latest recorded cid would pass
    /// every other test in this file but fail here.
    #[test]
    fn apply_commit_accepts_consuming_either_cid_after_drift() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/drifted";
        mint(&mut graph, uri, Vec::new());

        for cid in ["cid-A", "cid-B"] {
            let observe = Commit {
                consumes: Vec::new(),
                produces: String::new(),
                predicate: "notices".to_string(),
                via: None,
                responds_to: Some(StrongRef { uri: uri.to_string(), cid: cid.to_string() }),
                created_at: "1".to_string(),
            };
            graph.apply_commit("test", observe).expect("recording each drift observation is valid");
        }

        for cid in ["cid-A", "cid-B"] {
            let consume = Commit {
                consumes: vec![ConsumeRef::Strong(StrongRef { uri: uri.to_string(), cid: cid.to_string() })],
                produces: String::new(),
                predicate: "unlocks".to_string(),
                via: None,
                responds_to: None,
                created_at: "2".to_string(),
            };
            assert!(
                graph.apply_commit("test", consume).is_ok(),
                "consuming the older cid ({cid}) after drift must still be accepted -- existence, not currency"
            );
        }

        // Currency IS still not the rule, pinned explicitly so a future
        // "tighten this to latest-only" change fails a test instead of
        // shipping silently.
        let stale = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef { uri: uri.to_string(), cid: "cid-A".to_string() })],
            produces: String::new(),
            predicate: "unlocks-again".to_string(),
            via: None,
            responds_to: None,
            created_at: "3".to_string(),
        };
        assert!(
            graph.apply_commit("test", stale).is_ok(),
            "cid-A is still on record even though cid-B is newer -- currency was never the rule"
        );
    }

    /// A commit that observes a node's new cid (`respondsTo`) and, in the
    /// same breath, acts on that new cid (`consumes`) is self-consistent
    /// and must be accepted regardless of which field `apply_commit`
    /// happens to process first internally -- this is the real drift-flow
    /// shape (fetch, notice a new cid, act on it), not a synthetic case.
    #[test]
    fn apply_commit_accepts_a_commit_that_both_observes_and_consumes_the_same_new_cid() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/same-commit-drift";
        mint(&mut graph, uri, Vec::new());

        let baseline = Commit {
            consumes: Vec::new(),
            produces: String::new(),
            predicate: "notices".to_string(),
            via: None,
            responds_to: Some(StrongRef { uri: uri.to_string(), cid: "cid-old".to_string() }),
            created_at: "1".to_string(),
        };
        graph.apply_commit("test", baseline).expect("baseline observation is valid");

        let observe_and_act = Commit {
            consumes: vec![ConsumeRef::Strong(StrongRef { uri: uri.to_string(), cid: "cid-new".to_string() })],
            produces: String::new(),
            predicate: "unlocks".to_string(),
            via: None,
            responds_to: Some(StrongRef { uri: uri.to_string(), cid: "cid-new".to_string() }),
            created_at: "2".to_string(),
        };
        assert!(
            graph.apply_commit("test", observe_and_act).is_ok(),
            "observing a new cid and consuming it in the same commit must be self-consistent"
        );
    }

    /// `mint`'s node-addressing convention, but for a single `(subject,
    /// predicate, object)` triple asserted on top of an already-minted
    /// node -- what a `FactRef` test needs to set up "some prior commit
    /// really did assert this fact" without going through the full
    /// `assert_one` helper (which uses ordinary locally-minted identities,
    /// not `foreign_uri_node`-addressed ones -- see its own doc comment).
    fn assert_durable_fact(graph: &mut WorldGraph, subject_uri: &str, predicate: &NamedNode, object: Term) {
        let subject = vocab::foreign_uri_node(subject_uri);
        let commit = Commit {
            consumes: Vec::new(),
            produces: produces_text(&[Quad::new(
                subject,
                predicate.clone(),
                object,
                oxigraph::model::GraphName::DefaultGraph,
            )]),
            predicate: "becomes".to_string(),
            via: None,
            responds_to: None,
            created_at: "0".to_string(),
        };
        graph
            .apply_commit("test", commit)
            .expect("asserting a durable fact on an already-minted node is valid");
    }

    #[test]
    fn apply_commit_accepts_a_factref_matching_an_existing_triple() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/4";
        mint(&mut graph, uri, Vec::new());
        assert_durable_fact(
            &mut graph,
            uri,
            &vocab::name(),
            Term::Literal(lit_str("the original name")),
        );

        let referencing = Commit {
            consumes: vec![ConsumeRef::Fact(FactRef {
                commit: StrongRef {
                    uri: "at://did:example:test/org.jason-edelman.writtenworld.commit/whatever"
                        .to_string(),
                    cid: "cid-whatever".to_string(),
                },
                subject: uri.to_string(),
                predicate: vocab::name().as_str().to_string(),
                object: Some("the original name".to_string()),
            })],
            produces: String::new(),
            predicate: "retracts".to_string(),
            via: None,
            responds_to: None,
            created_at: "1".to_string(),
        };
        assert!(
            graph.apply_commit("test", referencing).is_ok(),
            "a factRef matching a triple that genuinely exists in the store should be accepted"
        );
    }

    #[test]
    fn apply_commit_rejects_a_factref_matching_no_triple() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/5";
        mint(&mut graph, uri, Vec::new());

        let referencing = Commit {
            consumes: vec![ConsumeRef::Fact(FactRef {
                commit: StrongRef {
                    uri: "at://did:example:test/org.jason-edelman.writtenworld.commit/whatever"
                        .to_string(),
                    cid: "cid-whatever".to_string(),
                },
                subject: uri.to_string(),
                predicate: vocab::name().as_str().to_string(),
                object: Some("a name nobody ever asserted".to_string()),
            })],
            produces: String::new(),
            predicate: "retracts".to_string(),
            via: None,
            responds_to: None,
            created_at: "1".to_string(),
        };
        assert!(
            graph.apply_commit("test", referencing).is_err(),
            "a factRef matching no triple the store actually holds should be rejected"
        );
    }
    #[test]
    fn apply_commit_asserts_via_and_responds_to_as_triples() {
        let mut graph = WorldGraph::new();
        let uri = "at://did:example:test/org.jason-edelman.writtenworld.room/3";
        mint(&mut graph, uri, Vec::new());

        let commit = Commit {
            consumes: Vec::new(),
            produces: String::new(),
            predicate: "grants".to_string(),
            via: Some(StrongRef {
                uri: "at://did:example:test/org.jason-edelman.writtenworld.operation/1"
                    .to_string(),
                cid: "op-cid".to_string(),
            }),
            responds_to: Some(StrongRef {
                uri: "at://did:example:other/org.jason-edelman.writtenworld.commit/9"
                    .to_string(),
                cid: "bridge-cid".to_string(),
            }),
            created_at: "2".to_string(),
        };
        graph.apply_commit("test", commit).expect("provenance-only commit is valid");

        let via_target = graph
            .all_with_predicate(&vocab::via())
            .into_iter()
            .next()
            .expect("a via triple was asserted");
        assert_eq!(
            via_target.1,
            Term::NamedNode(vocab::foreign_uri_node(
                "at://did:example:test/org.jason-edelman.writtenworld.operation/1"
            ))
        );
        assert!(graph.has_type(&via_target.0, &vocab::class_commit()));

        let responds_target = graph
            .all_with_predicate(&vocab::responds_to())
            .into_iter()
            .next()
            .expect("a respondsTo triple was asserted");
        assert_eq!(
            responds_target.1,
            Term::NamedNode(vocab::foreign_uri_node(
                "at://did:example:other/org.jason-edelman.writtenworld.commit/9"
            ))
        );
    }

    /// A `Commit` whose `produces` is a single `(subject, predicate,
    /// object)` triple, using ordinary locally-minted node identities
    /// rather than `mint`'s `foreign_uri_node` addressing -- the shape a
    /// real stateful verb's own re-assertion actually takes (see
    /// `Game::take`/`Game::drop`), not a cross-repo `consumes` reference.
    fn assert_one(graph: &mut WorldGraph, subject: &NamedNode, predicate: &NamedNode, object: &NamedNode) {
        let quad = Quad::new(
            subject.clone(),
            predicate.clone(),
            Term::NamedNode(object.clone()),
            oxigraph::model::GraphName::DefaultGraph,
        );
        let commit = Commit {
            consumes: Vec::new(),
            produces: produces_text(&[quad]),
            predicate: "becomes".to_string(),
            via: None,
            responds_to: None,
            created_at: "0".to_string(),
        };
        graph
            .apply_commit("test", commit)
            .expect("a single-triple produces-only commit is always valid");
    }

    #[test]
    fn current_value_reflects_only_the_latest_generation_of_a_predicate() {
        let mut graph = WorldGraph::new();
        let item = graph.fresh("item/");
        let alice = graph.fresh("agent/");
        let bob = graph.fresh("agent/");
        let held_by = vocab::held_by();

        // Never asserted -- None, not a panic or a stale default.
        assert_eq!(graph.current_value(&item, &held_by), None);

        assert_one(&mut graph, &item, &held_by, &alice);
        assert_eq!(
            graph.current_value(&item, &held_by),
            Some(Term::NamedNode(alice.clone()))
        );

        // Re-assert the same predicate on the same subject with a new
        // object -- `apply_commit` never deletes, so the store now
        // physically holds *both* generations of this fact side by side.
        assert_one(&mut graph, &item, &held_by, &bob);

        // The store really does still have both -- proving this test
        // actually exercises "later wins" logic, not just "there's only
        // ever one fact anyway".
        assert!(
            graph.objects(&item, &held_by).len() >= 2,
            "both generations should still be physically present in the store"
        );

        // But the materialized answer is only ever the latest.
        assert_eq!(
            graph.current_value(&item, &held_by),
            Some(Term::NamedNode(bob.clone()))
        );

        // A third generation, mint-then-update-then-update-again, same
        // rule holds.
        let carol = graph.fresh("agent/");
        assert_one(&mut graph, &item, &held_by, &carol);
        assert_eq!(
            graph.current_value(&item, &held_by),
            Some(Term::NamedNode(carol))
        );
    }

    #[test]
    fn current_value_is_scoped_to_its_own_subject_and_predicate() {
        let mut graph = WorldGraph::new();
        let item_a = graph.fresh("item/");
        let item_b = graph.fresh("item/");
        let alice = graph.fresh("agent/");
        let held_by = vocab::held_by();
        let name = vocab::name();

        assert_one(&mut graph, &item_a, &held_by, &alice);

        // A different subject, untouched, stays None.
        assert_eq!(graph.current_value(&item_b, &held_by), None);
        // A different predicate on the *same* subject, also untouched.
        assert_eq!(graph.current_value(&item_a, &name), None);
    }

    #[test]
    fn current_subjects_with_finds_only_current_holders() {
        let mut graph = WorldGraph::new();
        let item = graph.fresh("item/");
        let alice = graph.fresh("agent/");
        let bob = graph.fresh("agent/");
        let held_by = vocab::held_by();

        assert_one(&mut graph, &item, &held_by, &alice);
        assert_eq!(
            graph.current_subjects_with(&held_by, &Term::NamedNode(alice.clone())),
            vec![item.clone()]
        );
        assert!(graph
            .current_subjects_with(&held_by, &Term::NamedNode(bob.clone()))
            .is_empty());

        // Re-assert to bob -- alice's generation is stale now, even though
        // it's still physically in the store.
        assert_one(&mut graph, &item, &held_by, &bob);
        assert!(
            graph
                .current_subjects_with(&held_by, &Term::NamedNode(alice))
                .is_empty(),
            "alice no longer currently holds it"
        );
        assert_eq!(
            graph.current_subjects_with(&held_by, &Term::NamedNode(bob)),
            vec![item]
        );
    }

    // -- commit_log persistence (dump_commit_log / restore_commit_log) --

    #[test]
    fn commit_log_round_trips_through_dump_and_restore() {
        let mut graph = WorldGraph::new();
        let item = graph.fresh("item/");
        let alice = graph.fresh("agent/");
        let bob = graph.fresh("agent/");
        let held_by = vocab::held_by();

        assert_one(&mut graph, &item, &held_by, &alice);
        assert_one(&mut graph, &item, &held_by, &bob);

        // Before the fix, `current_value`/`current_subjects_with` were
        // only ever readable off the live `commit_log` -- this test
        // proves the dumped text alone carries enough to reconstruct the
        // identical answer on a completely fresh graph.
        let text = graph.dump_commit_log();
        assert!(!text.is_empty(), "a graph with real apply_commit history has something to dump");

        let mut reloaded = WorldGraph::new();
        reloaded
            .restore_commit_log(&text)
            .expect("dump_commit_log's own output must always parse back");

        assert_eq!(
            reloaded.current_value(&item, &held_by),
            Some(Term::NamedNode(bob.clone())),
            "restored commit_log must answer current_value identically to the live graph"
        );
        assert_eq!(
            reloaded.current_subjects_with(&held_by, &Term::NamedNode(bob)),
            vec![item.clone()]
        );
        assert!(reloaded
            .current_subjects_with(&held_by, &Term::NamedNode(alice))
            .is_empty());
    }

    #[test]
    fn restore_commit_log_of_empty_text_is_a_harmless_no_op() {
        let mut graph = WorldGraph::new();
        graph
            .restore_commit_log("")
            .expect("an empty commit-log text is valid -- the legacy/pre-fix case");
        let item = graph.fresh("item/");
        assert_eq!(graph.current_value(&item, &vocab::held_by()), None);
    }

    #[test]
    fn restore_commit_log_rejects_malformed_text() {
        let mut graph = WorldGraph::new();
        assert!(
            graph.restore_commit_log("not a valid commit-log header\n").is_err(),
            "a line that isn't a '--- ...' header must be rejected, not silently ignored"
        );
    }

    #[test]
    fn commit_log_round_trip_agrees_with_the_live_graph_even_with_multiple_produced_quads() {
        // A single `apply_commit` call that re-asserts the same
        // (subject, predicate) pair twice in one `produces` -- unusual for
        // this crate's own verbs today, but the general `Commit` contract
        // allows it. Deliberately does *not* assert which generation wins
        // live: `Commit::produces` parses through `parse_nquads`, itself a
        // scratch-`Store` round trip whose own iteration order isn't
        // insertion order (the same caveat this module documents
        // elsewhere for the store generally), so which of two same-
        // (subject, predicate) triples in one `produces` ends up "later"
        // in `commit_log` isn't something *this* commit's ordering alone
        // determines -- a separate, pre-existing characteristic, not
        // something `dump_commit_log`/`restore_commit_log` introduce or
        // are responsible for fixing. What they *do* guarantee, and what
        // this test actually pins, is that a restored graph agrees with
        // the live one's own answer, whatever that happens to be.
        let mut graph = WorldGraph::new();
        let item = graph.fresh("item/");
        let alice = graph.fresh("agent/");
        let bob = graph.fresh("agent/");
        let held_by = vocab::held_by();

        let quads = vec![
            Quad::new(
                item.clone(),
                held_by.clone(),
                Term::NamedNode(alice),
                oxigraph::model::GraphName::DefaultGraph,
            ),
            Quad::new(
                item.clone(),
                held_by.clone(),
                Term::NamedNode(bob),
                oxigraph::model::GraphName::DefaultGraph,
            ),
        ];
        let commit = Commit {
            consumes: Vec::new(),
            produces: produces_text(&quads),
            predicate: "becomes".to_string(),
            via: None,
            responds_to: None,
            created_at: "0".to_string(),
        };
        graph
            .apply_commit("test", commit)
            .expect("a multi-triple produces-only commit is valid");
        let live_answer = graph.current_value(&item, &held_by);
        assert!(live_answer.is_some());

        let text = graph.dump_commit_log();
        let mut reloaded = WorldGraph::new();
        reloaded.restore_commit_log(&text).expect("must parse back");
        assert_eq!(
            reloaded.current_value(&item, &held_by),
            live_answer,
            "a restored graph must agree with the live graph's own current_value answer"
        );
    }


    // -- validate_self_declared / is_closed_vocabulary (PR #37 bug 1) -----
    //
    // `write_commit` (`server/src/atproto/commit_write.rs`) and `appview`'s
    // resolve-time check both call `validate_self_declared` directly, with
    // no `Store` and no access to `validate`'s dedicated per-predicate
    // branches. Before this fix, `validate_self_declared` only exempted
    // `rdf:type` from the self-declaration requirement, so a commit
    // asserting desiring-machine anatomy or any other closed-vocabulary
    // predicate `validate` itself accepts unconditionally was wrongly
    // rejected at write/resolve time with "not a recognized predicate."

    #[test]
    fn validate_self_declared_accepts_structural_glue_predicate_without_declaration() {
        // `hasRequirement` is desiring-machine anatomy, not world content --
        // `validate` (the authoritative in-memory validator) accepts it
        // unconditionally via `is_structural_glue`, no self-declaration
        // required. This is the exact bug-1 repro from the PR #37 review:
        // a room asserting `hasRequirement` a requirement node, with no
        // `rdf:type ww:Relation` declaration anywhere.
        let room = vocab::fresh("room/", 1);
        let req = vocab::fresh("requirement/", 1);
        let quads = vec![Quad::new(
            room,
            vocab::has_requirement(),
            Term::NamedNode(req),
            oxigraph::model::GraphName::DefaultGraph,
        )];
        validate_self_declared(quads.iter(), |_| HashSet::new())
            .expect("hasRequirement is closed vocabulary -- must not require self-declaration");
    }

    #[test]
    fn validate_self_declared_accepts_other_closed_vocabulary_predicates_without_declaration() {
        // A sample beyond `is_structural_glue`'s own set -- predicates
        // `validate` gives a dedicated shape/range branch to rather than
        // leaving fully unchecked, but which still never reach `validate`'s
        // self-declaration branch. `validate_self_declared` has no `Store`
        // to run those dedicated branches' own checks (and per
        // `SPEC.md`, `appview` deliberately
        // doesn't try to), but it must still skip the self-declaration
        // requirement for them the same way `validate` implicitly does.
        let node = vocab::fresh("item/", 1);
        let quads = vec![
            Quad::new(
                node.clone(),
                vocab::locked(),
                Term::Literal(lit_bool(true)),
                oxigraph::model::GraphName::DefaultGraph,
            ),
            Quad::new(
                node.clone(),
                vocab::petition_status(),
                Term::NamedNode(vocab::status_pending()),
                oxigraph::model::GraphName::DefaultGraph,
            ),
            Quad::new(
                node,
                vocab::wear(),
                Term::Literal(lit_float(0.5)),
                oxigraph::model::GraphName::DefaultGraph,
            ),
        ];
        validate_self_declared(quads.iter(), |_| HashSet::new()).expect(
            "locked/petitionStatus/wear are closed vocabulary -- none require self-declaration",
        );
    }

    #[test]
    fn validate_self_declared_still_rejects_a_genuinely_novel_undeclared_predicate() {
        // The fix must not blow the self-declaration gate wide open --
        // a predicate outside both `is_closed_vocabulary` and any
        // declaration (local or `already_declared`) must still be rejected,
        // exactly as before.
        let a = vocab::fresh("node/", 1);
        let b = vocab::fresh("node/", 2);
        let novel = vocab::dynamic_predicate("totallyMadeUp").expect("valid predicate local name");
        let quads = vec![Quad::new(
            a,
            novel,
            Term::NamedNode(b),
            oxigraph::model::GraphName::DefaultGraph,
        )];
        let err = validate_self_declared(quads.iter(), |_| HashSet::new())
            .expect_err("an undeclared novel predicate must still be rejected");
        assert!(
            matches!(err, GraphError::Invalid(_)),
            "must fail with the same self-declaration error as before this fix"
        );
    }

    #[test]
    fn validate_self_declared_still_accepts_a_properly_self_declared_novel_predicate() {
        // The positive twin of the previous test: a genuinely novel
        // predicate that *does* self-declare (here, within the same
        // `quads` batch) must still be accepted, same as before this fix.
        let a = vocab::fresh("node/", 1);
        let b = vocab::fresh("node/", 2);
        let novel = vocab::dynamic_predicate("myCustomRelation").expect("valid predicate local name");
        let quads = vec![
            Quad::new(
                novel.clone(),
                vocab::rdf_type(),
                Term::NamedNode(vocab::class_relation()),
                oxigraph::model::GraphName::DefaultGraph,
            ),
            Quad::new(
                a,
                novel,
                Term::NamedNode(b),
                oxigraph::model::GraphName::DefaultGraph,
            ),
        ];
        validate_self_declared(quads.iter(), |_| HashSet::new())
            .expect("a predicate that self-declares as Relation within the same batch is valid");
    }

    #[test]
    fn validate_self_declared_rejects_blank_node_subjects_like_validate_does() {
        // PR #37 third review round, bug 1: `validate` (the authoritative
        // in-memory validator) unconditionally rejects any quad whose
        // subject is a blank node, before any predicate-specific logic runs
        // at all -- see the "blank node subjects unsupported" guard at the
        // top of `validate`'s own loop. `validate_self_declared` had no
        // equivalent guard, so a blank-node-subject quad silently passed
        // both write-time (`commit_write.rs`) and resolve-time (`appview`)
        // validation even for closed-vocabulary predicates like `locked`,
        // which the domain model considers fundamentally malformed. Parse
        // real N-Quads with a `_:b1` blank-node subject (the same path
        // `commit_write.rs`/`appview` actually use via `parse_produces`) and
        // confirm `validate_self_declared` now rejects it the same way
        // `validate` does.
        let quads = parse_produces(&format!(
            "_:b1 <{}> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n",
            vocab::locked().as_str(),
        ))
        .expect("valid N-Quads with a blank-node subject must still parse");
        let err = validate_self_declared(quads.iter(), |_| HashSet::new())
            .expect_err("a blank-node subject must be rejected, matching validate()'s behavior");
        assert!(
            matches!(&err, GraphError::Invalid(msg) if msg == "blank node subjects unsupported"),
            "must fail with the same error validate() uses for the same case, got {err:?}"
        );
    }
}
