//! The fixed floor, re-expressed as an RDF vocabulary instead of Rust enums.
//! Two kinds of fixed thing live here: the *shape* predicates (equips,
//! trigger, senses, requirement/effect structure) that desiring-machines
//! are built from, and the minimal set of base sorts (`Room`/`Item`/
//! `Player`/`Npc`/`Machine`) kept as a pragmatic floor rather than a
//! metaphysical one — see the session notes on why `EntityKind` didn't
//! fully dissolve into crystallization for this prototype. Everything else
//! — attribute predicates like `wear`, crystallized kind-classes — is data,
//! not vocabulary, and gets minted at runtime.

use oxigraph::model::NamedNode;

const NS: &str = "http://ww/";

fn iri(local: &str) -> NamedNode {
    NamedNode::new(format!("{NS}{local}")).expect("vocabulary IRIs are well-formed by construction")
}

/// Turns an externally-proposed predicate's local name (from `commune.rs`
/// -- the demiurge's Workers AI-sourced relation proposals) into a
/// namespaced IRI. Unlike `iri()` above, which only ever sees this
/// module's own hand-written, known-good local names, this is fallible:
/// the input isn't ours to trust is well-formed, so a bad name is a
/// rejected proposal, not a panic. Characters outside `[A-Za-z0-9_-]` are
/// stripped rather than rejected outright -- good enough for a predicate
/// name a model wrote in prose case, and it still fails closed (empty
/// after stripping) rather than silently minting an empty-local IRI.
/// Underscores and hyphens are kept, not stripped: they're both valid in
/// an IRI local name and are exactly what a model reaches for to write a
/// multi-word predicate ("lit_by", "worn-smooth-from") -- stripping them
/// used to collapse those into unreadable runs ("litby",
/// "wornsmoothfrom") and created a silent collision risk between two
/// differently-punctuated names that meant different things.
pub fn dynamic_predicate(local: &str) -> Result<NamedNode, String> {
    let cleaned: String = local
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if cleaned.is_empty() {
        return Err(format!(
            "'{local}' has no usable characters for a predicate name"
        ));
    }
    NamedNode::new(format!("{NS}{cleaned}")).map_err(|e| e.to_string())
}

pub fn rdf_type() -> NamedNode {
    oxigraph::model::vocab::rdf::TYPE.into_owned()
}

// Base sorts.
pub fn class_room() -> NamedNode {
    iri("Room")
}
pub fn class_item() -> NamedNode {
    iri("Item")
}
pub fn class_player() -> NamedNode {
    iri("Player")
}
pub fn class_npc() -> NamedNode {
    iri("Npc")
}
pub fn class_machine() -> NamedNode {
    iri("Machine")
}
pub fn class_edge() -> NamedNode {
    iri("Edge")
}
/// A base sort, not generated content -- the resolution machinery (the
/// alarm-triggered flush, an external resolver) needs to find these
/// reliably regardless of what a specific petition concerns, the same way
/// `Machine` is fixed even though what a *specific* machine does is
/// generated. See `commune::raise_petition`/`resolve_petition_delta`.
pub fn class_petition() -> NamedNode {
    iri("Petition")
}

