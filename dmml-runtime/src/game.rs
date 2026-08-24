use oxigraph::model::{NamedNode, Quad, Term};

use crate::command::{parse, HELP_TEXT};
use crate::commune;
use crate::demiurge;
use crate::direction::Direction;
use crate::graph::{
    as_bool, as_float, as_int, as_node, as_string, lit_bool, lit_float, lit_int, lit_str, Commit,
    Delta, WorldGraph,
};
use crate::machine::{self, Effect, Requirement};
use crate::render;
use crate::vocab;

pub struct Game {
    graph: WorldGraph,
    player: NamedNode,
    world_seed: u64,
    /// See `Game::reconstruction_gap`'s own doc comment. Deliberately a
    /// plain field, not a fact asserted into `graph` -- an earlier version
    /// of this fix recorded it as a real commit, which corrupted
    /// `content_hash` (and so `WorldGraph::fresh`'s collision-freedom
    /// guarantee) for a `Game` that's otherwise a faithful reconstruction,
    /// confirmed by `reconstructing_a_game_from_world_seed_plus_post_
    /// genesis_commits_matches_the_live_session` failing once that version
    /// landed. "Corruption as content" (see the README's own section on
    /// this, and `vocab::class_drift`/`Game::record_foreign_drift` for the
    /// original instance of the same principle) still applies to the
    /// *player-facing* honesty this exists for -- a caller can still see
    /// and act on the gap -- it just can't be a graph-level fact without
    /// also being a state-divergence bug.
    reconstruction_gap: Option<String>,
}

/// A ready-to-send command, with a player-facing label. `available_actions`
/// is the same "what can I do" affordance query the text dispatch already
/// runs -- exposed directly, so a button UI reads it off the graph instead
/// of re-deriving structure by parsing rendered prose back apart, which
/// would be a step backward from everything else this design does.
pub struct Action {
    pub label: String,
    pub command: String,
}

/// A room's correspondence with a record on someone else's atproto PDS --
/// see `Game::reach`/`foreign_link`/`record_foreign_drift` and the
/// README's "Corruption as content" section.
pub struct ForeignLink {
    pub uri: String,
    pub cid: Option<String>,
    pub snapshot: Option<String>,
}

/// Everything a caller needs to persist a `Game` and rebuild an identical
/// one later, and nothing more -- deliberately opaque beyond these three
/// fields so a persistence layer (a Durable Object, say) never needs to
/// know `Game`'s internal shape, only "store this, and hand it back to
/// `from_snapshot` later." `player` isn't here: it's re-derived on load
/// the same way it's derived live, by querying the graph for its one
/// Player-typed node, rather than persisted as a reference that could
/// drift from what the graph itself says is true. The transcript isn't
/// here either -- seeing `WorldGraph::load_nquads`'s doc comment for why
/// that's an accepted limitation, not an oversight.
///
/// `nquads` is no longer *only* an N-Quads dump, despite the name: it also
/// opaquely carries `WorldGraph::dump_commit_log`'s output, framed by
/// `encode_snapshot_blob`/`decode_snapshot_blob` below -- see those doc
/// comments for why. This field stays named `nquads` and typed `Vec<u8>`
/// (rather than gaining a fourth field) specifically so existing callers
/// outside this crate (`server`, `client`, both outside `engine`'s own
/// workspace and neither entitled to a breaking-change here) that already
/// treat this field as an opaque blob -- store it, hand it back, never
/// parse it themselves -- keep compiling and keep working unmodified, old
/// stored snapshots included (see `decode_snapshot_blob`'s legacy-format
/// fallback).
pub struct Snapshot {
    pub nquads: Vec<u8>,
    pub content_hash: u64,
    pub world_seed: u64,
}

/// The 4-byte tag `encode_snapshot_blob` prefixes onto a snapshot's
/// `nquads` field to mark "this blob also has a framed commit-log section
/// appended" -- chosen because a real `dump_nquads()` output never starts
/// with these bytes (valid N-Quads text starts with `<` or `_:`), so its
/// presence unambiguously distinguishes the new framed format from a
/// legacy pre-fix snapshot's bare N-Quads bytes without needing to search
/// blob content for a delimiter (which a delimiter chosen from printable
/// text could theoretically collide with; a fixed-position magic prefix
/// can't).
const SNAPSHOT_BLOB_MAGIC: &[u8; 4] = b"WWCL";

/// Combines a `dump_nquads()` blob and a `dump_commit_log()` text into the
/// single opaque byte blob `Snapshot.nquads` actually stores -- see that
/// field's own doc comment for why they're combined rather than living in
/// their own `Snapshot` fields. Layout: 4-byte magic, 8-byte little-endian
/// length of the N-Quads section, the N-Quads bytes themselves, then the
/// commit-log text verbatim (its own length is just "whatever's left").
fn encode_snapshot_blob(nquads: &[u8], commit_log: &str) -> Vec<u8> {
    let mut blob = Vec::with_capacity(4 + 8 + nquads.len() + commit_log.len());
    blob.extend_from_slice(SNAPSHOT_BLOB_MAGIC);
    blob.extend_from_slice(&(nquads.len() as u64).to_le_bytes());
    blob.extend_from_slice(nquads);
    blob.extend_from_slice(commit_log.as_bytes());
    blob
}

/// The inverse of `encode_snapshot_blob`. A blob that doesn't start with
/// `SNAPSHOT_BLOB_MAGIC` -- too short to even hold the header, or simply a
/// different prefix -- is treated as a legacy, pre-fix snapshot: its
/// entire content is the N-Quads section, and the commit-log section is
/// empty (`WorldGraph::restore_commit_log("")` is a no-op, leaving
/// `commit_log` empty exactly the way a `load_nquads`-built graph always
/// used to be). This is a real, accepted limitation for snapshots taken
/// before this fix existed -- their `heldBy`/`locatedIn` state was never
/// captured to begin with, so there's nothing to restore -- rather than an
/// error that would strand an existing session unable to load at all.
fn decode_snapshot_blob(blob: &[u8]) -> Result<(&[u8], &str), String> {
    const HEADER_LEN: usize = 4 + 8;
    if blob.len() < HEADER_LEN || &blob[0..4] != SNAPSHOT_BLOB_MAGIC {
        return Ok((blob, ""));
    }
    let len_bytes: [u8; 8] = blob[4..HEADER_LEN]
        .try_into()
        .expect("slice of exactly 8 bytes");
    let nquads_len = u64::from_le_bytes(len_bytes) as usize;
    let nquads_end = HEADER_LEN
        .checked_add(nquads_len)
        .ok_or("corrupt snapshot: commit-log framing length overflows")?;
    if nquads_end > blob.len() {
        return Err(
            "corrupt snapshot: commit-log framing length exceeds blob size".to_string(),
        );
    }
    let nquads = &blob[HEADER_LEN..nquads_end];
    let commit_log = std::str::from_utf8(&blob[nquads_end..])
        .map_err(|e| format!("corrupt snapshot: commit-log section is not valid UTF-8: {e}"))?;
    Ok((nquads, commit_log))
}

/// The fixed list of predicates only ever produced by `WorldGraph::
/// apply_commit`, never by the old `Delta`/`WorldGraph::commit` path --
/// `heldBy`/`locatedIn` (`take`/`drop`/`go`) and the DMML petition state
/// machine's own (`dmmlPetitionStatus`/`repliesTo`/`petitionReplyContent`,
/// see `commune.rs`'s "DMML petition state machine" doc comment). Kept as
/// one list, hand-maintained rather than derived, because `apply_commit`
/// itself has no registry of "predicates it produces" to introspect --
/// same honest hand-maintained-list posture `machine.rs`'s `Effect`/
/// `Requirement` kind strings already have. Used by `Game::replay_commit`
/// to detect (not prevent -- see its own doc comment) when a purely
/// replay-reconstructed `Game` may be missing state only `apply_commit`'s
/// `commit_log` bookkeeping can materialize.
fn at_risk_predicate(p: &NamedNode) -> Option<&'static NamedNode> {
    use std::sync::OnceLock;
    static AT_RISK: OnceLock<Vec<NamedNode>> = OnceLock::new();
    AT_RISK
        .get_or_init(|| {
            vec![
                vocab::held_by(),
                vocab::located_in(),
                vocab::dmml_petition_status(),
                vocab::replies_to(),
                vocab::petition_reply_content(),
            ]
        })
        .iter()
        .find(|candidate| *candidate == p)
}

impl Game {
    pub fn new(world_seed: u64) -> Self {
        let mut graph = WorldGraph::new();
        let boot = demiurge::bootstrap(&mut graph);

        Game {
            graph,
            player: boot.player,
            world_seed,
            reconstruction_gap: None,
        }
    }