/// A frozen, point-in-time copy of a room's facts, minted when a petition
/// is raised and referenced by its `petitionContext` -- see
/// `commune::freeze_room_snapshot`/`context_from_snapshot`. Exists so a
/// resolution answers the world as it was asked about (real triples, not
/// a stored JSON string), not as it happens to be by the time anything
/// gets around to resolving it. See issue #15 ("no strings in the
/// graph") for why this replaced a raw JSON literal.
pub fn class_petition_snapshot() -> NamedNode {
    iri("PetitionSnapshot")
}
// Petition shape predicates -- see `class_petition` above. `concerns`
// points at the Room the petition is about (kind-checked the same way
// `contains`/`holds` are); `context` points at a PetitionSnapshot (same
// treatment) since #15's "no strings" pass. `result` stays a string
// literal deliberately -- it's rendered prose (the room's text after
// resolution, from `render::render_room_text`), the same
// generated-fresh-for-UI kind of string the schema already trusts
// elsewhere, not stored ground truth anything reasons over.
pub fn petition_concerns() -> NamedNode {
    iri("petitionConcerns")
}
/// Points at one of `status_pending`/`status_resolved`/`status_expired`
/// below -- a closed, three-valued enum expressed as node identity rather
/// than a string tag, so a petition's status is exactly as inspectable and
/// joinable as any other graph reference. See `graph::validate`'s
/// closed-set check.
pub fn petition_status() -> NamedNode {
    iri("petitionStatus")
}
pub fn status_pending() -> NamedNode {
    iri("status/pending")
}
pub fn status_resolved() -> NamedNode {
    iri("status/resolved")
}
pub fn status_expired() -> NamedNode {
    iri("status/expired")
}
/// The frozen `commune::build_context` JSON at the moment the petition was
/// raised -- frozen, not re-derived at resolution time, so a resolution
/// answers the world as it was asked about, not as it happens to be by the
/// time a subscriber gets to it.
pub fn petition_context() -> NamedNode {
    iri("petitionContext")
}
/// The prose result once resolved -- what `commune::run` used to return
/// directly now lands here instead, for the client to pick up on its next
/// fetch.
pub fn petition_result() -> NamedNode {
    iri("petitionResult")
}
/// A ms-epoch timestamp: past this, the petition is nobody's responsibility
/// anymore. Pub/sub has no delivery guarantee -- a dispatch's subscribers
/// may not exist, may be offline, or may just decline to answer -- so a
/// petition needs its own exit from the queue that doesn't depend on any
/// subscriber ever showing up. See `commune::expire_stale_petitions`.
pub fn petition_expires_at() -> NamedNode {
    iri("petitionExpiresAt")
}

// -- DMML petition state machine (additive, alongside the mechanism above)
// --
//
// The petition mechanism above stays exactly as-is -- it's what `server/`,
// `mcp-server/`, and `client/` actually call today, and this migration
// doesn't touch any of those. This second set of predicates is a parallel,
// real `Commit`/`apply_commit`-shaped petition flow matching the wire
// protocol design (`petitioner/.../petition.raise` ->
// `target-did/.../petition.reply` -> `petitioner/.../petition.accept`,
// FactRef-linked) -- proving the shape exists and works, additively, same
// posture as `heldBy` landing next to `contains` without retiring it.
// Deliberately its *own* predicates, not a reuse of `petitionStatus`'s
// closed three-value enum above: that enum's validator branch in
// `graph::validate` is specific to the old mechanism's three values, and
// extending it would couple two mechanisms meant to stay independently
// retireable. Self-declared (`class_relation`/`class_attribute`) rather
// than a new closed-vocabulary branch, same as `locatedIn`/`heldBy` --
// declared once at genesis (`demiurge::bootstrap`) so a `replay_commit`
// re-run of this content validates.

/// Relation from a Petition (or PetitionReply) node to its current DMML
/// lifecycle status node (`dmml_status_raised`/`_replied`/`_resolved`/
/// `_expired` below). Self-declared as `class_relation`.
pub fn dmml_petition_status() -> NamedNode {
    iri("dmmlPetitionStatus")
}
pub fn dmml_status_raised() -> NamedNode {
    iri("dmmlStatus/raised")
}
pub fn dmml_status_replied() -> NamedNode {
    iri("dmmlStatus/replied")
}
pub fn dmml_status_resolved() -> NamedNode {
    iri("dmmlStatus/resolved")
}
pub fn dmml_status_expired() -> NamedNode {
    iri("dmmlStatus/expired")
}
/// A `PetitionReply` node's own class marker -- the DMML flow's answer to
/// a raised petition, minted before its content is ever applied to the
/// world (see `commune::reply_petition_dmml`'s own doc comment for why
/// applying and replying are two separate steps).
pub fn class_petition_reply() -> NamedNode {
    iri("PetitionReply")
}
/// Relation from a PetitionReply to the Petition it answers. Self-declared
/// as `class_relation`.
pub fn replies_to() -> NamedNode {
    iri("repliesTo")
}
/// The reply's proposed content, held as the same wire-shape JSON
/// `commune::parse_commune_json` already validates -- stored inert on the
/// reply node rather than applied immediately, so accepting is a genuinely
/// separate, later act by the petition's own raiser (data sovereignty: a
/// reply proposes, it doesn't commit on the petitioner's behalf). Applied
/// for real, atomically, only once `petition.accept` fires (see
/// `commune::accept_petition_dmml`). Self-declared as `class_attribute`.
pub fn petition_reply_content() -> NamedNode {
    iri("petitionReplyContent")
}