    /// The persistable half of a `Game` -- see `Snapshot`'s doc comment.
    /// Carries `WorldGraph::dump_commit_log`'s output alongside the plain
    /// N-Quads dump (see `encode_snapshot_blob`) so `heldBy`/`locatedIn`
    /// -- and any other `apply_commit`-sourced state -- survive the round
    /// trip through `from_snapshot`, not just what the older `Delta`/
    /// `commit` path wrote.
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let nquads = self.graph.dump_nquads().map_err(|e| e.to_string())?;
        let commit_log = self.graph.dump_commit_log();
        Ok(Snapshot {
            nquads: encode_snapshot_blob(&nquads, &commit_log),
            content_hash: self.graph.content_hash(),
            world_seed: self.world_seed,
        })
    }

    /// The other half: rebuilds a `Game` from a `Snapshot` a prior call to
    /// `snapshot` produced. Always starts with `reconstruction_gap: None`
    /// -- a stated tradeoff, not an accident: `Snapshot` has no field for
    /// it (see that struct's own doc comment for why it stays deliberately
    /// narrow), so a `Game` that *had* a recorded gap loses that specific
    /// fact across a snapshot round trip. Not a regression against #26's
    /// actual scope (replay-from-genesis reconstruction, which this
    /// doesn't touch) -- just a real, narrower gap than "never loses a
    /// gap marker," worth a future `Snapshot` field if it ever matters in
    /// practice. Fails only if the stored data is corrupt or
    /// doesn't contain the one Player-typed node every valid `Game` has --
    /// conditions that should never arise from a snapshot this crate
    /// itself produced, but the caller (persistence layer) is trusting
    /// external storage, not just memory, so this stays fallible rather
    /// than panicking. Also calls `demiurge::ensure_sense_machines`, a
    /// no-op for any session bootstrapped under the current code but a
    /// real fix for one that predates the percept pipeline -- see that
    /// function's own doc comment for why a returning player's world
    /// otherwise renders as "You perceive nothing" forever, world and
    /// player both perfectly intact, just missing an organ that didn't
    /// exist yet when they were first equipped.
    ///
    /// Unpacks `snapshot.nquads` via `decode_snapshot_blob` first: a
    /// snapshot produced by the current `snapshot()` carries a framed
    /// commit-log section restored via `WorldGraph::restore_commit_log`
    /// right after `load_nquads`, so `current_value`/`current_subjects_with`
    /// (and therefore `held_items`/`player_location_via_located_in`)
    /// answer the same way they did before the snapshot was taken. A
    /// legacy, pre-fix snapshot decodes with an empty commit-log section
    /// and loads exactly as it always did -- see `decode_snapshot_blob`'s
    /// own doc comment.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self, String> {
        let (nquads, commit_log) = decode_snapshot_blob(&snapshot.nquads)?;
        let mut graph =
            WorldGraph::load_nquads(snapshot.content_hash, nquads).map_err(|e| e.to_string())?;
        graph
            .restore_commit_log(commit_log)
            .map_err(|e| e.to_string())?;
        let player = graph
            .subjects(&vocab::rdf_type(), &Term::NamedNode(vocab::class_player()))
            .into_iter()
            .next()
            .ok_or("no Player-typed node found in the loaded graph")?;
        demiurge::ensure_sense_machines(&mut graph, &player);
        Ok(Game {
            graph,
            player,
            world_seed: snapshot.world_seed,
            reconstruction_gap: None,
        })
    }

    /// Delegates to `WorldGraph::set_now` -- see its doc comment. A caller
    /// (server, cli, client) invokes this with its own real wall-clock
    /// reading before any command that might commit, since `engine` has no
    /// clock of its own to read.
    pub fn set_now(&mut self, now_ms: u64) {
        self.graph.set_now(now_ms);
    }

    /// The seed this `Game` was constructed with -- what a caller
    /// reconstructing a world from a commit-log replay needs in order to
    /// regenerate an identical genesis via `Game::new` before replaying
    /// anything past it (see `Game::replay_commit`'s doc comment for why
    /// genesis itself is regenerated rather than replayed).
    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    /// Applies one already-committed delta, verbatim, as `source` --
    /// `text` is a `TranscriptEntry::canonical_text()` rendering (or
    /// anything `Delta::from_canonical_text` accepts), the same shape a
    /// commit-signing record's `delta` field carries. This is the one
    /// entry point a caller reconstructing a `Game` from an external
    /// commit log (a player's own PDS, replayed) uses to apply each
    /// record in turn, after regenerating genesis with `Game::new` --
    /// genesis itself is never replayed through here, since it's fully
    /// deterministic from `world_seed` alone and a caller can always
    /// regenerate it directly rather than needing genesis's own commits
    /// to literally exist in whatever log is being replayed. Still goes
    /// through the identical `WorldGraph::commit` validation path every
    /// other write does -- replaying a tampered or malformed record fails
    /// exactly like a live one would, which is the actual point of a
    /// caller replaying a log through `engine` rather than trusting it
    /// blindly. `now_ms` is stamped via `set_now` first, so
    /// `graph::creation_order` reflects the record's own original
    /// timestamp rather than whatever this call happens to run at.
    pub fn replay_commit(&mut self, source: &str, text: &str, now_ms: u64) -> Result<(), String> {
        let delta = Delta::from_canonical_text(text).map_err(|e| e.to_string())?;
        // Checked *before* `delta` is moved into `commit` below (`Delta`
        // isn't `Clone`) -- see `at_risk_predicate`'s own doc comment for
        // what this is actually detecting and why it's a report, not a
        // fix. `.cloned()` here is just `Option<&NamedNode> ->
        // Option<NamedNode>`, not a `Delta` clone.
        let at_risk = delta.add.iter().find_map(|q| at_risk_predicate(&q.predicate)).cloned();
        self.graph.set_now(now_ms);
        self.graph.commit(source, delta).map_err(|e| e.to_string())?;
        if let Some(predicate) = at_risk {
            self.record_reconstruction_gap(&predicate);
        }
        Ok(())
    }

    /// Records, once per `Game`, that this reconstruction may be missing
    /// material facts -- the "corruption as content" answer to #26 (see
    /// `Game::reconstruction_gap`'s own doc comment for why this lives as
    /// a plain field, not a graph fact). Deliberately does *not* try to
    /// reconstruct the missing state itself: `heldBy`/`locatedIn`/the DMML
    /// petition predicates stay exactly as blank as they already were on a
    /// replay-only reconstruction (see `replay_commit`'s own doc comment
    /// for why `commit_log` can't be rebuilt from a transcript alone) --
    /// chasing that would be the "every exception to 'everything is DMML'
    /// bites us" trap this fix is explicitly not taking. What changes is
    /// that the gap is now a real, inspectable fact about *this session*
    /// instead of a silent absence.
    fn record_reconstruction_gap(&mut self, predicate: &NamedNode) {
        if self.reconstruction_gap.is_none() {
            self.reconstruction_gap = Some(crate::graph::short(predicate));
        }
    }

    /// Whether this `Game` has a recorded reconstruction gap -- what a
    /// caller (a render pass, a client warning banner) checks before
    /// trusting `heldBy`/`locatedIn`/DMML-petition-derived state on a
    /// possibly-replay-only reconstruction. `None` on any ordinary live
    /// session or `Game::from_snapshot` round trip -- only ever set by
    /// `replay_commit`, and cleared by nothing (a gap, once real, stays
    /// true for the rest of this `Game`'s life; nothing here repairs it).
    pub fn reconstruction_gap(&self) -> Option<String> {
        self.reconstruction_gap.clone()
    }

    pub fn player_room(&self) -> NamedNode {
        self.graph
            .subjects(&vocab::contains(), &Term::NamedNode(self.player.clone()))
            .into_iter()
            .next()
            .expect("player is always in exactly one room")
    }

    pub fn look(&self) -> String {
        let room = self.player_room();
        render::render_room_text(&self.graph, &self.player, &room, &self.unexplored_from(&room))
    }

    /// The map, rendered the same way `look()` renders the room -- a
    /// caller (the "map" verb, and `server`'s `current_view` for the
    /// persistent panel) doesn't need to know it's a percept underneath.
    pub fn map(&self) -> String {
        match render::perceive_map(&self.graph, &self.player) {
            Some(p) => render::render_percept_text(&p),
            None => "You have no way to perceive that.".to_string(),
        }
    }

    /// The transcript's current length -- a caller marks this before
    /// running a command and passes it to `player_commits_since` afterward
    /// to see exactly what that one call committed. See `WorldGraph::
    /// transcript_since`'s doc comment for why a mark beats tracked state.
    pub fn transcript_len(&self) -> u64 {
        self.graph.transcript().len() as u64
    }

    /// Every commit at or after `since`, regardless of source: each one's
    /// ordinal position, a deterministic text a caller can hash (or, per
    /// the pantheon design, write verbatim as a replayable record) for a
    /// stable commit id, and who proposed it. Used to be two separate,
    /// source-filtered methods (`player_commits_since`/
    /// `demiurge_commits_since`) that each fed a different signing
    /// identity -- the player's own PDS via a browser session, a separate
    /// server-held "demiurge" identity via an app password. That split
    /// stopped making sense once the pantheon design retired the demiurge
    /// as a privileged, separately-identified role: there's one player,
    /// one PDS, and the gate is meant to be indifferent to provenance as
    /// long as the chain into that PDS is unbroken (see
    /// `graph::creation_order`'s doc comment for the timestamp half of
    /// that same invariant). `source` is what still carries "who proposed
    /// it" -- now recorded on the written record itself rather than
    /// implied by which identity signed it. Stays offline like the rest of
    /// this crate -- hashing and signing are a caller's job (see
    /// `server/src/pds_commits`), not this one's.
    pub fn commits_since(&self, since: u64) -> Vec<(u64, String, String)> {
        self.graph
            .transcript_since(since)
            .iter()
            .map(|e| (e.seq, e.canonical_text(), e.source.clone()))
            .collect()
    }

    /// A direction is unexplored exactly when `room` has no committed edge
    /// in that direction yet -- derived from the graph itself rather than
    /// tracked as separate mutable state, so there's nothing here that
    /// could drift out of sync with what's actually committed, and nothing
    /// extra to persist alongside a graph snapshot.
    fn unexplored_from(&self, room: &NamedNode) -> Vec<Direction> {
        Direction::ALL
            .into_iter()
            .filter(|d| self.edge_towards(room, *d).is_none())
            .collect()
    }

    /// The JSON body a caller should POST to `/api/commune`: the player's
    /// current room's facts plus the world's self-declared relation
    /// vocabulary so far. Doesn't touch the graph or network itself --
    /// this crate stays offline; a frontend (the web crate) is what
    /// actually makes the request and hands the response to
    /// `apply_commune_delta`.
    pub fn commune_context(&self) -> String {
        commune::build_context(&self.graph, &self.player_room())
    }

    /// Parses and commits a `/api/commune` response against the room the
    /// player is currently standing in, tagged "demiurge-ai" in the
    /// transcript so it's distinguishable from the deterministic local
    /// demiurge's own commits. Goes through the exact same
    /// `WorldGraph::commit` gate as everything else -- a malformed or
    /// rule-breaking proposal returns `Err` and changes nothing.
    pub fn apply_commune_delta(&mut self, json: &str) -> Result<String, String> {
        let room = self.player_room();
        let delta = commune::parse_commune_delta(&mut self.graph, &room, json)?;
        self.graph
            .commit("demiurge-ai", delta)
            .map_err(|e| e.to_string())?;
        Ok(render::render_room_text(
            &self.graph,
            &self.player,
            &room,
            &self.unexplored_from(&room),
        ))
    }

    /// Links the room the player is currently standing in to a foreign
    /// atproto record by its stable `at://` URI -- an act of declaring
    /// "this place is also that place," not a fetch (this crate has no
    /// I/O; a server-side caller does the actual fetching -- see
    /// `server::foreign_room` and the README's "Corruption as content"
    /// section). Overwrites any prior link this room had: a room
    /// corresponds to at most one foreign record at a time. The cached CID
    /// starts unset and is populated by the first `record_foreign_drift`
    /// call after a caller fetches it.
    pub fn reach(&mut self, at_uri: &str) -> Result<String, String> {
        let room = self.player_room();
        let uri_node = vocab::foreign_uri_node(at_uri);
        let mut d = Delta::new();
        if let Some(old) = self.graph.object(&room, &vocab::foreign_uri()) {
            d = d.retract(room.clone(), vocab::foreign_uri(), old);
        }
        if let Some(old_cid) = self.graph.object(&room, &vocab::foreign_cid()) {
            d = d.retract(room.clone(), vocab::foreign_cid(), old_cid);
        }
        d = d.assert(room, vocab::foreign_uri(), uri_node);
        self.graph.commit("player", d).map_err(|e| e.to_string())?;
        Ok(format!(
            "You reach outward -- this place answers to {at_uri} now, too."
        ))
    }

    /// The current room's foreign correspondence, if any. `cid`/`snapshot`
    /// are both `None` until a caller has fetched and recorded at least
    /// once via `record_foreign_drift`. A server-side caller uses `uri` to
    /// know what to fetch, `cid` to know whether a fresh fetch's CID counts
    /// as drift, and `snapshot` as the "before" half of the narrator's
    /// diff -- see `record_foreign_drift`.
    pub fn foreign_link(&self) -> Option<ForeignLink> {
        let room = self.player_room();
        let uri = self
            .graph
            .object(&room, &vocab::foreign_uri())
            .and_then(as_node)
            .and_then(|n| vocab::foreign_uri_from_node(&n))?;
        let cid = self
            .graph
            .object(&room, &vocab::foreign_cid())
            .and_then(as_node)
            .and_then(|n| vocab::foreign_cid_from_node(&n));
        let snapshot = self
            .graph
            .object(&room, &vocab::foreign_snapshot())
            .and_then(|t| as_string(&t));
        Some(ForeignLink { uri, cid, snapshot })
    }

    /// Records a fresh fetch's outcome for the current room's foreign
    /// link: updates the cached CID and content snapshot, and -- if there
    /// was a prior CID to compare against and it actually changed -- mints
    /// a `Drift` node (old CID, new CID, when observed) and accretes it as
    /// `noticedChange`. No narration is generated or stored; a `Drift`
    /// node is a structured record that something changed, not a claim
    /// about what. The first-ever observation (no prior CID cached) is a
    /// baseline, not a drift -- nothing to compare it against yet. Never
    /// touches any other fact about the room: ground truth is never
    /// rewritten to match the foreign source. Tagged `"narrator"` in the
    /// transcript -- distinct from `"player"`/`"demiurge"`/`"demiurge-ai"`,
    /// since this content didn't originate from anything this world's own
    /// generative agencies proposed, it's a report about somewhere else. A
    /// no-op if the current room has no foreign link at all (nothing to
    /// record against).
    pub fn record_foreign_drift(
        &mut self,
        new_cid: &str,
        new_snapshot: &str,
        observed_at_ms: u64,
    ) -> Result<(), String> {
        let room = self.player_room();
        if self.graph.object(&room, &vocab::foreign_uri()).is_none() {
            return Ok(());
        }
        let new_cid_node = vocab::foreign_cid_node(new_cid);
        let old_cid_node = self.graph.object(&room, &vocab::foreign_cid()).and_then(as_node);

        let mut d = Delta::new();
        if let Some(old) = &old_cid_node {
            d = d.retract(room.clone(), vocab::foreign_cid(), old.clone());
        }
        d = d.assert(room.clone(), vocab::foreign_cid(), new_cid_node.clone());
        if let Some(old_snapshot) = self.graph.object(&room, &vocab::foreign_snapshot()) {
            d = d.retract(room.clone(), vocab::foreign_snapshot(), old_snapshot);
        }
        d = d.assert(
            room.clone(),
            vocab::foreign_snapshot(),
            lit_str(new_snapshot.to_string()),
        );

        if let Some(old) = old_cid_node {
            if old != new_cid_node {
                let drift = self.graph.fresh("drift/");
                d = d
                    .assert(drift.clone(), vocab::rdf_type(), vocab::class_drift())
                    .assert(drift.clone(), vocab::drift_old_cid(), old)
                    .assert(drift.clone(), vocab::drift_new_cid(), new_cid_node)
                    .assert(
                        drift.clone(),
                        vocab::drift_observed_at(),
                        lit_int(observed_at_ms),
                    )
                    .assert(room, vocab::noticed_change(), drift);
            }
        }
        self.graph.commit("narrator", d).map_err(|e| e.to_string())
    }

    /// Raises a petition against the room the player is currently standing
    /// in and commits it immediately -- instant, no AI call, no dispatch in
    /// this path. `now_ms` is a wall-clock timestamp the caller supplies
    /// (this crate has no clock of its own); the petition expires
    /// `commune::DEFAULT_PETITION_TTL_MS` after that if nobody answers it.
    /// Returns the petition's own id so a caller (the DO's `/commune`
    /// route) can report it and schedule a dispatch.
    pub fn raise_petition_for_current_room(&mut self, now_ms: u64) -> NamedNode {
        self.graph.set_now(now_ms);
        let room = self.player_room();
        let (delta, petition) = commune::raise_petition(
            &mut self.graph,
            &room,
            now_ms,
            commune::DEFAULT_PETITION_TTL_MS,
        );
        self.graph
            .commit("player", delta)
            .expect("raising a petition is always valid");
        petition
    }

    /// Equips a durable, graph-visible marker onto the player's own node
    /// recording that `operator` has been let in -- the mechanical half of
    /// redeeming an invite ("invite as pentacle",
    /// [jedelman/written-world#8](https://github.com/jedelman/written-world/issues/8)):
    /// "casting a circle" extends what the player (and whoever they've
    /// invited in) can do, and that extension should be something the
    /// player's own world knows about, not just an HTTP credential this
    /// app's server checks silently. `operator` becomes the `source` tag
    /// on the commit this produces, same as `"player"`/`"demiurge"`/
    /// `"external-resolver"` elsewhere -- an invited operator's own
    /// identifier, not a generic label.
    ///
    /// Uses `vocab::operator_label` as a self-declared `Attribute` (see
    /// `graph::validate`'s novel-predicate handling) rather than a new
    /// `Effect`/`Requirement` kind: an invite's grant is content the
    /// pantheon's own graph should express, not new mechanics this crate
    /// needs to special-case.
    pub fn equip_operator(&mut self, operator: &str, now_ms: u64) -> Result<(), String> {
        let machine = self.graph.fresh("machine/");
        let delta = Delta::new()
            .assert(
                vocab::operator_label(),
                vocab::rdf_type(),
                vocab::class_attribute(),
            )
            .assert(machine.clone(), vocab::rdf_type(), vocab::class_machine())
            .assert(self.player.clone(), vocab::equips(), machine.clone())
            .assert(machine, vocab::operator_label(), lit_str(operator));
        self.graph.set_now(now_ms);
        self.graph
            .commit(operator, delta)
            .map_err(|e| e.to_string())
    }

    /// Every petition still awaiting resolution, oldest first -- the
    /// dispatch's work queue, and what an external resolver lists.
    pub fn pending_petitions(&self) -> Vec<NamedNode> {
        commune::pending_petitions(&self.graph)
    }

    /// Retires every pending petition whose TTL has passed into
    /// `"expired"` -- see `commune::expire_stale_petitions`. Returns the
    /// ids that expired, so a caller (the dispatch) knows not to notify
    /// subscribers about them.
    pub fn expire_stale_petitions(&mut self, now_ms: u64) -> Vec<NamedNode> {
        let (delta, expired) = commune::expire_stale_petitions(&self.graph, now_ms);
        if !expired.is_empty() {
            self.graph.set_now(now_ms);
            self.graph
                .commit("petition-queue", delta)
                .expect("expiring a petition is always valid");
        }
        expired
    }

    /// The frozen context a petition was raised with -- what a resolver
    /// (in-process or external) needs to actually answer it. Reconstructed
    /// from the petition's `PetitionSnapshot` node (real triples, see
    /// `commune::freeze_room_snapshot`) rather than read off a stored JSON
    /// literal -- see #15. The returned JSON shape is unchanged, so every
    /// existing caller of this method needed no changes.
    pub fn petition_context(&self, petition: &NamedNode) -> Option<String> {
        let snapshot = self
            .graph
            .object(petition, &vocab::petition_context())
            .and_then(as_node)?;
        commune::context_from_snapshot(&self.graph, &snapshot)
    }

    /// Resolves `petition` with `json` (the same wire shape `/api/commune`'s
    /// AI response has always used) and commits it. `source` tags the
    /// transcript with who answered -- "demiurge-ai" for the built-in
    /// Workers AI responder, or whatever an external resolver identifies
    /// itself as; the queue itself doesn't privilege one kind of client
    /// over another, so this isn't hardcoded. On success, the petitioned
    /// room's freshly rendered prose is both returned to the caller and
    /// stashed on the petition itself as `petitionResult`, so a client
    /// that isn't the one polling right now still picks it up on its next
    /// ordinary fetch. Two commits, not one -- the content+status flip
    /// first (so a validator rejection leaves the petition exactly as
    /// pending as it started, per `resolve_petition_delta`'s contract),
    /// then the result text, which needs the graph already updated to
    /// render correctly and so can't be folded into the first commit.
    /// `now_ms` is a wall-clock timestamp the caller supplies (this crate
    /// has no clock of its own) and gets stamped on both commits via
    /// `WorldGraph::set_now` -- resolution can land well after the petition
    /// was raised (an AI call, a human-in-the-loop external resolver), so
    /// it needs its own fresh reading rather than reusing whatever the
    /// graph's clock last held.
    pub fn resolve_petition(
        &mut self,
        petition: &NamedNode,
        json: &str,
        source: &str,
        now_ms: u64,
    ) -> Result<String, String> {
        self.graph.set_now(now_ms);
        let room = self
            .graph
            .object(petition, &vocab::petition_concerns())
            .and_then(as_node)
            .ok_or("petition has no concerns room")?;

        let delta = commune::resolve_petition_delta(&mut self.graph, petition, json)?;
        self.graph
            .commit(source, delta)
            .map_err(|e| e.to_string())?;

        let result_text =
            render::render_room_text(&self.graph, &self.player, &room, &self.unexplored_from(&room));
        let d2 = Delta::new().assert(
            petition.clone(),
            vocab::petition_result(),
            lit_str(result_text.clone()),
        );
        self.graph
            .commit(source, d2)
            .expect("stashing a petition's result text is always valid");

        Ok(result_text)
    }

    /// The DMML-shaped petition flow's raise step -- see
    /// `commune::raise_petition_dmml`'s own doc comment. Additive
    /// alongside `raise_petition_for_current_room` above, not a
    /// replacement: nothing outside `engine/` calls this yet (`server/`,
    /// `mcp-server/`, and `client/` all still use the mechanism above),
    /// so wiring it into any of those is separate, follow-up work once
    /// this shape's been verified.
    pub fn raise_petition_dmml_for_current_room(&mut self, now_ms: u64) -> Result<NamedNode, String> {
        let room = self.player_room();
        commune::raise_petition_dmml(
            &mut self.graph,
            &room,
            now_ms,
            commune::DEFAULT_PETITION_TTL_MS,
        )
    }

    /// The DMML flow's reply step -- see `commune::reply_petition_dmml`'s
    /// own doc comment. `source` tags the transcript the same way
    /// `resolve_petition`'s does (a real implementation would be the
    /// target DID's own identifier; engine-side this is just a label).
    pub fn reply_petition_dmml(
        &mut self,
        petition: &NamedNode,
        json: &str,
        source: &str,
    ) -> Result<NamedNode, String> {
        commune::reply_petition_dmml(&mut self.graph, petition, json, source)
    }

    /// The DMML flow's accept step -- see `commune::accept_petition_dmml`'s
    /// own doc comment. Renders and stashes the result text the same way
    /// `resolve_petition` does, once the reply's content has actually
    /// landed in the graph.
    pub fn accept_petition_dmml(
        &mut self,
        petition: &NamedNode,
        now_ms: u64,
    ) -> Result<String, String> {
        let room = self
            .graph
            .object(petition, &vocab::petition_concerns())
            .and_then(as_node)
            .ok_or("petition has no concerns room")?;
        commune::accept_petition_dmml(&mut self.graph, petition, now_ms)?;

        let result_text =
            render::render_room_text(&self.graph, &self.player, &room, &self.unexplored_from(&room));
        let d2 = Delta::new().assert(
            petition.clone(),
            vocab::petition_result(),
            lit_str(result_text.clone()),
        );
        self.graph
            .commit("player", d2)
            .expect("stashing a petition's result text is always valid");

        Ok(result_text)
    }

    /// Every currently sensible command from where the player stands right
    /// now: exits (explored, annotated if sealed; unexplored, offered as a
    /// way to trigger generation), and per-item verbs -- examine and take
    /// always where applicable, plus whatever a machine equipped to that
    /// item currently advertises and would actually fire (requirements
    /// checked, same as `fire_object_verb` checks them).
    pub fn available_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        let room = self.player_room();

        for edge in self
            .graph
            .objects(&room, &vocab::connects_to())
            .into_iter()
            .filter_map(as_node)
        {
            let Some(dir) = self
                .graph
                .object(&edge, &vocab::direction())
                .and_then(|t| as_string(&t))
            else {
                continue;
            };
            let locked = self
                .graph
                .object(&edge, &vocab::locked())
                .and_then(|t| as_bool(&t))
                .unwrap_or(false);
            let label = if locked {
                format!("{dir} (sealed)")
            } else {
                dir.clone()
            };
            actions.push(Action {
                label,
                command: format!("go {dir}"),
            });
        }
        for dir in self.unexplored_from(&room) {
            actions.push(Action {
                label: format!("{} (unknown)", dir.word()),
                command: format!("go {}", dir.word()),
            });
        }

        // Sorted by creation_order before iterating -- same fix, same
        // reasoning as `render::perceive_room`'s item listing (store
        // iteration order isn't stable across a snapshot round trip).
        let mut room_items: Vec<NamedNode> = self
            .graph
            .objects(&room, &vocab::contains())
            .into_iter()
            .filter_map(as_node)
            .collect();
        room_items.sort_by_key(|n| crate::graph::creation_order(&self.graph, n));
        for item in room_items {
            if !self.graph.has_type(&item, &vocab::class_item()) {
                continue;
            }
            let name = render::display_name(&self.graph, &item);
            actions.push(Action {
                label: format!("examine {name}"),
                command: format!("examine {name}"),
            });
            if self
                .graph
                .object(&item, &vocab::portable())
                .and_then(|t| as_bool(&t))
                .unwrap_or(false)
            {
                actions.push(Action {
                    label: format!("take {name}"),
                    command: format!("take {name}"),
                });
            }
            for verb in self.verbs_available_on(&item, &room) {
                actions.push(Action {
                    label: format!("{verb} {name}"),
                    command: format!("{verb} {name}"),
                });
            }
        }

        for item in self.held_items() {
            let name = render::display_name(&self.graph, &item);
            actions.push(Action {
                label: format!("drop {name}"),
                command: format!("drop {name}"),
            });
        }

        actions
    }

    /// Distinct trigger verbs from machines equipped to `item` whose
    /// requirements currently hold -- a lever's drift and threshold
    /// machines share the "pull" trigger, so this collapses to one button,
    /// not two.
    fn verbs_available_on(&self, item: &NamedNode, room: &NamedNode) -> Vec<String> {
        let mut verbs = std::collections::BTreeSet::new();
        for m in self
            .graph
            .objects(item, &vocab::equips())
            .into_iter()
            .filter_map(as_node)
        {
            let requirements: Vec<Requirement> = self
                .graph
                .objects(&m, &vocab::has_requirement())
                .into_iter()
                .filter_map(as_node)
                .filter_map(|r| machine::read_requirement(&self.graph, &r))
                .collect();
            if !requirements
                .iter()
                .all(|r| machine::requirement_met(&self.graph, r, room))
            {
                continue;
            }
            for t in self.graph.objects(&m, &vocab::trigger()) {
                if let Some(v) = as_string(&t) {
                    verbs.insert(v);
                }
            }
        }
        verbs.into_iter().collect()
    }

    pub fn handle(&mut self, input: &str) -> String {
        let cmd = parse(input);
        match cmd.verb.as_str() {
            "" => "Say something.".to_string(),
            "help" | "?" => HELP_TEXT.to_string(),
            "quit" | "exit" => "Goodbye.".to_string(),
            "assemblage" => {
                let target = if cmd.object.is_empty() {
                    self.player_room()
                } else {
                    let room = self.player_room();
                    self.find_by_name(&room, &cmd.object).unwrap_or(room)
                };
                render::render_assemblage(&self.graph, &target)
            }
            "kinds" => render::render_kinds(&self.graph),
            "relations" => render::render_relations(&self.graph),
            "map" => self.map(),
            "transcript" => self.graph.render_transcript(),
            // Case note: `command::parse` lowercases its whole input, so a
            // mixed-case at:// URI arrives lowercased here. Left as-is
            // rather than special-casing this one verb's parsing: both a
            // did:plc identifier and a TID record key are lowercase by
            // their own spec, so a real at:// URI survives this
            // unscathed in practice -- see the README's "Corruption as
            // content" section for the one place this could still bite
            // (a custom, mixed-case rkey).
            "reach" => match self.reach(&cmd.object) {
                Ok(msg) => msg,
                Err(e) => format!("That doesn't reach anywhere. ({e})"),
            },
            "look" => self.look(),
            "go" => self.go(&cmd.object),
            "take" => self.take(&cmd.object),
            "drop" => self.drop(&cmd.object),
            "conjure" => match self.conjure(&cmd.object) {
                Ok(msg) => msg,
                Err(e) => e,
            },
            "inventory" => render::render_inventory(&self.graph, &self.player),
            "examine" => self.examine(&cmd.object),
            verb => self.fire_object_verb(verb, &cmd.object),
        }
    }

    fn edge_towards(&self, room: &NamedNode, dir: Direction) -> Option<NamedNode> {
        self.graph
            .objects(room, &vocab::connects_to())
            .into_iter()
            .filter_map(as_node)
            .find(|e| {
                self.graph
                    .object(e, &vocab::direction())
                    .and_then(|t| as_string(&t))
                    .as_deref()
                    == Some(dir.word())
            })
    }

    /// Moves the player through `object` (a direction word), generating the
    /// frontier past it first if nobody's been that way yet. Still the old
    /// `Delta`/`WorldGraph::commit` path for the actual room-membership
    /// change (`contains`) -- see `vocab::located_in`'s own doc comment for
    /// the full account of why. In brief: this crate's `#105` task asked for
    /// `contains` to fully migrate onto `apply_commit`/`current_value`, the
    /// same way `take`/`drop` retired `holds` for `heldBy`. Investigating
    /// every read site (see this method's own git history / PR description)
    /// turned up a blocker `holds` never hit: `WorldGraph::load_nquads`
    /// (what `Game::from_snapshot` uses) used to never restore
    /// `commit_log`, so any fact readable only through `current_value`/
    /// `current_subjects_with` went permanently blank the moment a session
    /// was reconstructed from a snapshot. `player_room` -- read by nearly
    /// every dispatch in `handle` -- would have inherited that blankness as
    /// an outright panic (`.expect("player is always in exactly one
    /// room")`), not a quiet degradation, and it's directly exercised by
    /// the existing `game_snapshot_and_from_snapshot_round_trip_a_playable_game`
    /// test (confirmed by reproducing it). That persistence gap is now
    /// closed (see `WorldGraph::dump_commit_log`/`restore_commit_log`), but
    /// `go` still commits `contains` the old way for `player_room` and
    /// everything else that reads it -- migrating `player_room` onto
    /// `locatedIn` is a separate read-side change this fix doesn't
    /// attempt, not something the persistence gap forces anymore -- and
    /// *additionally* records the destination via `apply_commit`'s
    /// `locatedIn` predicate, proving the write-side mechanism really
    /// works and materializes correctly across repeated transitions *and*
    /// a snapshot reload (see `player_location_via_located_in` and its
    /// tests).
    fn go(&mut self, object: &str) -> String {
        let Some(dir) = Direction::parse(object) else {
            return "Go where?".to_string();
        };
        let room = self.player_room();

        if self.edge_towards(&room, dir).is_none() {
            demiurge::generate_frontier(&mut self.graph, self.world_seed, &room, dir);
        }

        let Some(edge) = self.edge_towards(&room, dir) else {
            return "You can't go that way.".to_string();
        };
        let locked = self
            .graph
            .object(&edge, &vocab::locked())
            .and_then(|t| as_bool(&t))
            .unwrap_or(false);
        if locked {
            return "That way is sealed shut. Something here might open it.".to_string();
        }

        let dest = self
            .graph
            .object(&edge, &vocab::to())
            .and_then(as_node)
            .expect("a committed edge always has a destination");

        // Same delta, same commit: arriving and being recorded as having
        // arrived aren't two events, just two facts about one. Every room
        // is asserted with a `visits` counter at creation (0 for generated
        // rooms, 1 for the Threshold since the player starts there), so
        // this is always a retract-then-assert, never a bare assert.
        let visits = self
            .graph
            .object(&dest, &vocab::visits())
            .and_then(|t| as_int(&t))
            .unwrap_or(0);
        let mut d = Delta::new()
            .retract(room.clone(), vocab::contains(), self.player.clone())
            .assert(dest.clone(), vocab::contains(), self.player.clone());
        if let Some(old) = self.graph.object(&dest, &vocab::visits()) {
            d = d.retract(dest.clone(), vocab::visits(), old);
        }
        d = d.assert(dest.clone(), vocab::visits(), lit_int((visits + 1) as u64));
        self.graph
            .commit("player", d)
            .expect("player movement is always valid");

        // The apply_commit half of this method's migration -- see its own
        // doc comment for why this is additive, not a replacement for the
        // `contains` commit just above. `consumes` stays empty for the same
        // reason `take`/`drop`/`conjure` leave it empty: nothing here needs
        // the referential guard it exists for, since `current_value` already
        // resolves "later wins" without one.
        let quad = Quad::new(
            self.player.clone(),
            vocab::located_in(),
            Term::NamedNode(dest.clone()),
            oxigraph::model::GraphName::DefaultGraph,
        );
        let commit = Commit {
            consumes: Vec::new(),
            produces: format!("{quad} ."),
            predicate: "entersRoom".to_string(),
            via: None,
            responds_to: None,
            created_at: self.graph.now_ms().to_string(),
        };
        self.graph
            .apply_commit("player", commit)
            .expect("recording a room transition is always valid");

        render::render_room_text(&self.graph, &self.player, &dest, &self.unexplored_from(&dest))
    }

    /// The player's current room, per `locatedIn`'s own materialized state
    /// (see `vocab::located_in`'s doc comment) -- deliberately *not* what
    /// `player_room` (the crate's one authoritative read, used everywhere
    /// else) relies on. Exists so a caller -- this crate's own tests
    /// included -- can prove the `apply_commit`/`current_value` half of
    /// `go`'s migration actually materializes correctly across multiple
    /// room transitions within one live session, without reaching into
    /// `Game`'s private `graph` field. `None` before the player's first
    /// `go` -- bootstrap places the player in the Threshold via the old
    /// `contains` path only, so there's no `locatedIn` fact yet to read.
    /// Survives a snapshot round trip now (see `vocab::located_in`'s doc
    /// comment for the fix); a snapshot taken before that fix existed
    /// still decodes to `None` here after reload, same as it always did.
    pub fn player_location_via_located_in(&self) -> Option<NamedNode> {
        self.graph
            .current_value(&self.player, &vocab::located_in())
            .and_then(as_node)
    }

    fn find_by_name(&self, room: &NamedNode, name: &str) -> Option<NamedNode> {
        self.graph
            .objects(room, &vocab::contains())
            .into_iter()
            .filter_map(as_node)
            .find(|n| matches_name(&self.graph, n, name))
    }

    /// Items the player currently holds, per `heldBy`'s materialized state
    /// (see `vocab::held_by`/`WorldGraph::current_subjects_with`) -- the
    /// read-side counterpart to `take`/`drop`'s writes below, and what
    /// every other spot in this file that used to do a raw
    /// `objects(&self.player, &vocab::holds())` pattern lookup now calls
    /// instead (that predicate is retired for these verbs -- see `take`'s
    /// own doc comment).
    fn held_items(&self) -> Vec<NamedNode> {
        self.graph
            .current_subjects_with(&vocab::held_by(), &Term::NamedNode(self.player.clone()))
    }

    /// `take`/`drop`: the pair of body-verbs this prototype migrated off
    /// `Delta`/`WorldGraph::commit` onto `graph::Commit`/`apply_commit` +
    /// `WorldGraph::current_value` materialization for real -- not a pure
    /// mint like `conjure` (see its own doc comment for why that one
    /// dodged this), but a genuine repeatable state transition: the same
    /// item's holder changes every time it's taken or dropped, and nothing
    /// here ever retracts the prior `heldBy` fact -- `apply_commit` can't
    /// (see its own "no deletions" doc comment) -- so the store
    /// accumulates one `heldBy` generation per take/drop over an item's
    /// life, and only `current_value`/`current_subjects_with`, walking
    /// `commit_log`'s own order, can say which one is live.
    ///
    /// This is a *partial* migration, deliberately: only the "who holds
    /// this item" half (`holds`/`heldBy`) moved. "Which room is the
    /// player standing in" and "which items does a room currently
    /// contain" (`contains`) did not -- that predicate is read all over
    /// `render.rs`, `commune.rs`, and `demiurge.rs` for general
    /// room-content listing having nothing to do with this one item, and
    /// `apply_commit`'s no-delete rule means retiring it here would leave
    /// every one of those call sites looking at a stale "still in the old
    /// room" fact forever -- exactly the blast radius `conjure`'s own doc
    /// comment already flagged. `holds`, by contrast, is read/written
    /// nowhere but this file and `render::render_inventory` (see
    /// `vocab::held_by`), which is what made it the safe half to actually
    /// retire rather than another parallel shadow copy of the same state.
    fn take(&mut self, name: &str) -> String {
        if name.is_empty() {
            return "Take what?".to_string();
        }
        let room = self.player_room();
        let Some(item) = self.find_by_name(&room, name) else {
            return format!("There's no {name} here.");
        };
        let portable = self
            .graph
            .object(&item, &vocab::portable())
            .and_then(|t| as_bool(&t))
            .unwrap_or(false);
        let label = render::display_name(&self.graph, &item);
        if !portable {
            return format!("The {label} won't budge.");
        }

        // Leaving the room: still the old `Delta` path -- `contains` stays
        // on it, see this method's own doc comment for why.
        let leave = Delta::new().retract(room.clone(), vocab::contains(), item.clone());
        self.graph
            .commit("player", leave)
            .expect("leaving a room is always valid once the item's confirmed present");

        // Who holds it now: the real state transition, recorded via
        // `Commit`/`apply_commit` instead. `consumes` stays empty -- same
        // as `conjure` -- because `consumes`'s existence guard resolves
        // through `vocab::foreign_uri_node`, the cross-repo/atproto
        // addressing scheme a locally-minted `item` was never given an
        // address under; nothing here needs that guard anyway, since
        // `current_value` already tells a reader which generation of
        // `heldBy` is live without one.
        let quad = Quad::new(
            item.clone(),
            vocab::held_by(),
            Term::NamedNode(self.player.clone()),
            oxigraph::model::GraphName::DefaultGraph,
        );
        let commit = Commit {
            consumes: Vec::new(),
            produces: format!("{quad} ."),
            predicate: "takenBy".to_string(),
            via: None,
            responds_to: None,
            created_at: self.graph.now_ms().to_string(),
        };
        self.graph
            .apply_commit("player", commit)
            .expect("recording a take is always valid");

        format!("You take the {label}.")
    }

    /// See `take`'s doc comment -- this is its inverse half of the same
    /// migration.
    fn drop(&mut self, name: &str) -> String {
        if name.is_empty() {
            return "Drop what?".to_string();
        }
        let room = self.player_room();
        let Some(item) = self
            .held_items()
            .into_iter()
            .find(|n| matches_name(&self.graph, n, name))
        else {
            return format!("You aren't carrying a {name}.");
        };
        let label = render::display_name(&self.graph, &item);

        // Re-entering the room: still the old `Delta` path, same reasoning
        // as `take`'s own leave-the-room half.
        let enter = Delta::new().assert(room.clone(), vocab::contains(), item.clone());
        self.graph
            .commit("player", enter)
            .expect("entering a room is always valid");

        // No longer held by anyone: `vocab::nobody()` is the explicit
        // sentinel this needs, since `apply_commit` can't retract the
        // prior `heldBy(item, player)` fact -- see `vocab::nobody`'s own
        // doc comment for why a bare "assert nothing" wouldn't be
        // distinguishable from "never taken".
        let quad = Quad::new(
            item.clone(),
            vocab::held_by(),
            Term::NamedNode(vocab::nobody()),
            oxigraph::model::GraphName::DefaultGraph,
        );
        let commit = Commit {
            consumes: Vec::new(),
            produces: format!("{quad} ."),
            predicate: "releasedBy".to_string(),
            via: None,
            responds_to: None,
            created_at: self.graph.now_ms().to_string(),
        };
        self.graph
            .apply_commit("player", commit)
            .expect("recording a drop is always valid");

        format!("You drop the {label}.")
    }

    /// Mints a fresh, portable `Item` into the room the player is standing
    /// in -- the one real gameplay path routed through `graph::Commit`/
    /// `WorldGraph::apply_commit` instead of `Delta`/`WorldGraph::commit`
    /// (see that method's doc comment for the consume/produce semantics it
    /// implements). Deliberately a pure mint: `consumes` is empty, so this
    /// exercises only the "produces" half of a `Commit`'s contract.
    ///
    /// The consume-and-supersede half is *not* wired into any live verb
    /// here on purpose: `apply_commit` never removes a triple (see its own
    /// doc comment -- "no deletions, only a record that consumption
    /// happened"), so routing an already-existing stateful verb like
    /// `take`/`drop`/`go` through it instead of `Delta` would leave the
    /// *old* fact (the item still `contains`-ed by its old room, the
    /// player still `contains`-ed by their old room) sitting in the store
    /// right alongside the new one -- every other query in this crate
    /// (`player_room`, `find_by_name`, ...) assumes exactly one current
    /// `contains`/`holds` fact and would start returning whichever one the
    /// store's own (non-insertion-ordered) iteration happens to hit first.
    /// That consume+retract path is real and tested (see
    /// `graph::tests::apply_commit_mint_then_retract_...`), just not one
    /// this prototype's render/query layer is taught to treat a
    /// `consume_state`-retracted fact as absent yet -- a real follow-up,
    /// not something to fake here.
    pub fn conjure(&mut self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Conjure what?".to_string());
        }
        let room = self.player_room();
        let item = self.graph.fresh("item/");
        let quads = vec![
            Quad::new(
                item.clone(),
                vocab::rdf_type(),
                Term::NamedNode(vocab::class_item()),
                oxigraph::model::GraphName::DefaultGraph,
            ),
            Quad::new(
                item.clone(),
                vocab::name(),
                Term::Literal(lit_str(name.to_string())),
                oxigraph::model::GraphName::DefaultGraph,
            ),
            Quad::new(
                item.clone(),
                vocab::portable(),
                Term::Literal(lit_bool(true)),
                oxigraph::model::GraphName::DefaultGraph,
            ),
            Quad::new(
                room,
                vocab::contains(),
                Term::NamedNode(item),
                oxigraph::model::GraphName::DefaultGraph,
            ),
        ];
        // `Commit::produces` wants standard, dot-terminated N-Quads text
        // (see `graph::parse_nquads`'s doc comment) -- `Quad::to_string()`
        // itself omits the trailing `.`, same gap `Delta::from_canonical_text`
        // works around.
        let produces = quads
            .iter()
            .map(|q| format!("{q} ."))
            .collect::<Vec<_>>()
            .join("\n");

        let commit = Commit {
            consumes: Vec::new(),
            produces,
            predicate: "mints".to_string(),
            via: None,
            responds_to: None,
            // `engine` has no clock/date-formatting of its own (see
            // `WorldGraph::now_ms`'s doc comment) -- a real atproto-facing
            // caller stamps a proper ISO-8601 `createdAt` when it builds
            // the record for signing; nothing in this crate parses or
            // otherwise depends on this field's format. Reads back
            // whatever the caller last set via `Game::set_now` (same
            // convention `go`/`take`/`drop` rely on -- they don't take an
            // explicit `now_ms` either) rather than taking its own
            // parameter, since this verb is reached through the generic
            // `handle` dispatcher, not a dedicated entry point.
            created_at: self.graph.now_ms().to_string(),
        };
        self.graph
            .apply_commit("player", commit)
            .map_err(|e| e.to_string())?;
        Ok(format!("You conjure a {name} into being."))
    }

    fn examine(&self, name: &str) -> String {
        if name.is_empty() {
            return self.look();
        }
        let room = self.player_room();
        let target = self
            .find_by_name(&room, name)
            .or_else(|| self.held_items().into_iter().find(|n| matches_name(&self.graph, n, name)));
        match target {
            Some(t) => match render::perceive_examine(&self.graph, &self.player, &t) {
                Some(p) => render::render_percept_text(&p),
                None => "You have no way to examine that.".to_string(),
            },
            None => format!("You see no {name} here."),
        }
    }

    /// Everything that isn't a universal body-verb resolves here: find the
    /// named object, ask what machines equipped to *it* (not the player)
    /// advertise this verb, fire whichever have their requirements met.
    /// This is the "what can I do" control surface from the design
    /// discussion, made real -- affordance is a graph query, not a match
    /// arm, for anything beyond the fixed handful of body-verbs above.
    fn fire_object_verb(&mut self, verb: &str, object: &str) -> String {
        if object.is_empty() {
            return format!("You have no way to {verb}.");
        }
        let room = self.player_room();
        let Some(target) = self.find_by_name(&room, object) else {
            return format!("There's no {object} here.");
        };

        let machines = machine::machines_for_verb(&self.graph, &target, verb);
        if machines.is_empty() {
            return format!(
                "You have no way to {verb} the {}.",
                render::display_name(&self.graph, &target)
            );
        }

        let mut messages = Vec::new();
        for m in machines {
            let requirements: Vec<Requirement> = self
                .graph
                .objects(&m, &vocab::has_requirement())
                .into_iter()
                .filter_map(as_node)
                .filter_map(|r| machine::read_requirement(&self.graph, &r))
                .collect();
            if !requirements
                .iter()
                .all(|r| machine::requirement_met(&self.graph, r, &room))
            {
                continue;
            }
            let Some(effect_node) = self
                .graph
                .object(&m, &vocab::has_effect())
                .and_then(as_node)
            else {
                continue;
            };
            let Some(effect) = machine::read_effect(&self.graph, &effect_node) else {
                continue;
            };

            let delta = self.build_effect_delta(&effect);
            match self.graph.commit("player", delta) {
                Ok(()) => messages.push(render::describe_effect_outcome(&self.graph, &effect)),
                Err(e) => messages.push(format!("The mechanism jams. ({e})")),
            }
        }

        if messages.is_empty() {
            "Nothing happens.".to_string()
        } else {
            messages.join(" ")
        }
    }

    fn build_effect_delta(&self, effect: &Effect) -> Delta {
        match effect {
            Effect::IncrementAttr { node, attr, step } => {
                let current = self
                    .graph
                    .object(node, attr)
                    .and_then(|t| as_float(&t))
                    .unwrap_or(0.0);
                let (lo, hi) = crate::graph::graded_range(attr).unwrap_or((f32::MIN, f32::MAX));
                let new_value = (current + step).clamp(lo, hi);
                let mut d = Delta::new();
                if let Some(old) = self.graph.object(node, attr) {
                    d = d.retract(node.clone(), attr.clone(), old);
                }
                d.assert(node.clone(), attr.clone(), lit_float(new_value))
            }
            Effect::SetEdgeLocked { edge, value } => {
                let mut d = Delta::new();
                if let Some(old) = self.graph.object(edge, &vocab::locked()) {
                    d = d.retract(edge.clone(), vocab::locked(), old);
                }
                d.assert(edge.clone(), vocab::locked(), lit_bool(*value))
            }
            // Never actually reached: a `GenerateFrontier` machine is
            // equipped to a room, not to an item/npc `fire_object_verb`
            // resolves an object against, so this dispatch path can't find
            // one to fire. Handled as an inert no-op (not a panic) rather
            // than assumed unreachable, since a future content bug
            // mis-equipping one onto an object should degrade quietly, not
            // crash a player's turn.
            Effect::GenerateFrontier { .. } => Delta::new(),
        }
    }
}

fn matches_name(graph: &WorldGraph, node: &NamedNode, name: &str) -> bool {
    graph
        .object(node, &vocab::name())
        .and_then(|t| as_string(&t))
        .is_some_and(|n| n.to_lowercase().contains(&name.to_lowercase()))
}