/// Marks a predicate IRI, as a *subject* of `rdf:type`, as usable
/// node-to-node -- the self-declaration a novel relation must carry before
/// the validator will accept any triple that uses it. See
/// `graph::validate`'s handling of predicates outside the closed
/// shape-list.
pub fn class_relation() -> NamedNode {
    iri("Relation")
}
/// Same self-declaration mechanism as `class_relation`, but for a novel
/// predicate used node-to-literal instead.
pub fn class_attribute() -> NamedNode {
    iri("Attribute")
}

// Descriptive / spatial predicates. `description` doesn't live here
// anymore -- prose is never itself information, only the graph is, so
// nothing gets to store an authored sentence as if it were a fact.
// `render.rs` composes descriptive text fresh from an entity's real
// (dampness/decay/light, wear, ...) facts at render time instead; see
// "Ground truth is the graph, never prose" in the README. `name` is the
// one deliberate exception -- a label, not narrated content.
pub fn name() -> NamedNode {
    iri("name")
}
pub fn contains() -> NamedNode {
    iri("contains")
}
pub fn holds() -> NamedNode {
    iri("holds")
}

/// The forward (item -> holder) counterpart to `holds` -- populated only
/// via `WorldGraph::apply_commit`, never `Delta`/`WorldGraph::commit`, and
/// read back only via `WorldGraph::current_value`/`current_subjects_with`,
/// never a raw `objects`/`object` pattern lookup. `take`/`drop` are the
/// one pair of verbs this migrated off the `Delta` path (see their own
/// doc comments in `game.rs`): `holds` (player -> holds -> item) is
/// read/written nowhere in this crate but `game.rs` and
/// `render::render_inventory`, unlike the far more broadly shared
/// `contains` (room -> contains -> item/player/npc, read across
/// `render.rs`, `commune.rs`, `demiurge.rs`), which is why `holds`
/// specifically was the low-blast-radius half of "an item changes
/// location/holder" this prototype actually retired.
///
/// A single-valued (functional) predicate from the item's point of view --
/// exactly the shape `current_value` answers "later wins" for -- unlike
/// `contains`/`holds` themselves, which are inherently multi-valued from
/// their subject's side (a room contains many items at once) and so were
/// never a fit for a single-winner "current value" query to begin with;
/// that mismatch is *why* this predicate exists as its own thing rather
/// than `current_value` just being pointed at `holds`.
pub fn held_by() -> NamedNode {
    iri("heldBy")
}

/// The `heldBy` sentinel for "not currently held by anyone" -- asserted by
/// `drop`. Distinct from `WorldGraph::current_value` returning `None`
/// (which means "no `apply_commit` ever asserted `heldBy` for this item at
/// all", i.e. never taken even once): without this sentinel, a
/// taken-then-dropped item would be indistinguishable from one nobody has
/// ever touched.
pub fn nobody() -> NamedNode {
    iri("Nobody")
}

/// The forward (occupant -> room) counterpart to `contains` -- same shape
/// and same reasoning as `held_by`'s own doc comment above, just for room
/// membership instead of holding: `contains` is inherently multi-valued
/// from a *room's* side (a room contains many things at once), so it was
/// never a fit for `current_value`'s single-winner "later wins" query;
/// `locatedIn`, read from the *occupant's* side, is.
///
/// Unlike `held_by`, this predicate is *not* a completed migration --
/// see `Game::go`'s own doc comment for the full account. In short:
/// `commit_log` -- the one structure `current_value`/`current_subjects_with`
/// read -- used to go unrestored across a `Game::snapshot`/
/// `Game::from_snapshot` round trip (the tested, load-bearing persistence
/// path a real server-side caller depends on), silently blanking every
/// `apply_commit`-sourced fact the instant a session was reconstructed
/// from a snapshot. For `heldBy` that gap was real but untested and
/// low-stakes (a returning player's inventory listing would go quietly
/// empty). For `contains`, `player_room` -- the single most invoked read
/// in this entire crate, called by nearly every `handle` dispatch -- would
/// have inherited that same gap, and its `.expect("player is always in
/// exactly one room")` would panic instead of degrading quietly. Confirmed
/// by reproduction, not supposition: with the write side moved and
/// `player_room` migrated to read it,
/// `game_snapshot_and_from_snapshot_round_trip_a_playable_game` started
/// panicking on the very first post-restore call.
///
/// That gap is now closed -- see `WorldGraph::dump_commit_log`/
/// `restore_commit_log` and `Game::snapshot`/`Game::from_snapshot`, which
/// carry `commit_log` across the round trip alongside the plain N-Quads
/// dump, and `held_by`/`heldBy`'s own regression coverage
/// (`engine/tests/gameplay.rs`) for `apply_commit`-sourced state
/// surviving a reload. `locatedIn` is asserted on every `go` (proving the
/// write-side mechanism, and materializing correctly across repeated
/// transitions within one live session *and* across a snapshot reload --
/// see `Game::player_location_via_located_in` and its tests), but
/// `contains` itself still keeps every existing reader, `player_room`
/// included: retiring `contains` for real is a read-side migration this
/// fix doesn't attempt, not a persistence limitation anymore -- the
/// blocker this comment used to describe is gone, but the migration
/// itself is still a separate, deliberate scope cut.
pub fn located_in() -> NamedNode {
    iri("locatedIn")
}

pub fn connects_to() -> NamedNode {
    iri("connectsTo")
}
pub fn to() -> NamedNode {
    iri("to")
}
pub fn direction() -> NamedNode {
    iri("direction")
}
pub fn locked() -> NamedNode {
    iri("locked")
}
pub fn portable() -> NamedNode {
    iri("portable")
}

// Attribute predicates (the open, "lexical" side — new ones get minted the
// same way at runtime; these are just the ones this prototype's demiurge
// currently knows how to use).
pub fn wear() -> NamedNode {
    iri("wear")
}

// Room ground facts. Rendered prose is composed from these (render.rs)
// rather than stored as content -- see graph::GRADED_ATTRS for their
// declared ranges, and demiurge.rs for how they're rolled at generation.
pub fn dampness() -> NamedNode {
    iri("dampness")
}
pub fn decay() -> NamedNode {
    iri("decay")
}
pub fn light() -> NamedNode {
    iri("light")
}
/// A counter, not a graded [0,1] fact: how many times the player has
/// entered this room. Unbounded above, only ever increases. Ties a room's
/// rendering to the player's own history with it, not just its intrinsic
/// state.
pub fn visits() -> NamedNode {
    iri("visits")
}

/// Self-declared as a `ww:Attribute` wherever it's first used (see
/// `graph::validate`'s novel-predicate handling) rather than added as a
/// dedicated schema branch here -- an invited operator's standing is
/// content, not new mechanics. Carried by the machine `Game::
/// equip_operator` equips onto the player's own node when an invite is
/// minted, naming who was let in (see jedelman/written-world#8, "invite as
/// pentacle").
pub fn operator_label() -> NamedNode {
    iri("operatorLabel")
}

// Desiring-machine shape predicates — a machine's "grammar."
pub fn equips() -> NamedNode {
    iri("equips")
}
pub fn trigger() -> NamedNode {
    iri("trigger")
}
pub fn senses() -> NamedNode {
    iri("senses")
}
/// What a sense-machine's operation produces -- "room" | "map" so far. Lets
/// `render::perceive_room`/`perceive_map` find the machine(s) equipped for
/// a given percept kind the same way `machine::machines_for_verb` finds
/// action-machines by their trigger, instead of a call site assuming a
/// sense-machine exists at all. See `dmml_runtime::percept`.
pub fn render_kind() -> NamedNode {
    iri("renderKind")
}
/// Structural glue naming which percept field a sensed predicate exposes.
///
/// Replaces a hardcoded if-chain in `render::perceive_room`; mirroring how
/// `range_min`/`range_max` describe a predicate's own domain rather than
/// world content, this describes a predicate's own relationship to a
/// percept field name. Exempt from self-declaration via
/// `graph::is_structural_glue`.
pub fn unlocks_field() -> NamedNode {
    iri("unlocksField")
}
pub fn has_requirement() -> NamedNode {
    iri("hasRequirement")
}
pub fn has_effect() -> NamedNode {
    iri("hasEffect")
}

// Requirement-node shape.
pub fn requirement_kind() -> NamedNode {
    iri("requirementKind")
}
pub fn requirement_room() -> NamedNode {
    iri("requirementRoom")
}
pub fn requirement_edge() -> NamedNode {
    iri("requirementEdge")
}
pub fn requirement_locked_value() -> NamedNode {
    iri("requirementLockedValue")
}
pub fn requirement_attr_node() -> NamedNode {
    iri("requirementAttrNode")
}
pub fn requirement_attr_predicate() -> NamedNode {
    iri("requirementAttrPredicate")
}
pub fn requirement_threshold() -> NamedNode {
    iri("requirementThreshold")
}

// Effect-node shape.
pub fn effect_kind() -> NamedNode {
    iri("effectKind")
}
pub fn effect_target_node() -> NamedNode {
    iri("effectTargetNode")
}
pub fn effect_attr_predicate() -> NamedNode {
    iri("effectAttrPredicate")
}
pub fn effect_step() -> NamedNode {
    iri("effectStep")
}
pub fn effect_edge() -> NamedNode {
    iri("effectEdge")
}
pub fn effect_locked_value() -> NamedNode {
    iri("effectLockedValue")
}
/// Which Theos-flavored noun/attribute pool a `GenerateFrontier` effect
/// draws from ("stone", "vine", ...) -- an open string, not a closed enum,
/// since which Theoi exist is content (`demiurge.rs`'s pantheon), not
/// schema. Lives on the same fixed Effect-node shape as `effectStep`/
/// `effectLockedValue` rather than going through the self-declaring
/// Relation/Attribute path, because it's part of how *any* effect kind is
/// read back (`machine::read_effect`), not a novel predicate an external
/// proposer introduces.
pub fn effect_domain() -> NamedNode {
    iri("effectDomain")
}

// Crystallization bookkeeping — lives in the graph itself so `kinds` can
// just be a query, not a side-channel.
pub fn seen_count() -> NamedNode {
    iri("seenCount")
}

// Foreign-room linking ("reach") — see dmml_runtime::game::Game::reach and the
// README's "Corruption as content" section. A Room may declare a
// correspondence with a record living on someone else's atproto PDS: an
// `at://` URI (stable, never changes) plus the strong-ref CID this graph
// last observed it at (a content hash — see server::foreign_room for what
// re-fetches and compares it). Both stored as real node references rather
// than opaque string literals -- the engine still never interprets
// atproto's own addressing scheme beyond that, same as `petitionContext`
// holds opaque JSON it never parses.
pub fn foreign_uri() -> NamedNode {
    iri("foreignUri")
}
pub fn foreign_cid() -> NamedNode {
    iri("foreignCid")
}

/// An `at://` URI's authority segment embeds a DID (`did:plc:...`), which
/// puts a colon exactly where bare IRI syntax doesn't allow one unescaped
/// -- RFC 3987's authority grammar expects only digits after a colon (the
/// port), so `at://did:plc:abc123/...` isn't itself a valid IRI, even
/// though it's a perfectly valid at:// URI by atproto's own (looser)
/// grammar. Percent-encoding the whole thing into one opaque segment under
/// this crate's own namespace sidesteps needing to parse atproto's URI
/// grammar at all -- lossless and reversible via `foreign_uri_from_node`.
pub fn foreign_uri_node(at_uri: &str) -> NamedNode {
    iri(&format!("foreign/{}", percent_encode(at_uri)))
}

/// The inverse of `foreign_uri_node`. `None` if `n` isn't shaped like one
/// -- defensive; every caller in this crate only ever constructs these via
/// `foreign_uri_node`.
pub fn foreign_uri_from_node(n: &NamedNode) -> Option<String> {
    n.as_str()
        .strip_prefix(&format!("{NS}foreign/"))
        .and_then(percent_decode)
}

/// Extracts the DID authority segment from an `at://<did>/<collection>/<rkey>[#frag]`
/// URI -- `None` if `at_uri` isn't shaped like one at all (missing the
/// `at://` scheme, or an empty authority segment). Pure string parsing, no
/// RDF/`NamedNode` involvement -- this is the one thing a `FactRef`'s
/// same-repo scope guard (`SPEC.md`, section 6) actually needs to check
/// -- shared by `server/src/atproto/
/// commit_write.rs`'s write-time best-effort guard and `appview`'s
/// resolve-time authoritative guard so the two can't independently drift
/// on what counts as "same repo" the way `is_closed_vocabulary` closes an
/// analogous drift risk between `validate`/`validate_self_declared`
/// (jedelman/written-world PR #37 code review, bug 1).
pub fn did_of_at_uri(at_uri: &str) -> Option<&str> {
    at_uri
        .strip_prefix("at://")?
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
}

/// A content-addressed hash has no scheme of its own to make it a valid
/// IRI by itself -- same percent-encoding treatment as `foreign_uri_node`,
/// under a distinct prefix so the two never collide in a graph dump a
/// human is reading.
pub fn foreign_cid_node(cid: &str) -> NamedNode {
    iri(&format!("foreign-cid/{}", percent_encode(cid)))
}

/// The inverse of `foreign_cid_node`.
pub fn foreign_cid_from_node(n: &NamedNode) -> Option<String> {
    n.as_str()
        .strip_prefix(&format!("{NS}foreign-cid/"))
        .and_then(percent_decode)
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
/// The foreign record's own JSON content, as of the last fetch -- kept so
/// a future fetch has something to compare against, never itself rendered
/// to the player and never validated as a fact about this room. Overwritten
/// on every fetch, matching fact rather than accreting -- there's exactly
/// one "last known state," not a history of them.
pub fn foreign_snapshot() -> NamedNode {
    iri("foreignSnapshot")
}
/// An assert-only, accreting relation from a room to a `Drift` node --
/// never retracted, never read by any requirement (the "descriptive"
/// effect class: termination is trivial because nothing in this vocabulary
/// can ever gate a machine). Ground facts about the room are never
/// rewritten to match the foreign source; a `Drift` node is the structured
/// record that *something* changed, not a narrated claim about *what* --
/// see `class_drift` and "Corruption as content" in the README.
pub fn noticed_change() -> NamedNode {
    iri("noticedChange")
}

/// A structured record of a foreign correspondence's CID changing between
/// two observed fetches -- symbols and a quantity, never a sentence. What
/// used to be an AI-narrated guess at "what a player would notice" is now
/// exactly what's actually known: the old identifier, the new one, and
/// when the change was observed. See `Game::record_foreign_drift`.
pub fn class_drift() -> NamedNode {
    iri("Drift")
}
pub fn drift_old_cid() -> NamedNode {
    iri("driftOldCid")
}
pub fn drift_new_cid() -> NamedNode {
    iri("driftNewCid")
}
/// A ms-epoch timestamp, same convention as `petition_expires_at`.
pub fn drift_observed_at() -> NamedNode {
    iri("driftObservedAt")
}

// Commit provenance -- see `graph::Commit`/`WorldGraph::apply_commit`.
// `via`/`respondsTo` are asserted onto a freshly minted `Commit`-typed
// node only when the corresponding field is present on the `Commit`
// record being applied. Both point at a `foreign_uri_node`-addressed
// node -- the same addressing `consumes` resolves through -- so a
// via/respondsTo target is inspectable/joinable the same way any other
// foreign strong-ref is (see `foreign_uri`/`foreign_cid` above), and
// `apply_commit` also asserts that target node's own `foreignCid` fact
// from the `StrongRef`'s `cid`, same shape `reach`/`record_foreign_drift`
// already use for a room's own foreign correspondence.
pub fn class_commit() -> NamedNode {
    iri("Commit")
}
/// The `Commit` record's own open-ontology verb ("mints", "becomes", ...),
/// carried onto its minted `Commit` node as a plain string so the node
/// isn't just a bare anchor for `via`/`respondsTo` -- see `class_commit`.
pub fn commit_predicate() -> NamedNode {
    iri("commitPredicate")
}
pub fn via() -> NamedNode {
    iri("via")
}
pub fn responds_to() -> NamedNode {
    iri("respondsTo")
}

/// A world's own immutable lineage root -- minted once, first, via
/// `apply_commit` in `demiurge::bootstrap` (`Bootstrap::seed`, a public
/// field precisely so later genesis content -- or `generate_frontier`'s
/// own future migration onto this same pattern, still unbuilt -- has a
/// real handle to `via` without re-deriving it), addressed via
/// `foreign_uri_node` so it's a real `StrongRef`/`via` target the same way
/// any other durably-addressed node is. Nothing ever `consumes` it (there
/// is nothing to retract about a world's own origin); world-generated
/// content links back to it through `Commit::via`, the same provenance
/// mechanism a cross-repo `via`/`respondsTo` reference already uses, not a
/// bespoke "seed" relation. This is the concrete answer to "world-gen
/// content needs commit lineage" (#1/#50 Tier 1 item 2's actual blocker):
/// once a piece of genesis content is minted through `apply_commit` with
/// `via` pointing here, it has a real, addressable commit a later
/// `FactRef` can consume against -- the gap `contains`'s own migration
/// hits today (world-gen content predates the `Commit` model entirely, so
/// nothing about it is currently `consumes`-addressable at all).
pub fn class_seed() -> NamedNode {
    iri("Seed")
}

pub fn class_iri(n: u64) -> NamedNode {
    iri(&format!("class/{n}"))
}

pub fn fresh(local_prefix: &str, n: u64) -> NamedNode {
    iri(&format!("{local_prefix}{n}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn did_of_at_uri_extracts_the_authority_segment() {
        assert_eq!(
            did_of_at_uri("at://did:plc:abc123/org.jason-edelman.writtenworld.commit/xyz"),
            Some("did:plc:abc123")
        );
    }

    #[test]
    fn did_of_at_uri_handles_a_fragment_suffixed_uri() {
        assert_eq!(
            did_of_at_uri("at://did:plc:abc123/org.jason-edelman.writtenworld.commit/xyz#node"),
            Some("did:plc:abc123")
        );
    }

    #[test]
    fn did_of_at_uri_rejects_non_at_uris() {
        assert_eq!(did_of_at_uri("https://example.com/foo"), None);
        assert_eq!(did_of_at_uri("did:plc:abc123"), None);
        assert_eq!(did_of_at_uri(""), None);
    }

    #[test]
    fn did_of_at_uri_rejects_an_empty_authority() {
        assert_eq!(did_of_at_uri("at:///collection/rkey"), None);
    }
}
