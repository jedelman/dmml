//! The demiurge's Workers AI-sourced side: building the context sent to
//! `/api/commune`, and parsing what comes back into a `Delta`. Everything
//! here treats the model's JSON exactly as untrusted as player input --
//! parsing only ever produces a `Delta`, and that `Delta` still has to
//! clear `WorldGraph::commit`'s validator (including the self-declaring
//! Relation/Attribute check in `graph::validate`) before anything in it
//! becomes real. A malformed or rule-breaking proposal fails to parse or
//! fails to commit; it never partially applies.

use std::collections::{HashMap, HashSet};

use oxigraph::model::{NamedNode, Quad, Term};
use serde::{Deserialize, Serialize};

use crate::graph::{as_float, as_int, as_node, as_string, lit_bool, lit_float, lit_int, lit_str};
use crate::graph::{short, Commit, ConsumeRef, Delta, FactRef, StrongRef, WorldGraph};
use crate::vocab;

// -- Outbound: the context the Worker's prompt is grounded in ----------

#[derive(Serialize)]
struct RoomContext {
    name: String,
    dampness: f32,
    decay: f32,
    light: f32,
    visits: i64,
    items: Vec<String>,
    exits: Vec<String>,
}

#[derive(Serialize)]
struct VocabEntry {
    predicate: String,
    #[serde(rename = "type")]
    kind: &'static str,
    uses: usize,
}

#[derive(Serialize)]
struct CommuneContext {
    room: RoomContext,
    vocabulary: Vec<VocabEntry>,
}

/// The player's current room and the world's self-declared relation
/// vocabulary so far, as the JSON body `/api/commune` expects. Sent fresh
/// on every commune call rather than cached -- the Worker is stateless
/// (the graph only ever lives client-side), so this *is* the model's only
/// view of what already exists.
pub fn build_context(graph: &WorldGraph, room: &NamedNode) -> String {
    let name = graph
        .object(room, &vocab::name())
        .and_then(|t| as_string(&t))
        .unwrap_or_else(|| "an unnamed place".to_string());
    let dampness = graph
        .object(room, &vocab::dampness())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let decay = graph
        .object(room, &vocab::decay())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let light = graph
        .object(room, &vocab::light())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.5);
    let visits = graph
        .object(room, &vocab::visits())
        .and_then(|t| as_int(&t))
        .unwrap_or(0);

    let items: Vec<String> = graph
        .objects(room, &vocab::contains())
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n),
            _ => None,
        })
        .filter(|n| graph.has_type(n, &vocab::class_item()))
        .filter_map(|n| graph.object(&n, &vocab::name()).and_then(|t| as_string(&t)))
        .collect();

    let exits: Vec<String> = graph
        .objects(room, &vocab::connects_to())
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n),
            _ => None,
        })
        .filter_map(|edge| {
            graph
                .object(&edge, &vocab::direction())
                .and_then(|t| as_string(&t))
        })
        .collect();

    let mut vocabulary = Vec::new();
    for p in graph.subjects(
        &vocab::rdf_type(),
        &Term::NamedNode(vocab::class_relation()),
    ) {
        vocabulary.push(VocabEntry {
            predicate: short(&p),
            kind: "Relation",
            uses: graph.all_with_predicate(&p).len(),
        });
    }
    for p in graph.subjects(
        &vocab::rdf_type(),
        &Term::NamedNode(vocab::class_attribute()),
    ) {
        vocabulary.push(VocabEntry {
            predicate: short(&p),
            kind: "Attribute",
            uses: graph.all_with_predicate(&p).len(),
        });
    }

    let context = CommuneContext {
        room: RoomContext {
            name,
            dampness,
            decay,
            light,
            visits,
            items,
            exits,
        },
        vocabulary,
    };
    serde_json::to_string(&context).expect("CommuneContext always serializes")
}

// -- Petition snapshots: the same room facts, as real triples -----------
//
// `build_context` above is fine as-is for the live `/api/commune` call --
// it's never stored, just an outbound wire payload built fresh every time,
// exactly the "prose is a UI surface only, generated fresh" case issue #15
// already sanctions. A *petition's* context is different: it used to get
// frozen onto the graph as that same JSON string (`vocab::petition_context`
// held it as a literal), which is exactly the violation #15 was about --
// stored information a resolver (a Theos, an external client) has to
// re-parse rather than query. These two functions replace that: mint the
// frozen facts as real triples at raise time, then reconstruct the
// identical wire shape from them at read time, so `/api/commune` and every
// existing resolver need zero changes.

/// Mints a `PetitionSnapshot` node carrying `room`'s facts as real triples
/// -- quantities and item/edge references, not a JSON string -- frozen at
/// raise time. Returns the delta to fold into the petition's own delta,
/// plus the snapshot's node for `petitionContext` to reference. The
/// vocabulary summary deliberately isn't frozen here: it's global and
/// monotonic, not something that drifts per room, so `context_from_snapshot`
/// reads it live off the graph instead of off this snapshot.
pub fn freeze_room_snapshot(graph: &mut WorldGraph, room: &NamedNode) -> (Delta, NamedNode) {
    let name = graph
        .object(room, &vocab::name())
        .and_then(|t| as_string(&t))
        .unwrap_or_else(|| "an unnamed place".to_string());
    let dampness = graph
        .object(room, &vocab::dampness())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let decay = graph
        .object(room, &vocab::decay())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let light = graph
        .object(room, &vocab::light())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.5);
    let visits = graph
        .object(room, &vocab::visits())
        .and_then(|t| as_int(&t))
        .unwrap_or(0);

    let items: Vec<NamedNode> = graph
        .objects(room, &vocab::contains())
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n),
            _ => None,
        })
        .filter(|n| graph.has_type(n, &vocab::class_item()))
        .collect();

    let exits: Vec<NamedNode> = graph
        .objects(room, &vocab::connects_to())
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n),
            _ => None,
        })
        .collect();

    let snapshot = graph.fresh("petition-snapshot/");
    let mut d = Delta::new()
        .assert(
            snapshot.clone(),
            vocab::rdf_type(),
            vocab::class_petition_snapshot(),
        )
        .assert(snapshot.clone(), vocab::name(), lit_str(name))
        .assert(snapshot.clone(), vocab::dampness(), lit_float(dampness))
        .assert(snapshot.clone(), vocab::decay(), lit_float(decay))
        .assert(snapshot.clone(), vocab::light(), lit_float(light))
        .assert(snapshot.clone(), vocab::visits(), lit_int(visits as u64));
    for item in items {
        d = d.assert(snapshot.clone(), vocab::contains(), item);
    }
    for edge in exits {
        d = d.assert(snapshot.clone(), vocab::connects_to(), edge);
    }
    (d, snapshot)
}

/// Reconstructs the exact JSON shape `build_context` returns directly, from
/// a `PetitionSnapshot`'s frozen triples plus a *live* read of the world's
/// self-declared vocabulary (never frozen -- see `freeze_room_snapshot`'s
/// doc comment for why). What changed for #15 is what the graph stores,
/// not the wire shape built on top of it -- `Game::petition_context` calls
/// this and every existing caller keeps working unmodified.
pub fn context_from_snapshot(graph: &WorldGraph, snapshot: &NamedNode) -> Option<String> {
    let name = graph
        .object(snapshot, &vocab::name())
        .and_then(|t| as_string(&t))?;
    let dampness = graph
        .object(snapshot, &vocab::dampness())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let decay = graph
        .object(snapshot, &vocab::decay())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.0);
    let light = graph
        .object(snapshot, &vocab::light())
        .and_then(|t| as_float(&t))
        .unwrap_or(0.5);
    let visits = graph
        .object(snapshot, &vocab::visits())
        .and_then(|t| as_int(&t))
        .unwrap_or(0);

    let items: Vec<String> = graph
        .objects(snapshot, &vocab::contains())
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n),
            _ => None,
        })
        .filter_map(|n| graph.object(&n, &vocab::name()).and_then(|t| as_string(&t)))
        .collect();

    let exits: Vec<String> = graph
        .objects(snapshot, &vocab::connects_to())
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n),
            _ => None,
        })
        .filter_map(|edge| {
            graph
                .object(&edge, &vocab::direction())
                .and_then(|t| as_string(&t))
        })
        .collect();

    let mut vocabulary = Vec::new();
    for p in graph.subjects(
        &vocab::rdf_type(),
        &Term::NamedNode(vocab::class_relation()),
    ) {
        vocabulary.push(VocabEntry {
            predicate: short(&p),
            kind: "Relation",
            uses: graph.all_with_predicate(&p).len(),
        });
    }
    for p in graph.subjects(
        &vocab::rdf_type(),
        &Term::NamedNode(vocab::class_attribute()),
    ) {
        vocabulary.push(VocabEntry {
            predicate: short(&p),
            kind: "Attribute",
            uses: graph.all_with_predicate(&p).len(),
        });
    }

    let context = CommuneContext {
        room: RoomContext {
            name,
            dampness,
            decay,
            light,
            visits,
            items,
            exits,
        },
        vocabulary,
    };
    Some(serde_json::to_string(&context).expect("CommuneContext always serializes"))
}

// -- Inbound: what /api/commune returns ---------------------------------

#[derive(Deserialize)]
struct CommuneDelta {
    #[serde(default)]
    entities: Vec<CommuneEntity>,
    #[serde(default)]
    declarations: Vec<CommuneDeclaration>,
    #[serde(default)]
    triples: Vec<CommuneTriple>,
}

#[derive(Deserialize)]
struct CommuneEntity {
    #[serde(rename = "localId")]
    local_id: String,
    kind: String,
    name: String,
}

#[derive(Deserialize)]
struct CommuneDeclaration {
    predicate: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct CommuneTriple {
    subject: String,
    predicate: String,
    object: CommuneObject,
}

#[derive(Deserialize)]
struct CommuneObject {
    kind: String,
    #[serde(rename = "ref", default)]
    node_ref: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    datatype: Option<String>,
}

const MAX_ENTITIES: usize = 4;
const MAX_DECLARATIONS: usize = 4;
const MAX_TRIPLES: usize = 12;

/// Turns the Worker's JSON response into a `Delta` against `room`. Mints a
/// fresh node for each declared entity, maps predicate local names onto
/// namespaced IRIs via `vocab::dynamic_predicate`, and resolves "room" plus
/// any of this response's own entity ids as node references. Every failure
/// path here is a rejected proposal (`Err`), never a partial `Delta` --
/// this function either returns something wholly ready to hand to
/// `WorldGraph::commit`, or nothing.
pub fn parse_commune_delta(
    graph: &mut WorldGraph,
    room: &NamedNode,
    json: &str,
) -> Result<Delta, String> {
    parse_commune_json(graph, room, json)
}

/// The shared core both `parse_commune_delta` (the original direct-apply
/// path) and `resolve_petition_delta` (petition resolution) build on --
/// entities/declarations/triples parsing, self-declaration enforcement,
/// and the MAX_* caps are identical either way. Split out so a petition's
/// content isn't validated by a second, drifted copy of this logic.
fn parse_commune_json(
    graph: &mut WorldGraph,
    room: &NamedNode,
    json: &str,
) -> Result<Delta, String> {
    let parsed: CommuneDelta = serde_json::from_str(json).map_err(|e| e.to_string())?;

    if parsed.entities.len() > MAX_ENTITIES {
        return Err(format!(
            "too many entities ({} > {MAX_ENTITIES})",
            parsed.entities.len()
        ));
    }
    if parsed.declarations.len() > MAX_DECLARATIONS {
        return Err(format!(
            "too many declarations ({} > {MAX_DECLARATIONS})",
            parsed.declarations.len()
        ));
    }
    if parsed.triples.len() > MAX_TRIPLES {
        return Err(format!(
            "too many triples ({} > {MAX_TRIPLES})",
            parsed.triples.len()
        ));
    }

    let mut ids: HashMap<String, NamedNode> = HashMap::new();
    ids.insert("room".to_string(), room.clone());

    let mut d = Delta::new();

    for e in &parsed.entities {
        if ids.contains_key(&e.local_id) {
            return Err(format!("duplicate localId '{}'", e.local_id));
        }
        let class = match e.kind.as_str() {
            "Item" => vocab::class_item(),
            "Npc" => vocab::class_npc(),
            other => return Err(format!("cannot mint entity kind '{other}'")),
        };
        let node = graph.fresh("commune/");
        d = d
            .assert(node.clone(), vocab::rdf_type(), class)
            .assert(node.clone(), vocab::name(), lit_str(e.name.clone()))
            .assert(room.clone(), vocab::contains(), node.clone());
        ids.insert(e.local_id.clone(), node);
    }

    // Predicates this response is allowed to use in `triples`: whatever it
    // declares right here, plus whatever the world already has on record
    // as a self-declared Relation/Attribute from a past commune call. The
    // fixed schema vocabulary (contains, wear, locked, ...) is
    // deliberately never in this set -- those predicates carry strict
    // subject/object kind rules meant for the deterministic generator's
    // own bookkeeping, and a model reaching for "contains" to express
    // placement produces exactly the kind of confusing low-level rejection
    // (e.g. "contains object must have one of [...]") this check turns
    // into a clear, actionable one instead. Containment for a minted
    // entity is already handled automatically above; the model has no
    // legitimate reason to assert `contains` itself.
    let mut usable_predicates: HashSet<NamedNode> = HashSet::new();

    for decl in &parsed.declarations {
        let predicate = vocab::dynamic_predicate(&decl.predicate)?;
        let class = match decl.kind.as_str() {
            "Relation" => vocab::class_relation(),
            "Attribute" => vocab::class_attribute(),
            other => return Err(format!("unknown declaration type '{other}'")),
        };
        d = d.assert(predicate.clone(), vocab::rdf_type(), class);
        usable_predicates.insert(predicate);
    }

    for t in &parsed.triples {
        let subject = ids
            .get(&t.subject)
            .cloned()
            .ok_or_else(|| format!("triple references unknown subject '{}'", t.subject))?;
        let predicate = vocab::dynamic_predicate(&t.predicate)?;
        if !usable_predicates.contains(&predicate)
            && !graph.has_type(&predicate, &vocab::class_relation())
            && !graph.has_type(&predicate, &vocab::class_attribute())
        {
            return Err(format!(
                "predicate '{}' is not a self-declared relation or attribute -- \
                 declare it in this response or reuse one already declared, \
                 not the fixed schema vocabulary",
                t.predicate
            ));
        }
        let object = resolve_object(&t.object, &ids)?;
        d = d.assert(subject, predicate, object);
    }

    Ok(d)
}

// -- Petitions: an async message queue between the graph and whoever's
// listening -- a webhook subscriber, an external resolver polling the
// list/resolve endpoints, the demiurge's own Workers AI responding to a
// dispatch, the player themselves -- staying immanent to the graph
// instead of a host-level side channel. See vocab.rs's petition
// predicates. The queue itself doesn't know or care who resolves a
// petition or how; it only mints requests, dispatches their existence
// (see `GameObject::alarm` in the server crate), and retires whatever
// nobody got to in time.

/// Ten minutes -- long enough for a webhook round trip or a player to
/// notice and act, short enough that an abandoned petition doesn't sit in
/// the queue forever. `Game::raise_petition_for_current_room` uses this by
/// default; callers that want a different window can call `raise_petition`
/// directly.
pub const DEFAULT_PETITION_TTL_MS: u64 = 10 * 60 * 1000;

/// Raises a petition against `room`: mints a fresh Petition node, freezes
/// the room's facts onto a `PetitionSnapshot` node and points
/// `petitionContext` at it (frozen, not re-derived at resolution time -- a
/// resolution answers the world as it was asked about, not as it happens
/// to be by the time anything gets around to it), and marks it pending
/// with an expiry of `now_ms + ttl_ms`. See `freeze_room_snapshot` for why
/// this is real triples rather than the JSON string it used to be (#15).
/// Instant and synchronous -- no dispatch, no AI call, no I/O of any kind
/// happens here, this only enqueues the request (this crate has no clock
/// of its own, hence `now_ms` arriving as a plain parameter rather than
/// being read here). Returns the delta to commit alongside the freshly
/// minted petition's own id, so the caller can report it without a second
/// lookup.
pub fn raise_petition(
    graph: &mut WorldGraph,
    room: &NamedNode,
    now_ms: u64,
    ttl_ms: u64,
) -> (Delta, NamedNode) {
    let (snapshot_delta, snapshot) = freeze_room_snapshot(graph, room);
    let petition = graph.fresh("petition/");
    let d = snapshot_delta
        .assert(petition.clone(), vocab::rdf_type(), vocab::class_petition())
        .assert(petition.clone(), vocab::petition_concerns(), room.clone())
        .assert(
            petition.clone(),
            vocab::petition_status(),
            vocab::status_pending(),
        )
        .assert(petition.clone(), vocab::petition_context(), snapshot)
        .assert(
            petition.clone(),
            vocab::petition_expires_at(),
            lit_int(now_ms + ttl_ms),
        );
    (d, petition)
}

/// Retires every pending petition whose `petitionExpiresAt` has passed
/// into `"expired"` -- pub/sub has no delivery guarantee, so a petition
/// needs an exit from the queue that doesn't depend on any subscriber ever
/// showing up. Returns the delta to commit alongside the ids that expired,
/// so a caller (the flush) can skip dispatching notifications for them.
/// Doesn't touch petitions that are already resolved, or ones still
/// within their TTL.
pub fn expire_stale_petitions(graph: &WorldGraph, now_ms: u64) -> (Delta, Vec<NamedNode>) {
    let mut d = Delta::new();
    let mut expired = Vec::new();
    for petition in pending_petitions(graph) {
        let past_ttl = graph
            .object(&petition, &vocab::petition_expires_at())
            .and_then(|t| as_int(&t))
            .is_some_and(|expires_at| (expires_at as u64) <= now_ms);
        if past_ttl {
            d = d
                .retract(
                    petition.clone(),
                    vocab::petition_status(),
                    vocab::status_pending(),
                )
                .assert(
                    petition.clone(),
                    vocab::petition_status(),
                    vocab::status_expired(),
                );
            expired.push(petition);
        }
    }
    (d, expired)
}

/// Every petition still awaiting resolution, oldest first -- the flush's
/// work queue, and what an external resolver lists. FIFO via
/// `graph::creation_order`'s transcript-timestamp lookup, the same
/// ordering primitive `machine::machines_for_verb` uses for firing order.
pub fn pending_petitions(graph: &WorldGraph) -> Vec<NamedNode> {
    let mut petitions = graph.subjects(
        &vocab::petition_status(),
        &Term::NamedNode(vocab::status_pending()),
    );
    petitions.sort_by_key(|p| crate::graph::creation_order(graph, p));
    petitions
}

/// Resolves `petition` using `json` (the same wire shape `/api/commune`'s
/// AI response has always used), reusing `parse_commune_json` so a
/// petition's content is validated exactly as a direct commune call always
/// has been. Re-checks the petition is still "pending" first: Durable
/// Objects serialize requests to one instance so there's no true
/// concurrency hazard, but a stale caller (the flush and an external
/// resolver racing, or the flush retrying a petition it already handled)
/// should fail cleanly rather than double-apply. Flips status to
/// "resolved" as part of the same delta as the content itself, so a
/// commit that fails validation leaves the petition exactly as pending as
/// it started -- nothing here can half-resolve a petition.
///
/// Deliberately doesn't touch `petitionResult`: that's the room's
/// freshly-rendered prose, which needs `render.rs` and the graph
/// post-commit -- one layer up from what this module concerns itself
/// with. See `Game::resolve_petition`.
pub fn resolve_petition_delta(
    graph: &mut WorldGraph,
    petition: &NamedNode,
    json: &str,
) -> Result<Delta, String> {
    let status = graph.object(petition, &vocab::petition_status()).and_then(as_node);
    if status.as_ref() != Some(&vocab::status_pending()) {
        let label = status
            .as_ref()
            .map(short)
            .unwrap_or_else(|| "an unknown state".to_string());
        return Err(format!(
            "petition is {label}, not pending -- already resolved or expired"
        ));
    }
    let room = graph
        .object(petition, &vocab::petition_concerns())
        .and_then(as_node)
        .ok_or("petition has no concerns room")?;

    let mut d = parse_commune_json(graph, &room, json)?;
    d = d
        .retract(
            petition.clone(),
            vocab::petition_status(),
            vocab::status_pending(),
        )
        .assert(
            petition.clone(),
            vocab::petition_status(),
            vocab::status_resolved(),
        );
    Ok(d)
}

// -- DMML petition state machine (additive -- see vocab.rs's own
// "DMML petition state machine" doc comment for why this is a second,
// parallel mechanism rather than a migration of the one above) --------
//
// Two review notes worth recording rather than silently resolving:
//
// - **Reuses `petitionConcerns`/`petitionContext`** from the old mechanism
//   rather than minting DMML-specific copies, deliberately (unlike
//   `petitionStatus`, which *does* get its own `dmmlPetitionStatus` -- see
//   that predicate's own doc comment for why). "Which room does this
//   petition concern" and "what were the room's facts at raise time" are
//   the same question regardless of which state machine is asking it;
//   giving them two parallel predicates would be vocabulary duplication
//   for no real semantic difference. The real coupling this creates: if
//   the old mechanism is ever retired, `graph::validate`'s dedicated
//   `petitionConcerns`/`petitionContext` shape-check branches (kept for
//   its sake) would need to stay for this mechanism's sake too, or get
//   ported to self-declaration. Not a problem today -- both mechanisms
//   coexist -- just not independently free to diverge on this one point
//   the way they are on status.
// - **No additional concurrency guard beyond the `current_value`-then-
//   `apply_commit` check each function already does.** A true TOCTOU gap
//   exists in the abstract (two concurrent replies to the same petition
//   could both pass the check before either commits) -- but this is the
//   same property `resolve_petition_delta`'s own doc comment already
//   documents and accepts for the old mechanism ("Durable Objects
//   serialize requests to one instance so there's no true concurrency
//   hazard"): this code runs under the identical single-instance
//   serialization, so it inherits that guarantee rather than needing a
//   new one. Would need real attention if this mechanism is ever driven
//   from somewhere that doesn't serialize requests the same way.

/// Raises a petition against `room` via the DMML `Commit`/`apply_commit`
/// path, matching the wire protocol shape `petitioner/.../petition.raise`
/// is meant to have once wired to real cross-DID atproto records: a pure
/// mint (`consumes: []`), `predicate: "petition.raise"`. Still freezes the
/// room's context via `freeze_room_snapshot` on the old `Delta`/`commit`
/// path first (unchanged) -- see this module's vocab doc comment for why
/// only the state machine's own transitions move onto `Commit`, not the
/// snapshot machinery petitions have always shared with room-content.
///
/// The petition's own node identity is minted via `vocab::foreign_uri_node`
/// over a locally-fresh id (`WorldGraph::fresh`'s only role here is
/// guaranteeing uniqueness; the resulting node is discarded, only its
/// string form is kept) rather than a plain `WorldGraph::fresh` node
/// directly -- so it's independently addressable by a `StrongRef`/`FactRef`
/// later, the same "durable node identity" convention any real cross-repo
/// reference already uses (see `FactRef`'s own doc comment). This is what
/// makes `reply_petition_dmml`'s FactRef-guarded double-reply rejection
/// possible at all.
pub fn raise_petition_dmml(
    graph: &mut WorldGraph,
    room: &NamedNode,
    now_ms: u64,
    ttl_ms: u64,
) -> Result<NamedNode, String> {
    let (snapshot_delta, snapshot) = freeze_room_snapshot(graph, room);
    graph.set_now(now_ms);
    graph
        .commit("player", snapshot_delta)
        .map_err(|e| e.to_string())?;

    let petition_id = graph.fresh("petition-dmml/");
    let petition = vocab::foreign_uri_node(petition_id.as_str());
    let quads = vec![
        Quad::new(
            petition.clone(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_petition()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            petition.clone(),
            vocab::petition_concerns(),
            Term::NamedNode(room.clone()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            petition.clone(),
            vocab::petition_context(),
            Term::NamedNode(snapshot),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            petition.clone(),
            vocab::petition_expires_at(),
            Term::Literal(lit_int(now_ms + ttl_ms)),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            petition.clone(),
            vocab::dmml_petition_status(),
            Term::NamedNode(vocab::dmml_status_raised()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
    ];
    let produces = quads
        .iter()
        .map(|q| format!("{q} ."))
        .collect::<Vec<_>>()
        .join("\n");
    let commit = Commit {
        consumes: Vec::new(),
        produces,
        predicate: "petition.raise".to_string(),
        via: None,
        responds_to: None,
        created_at: graph.now_ms().to_string(),
    };
    graph
        .apply_commit("player", commit)
        .map_err(|e| e.to_string())?;
    Ok(petition)
}

/// A `FactRef` consuming `petition`'s current `dmmlPetitionStatus` fact,
/// wildcarded (`object: None`) rather than pinned to a specific status
/// value. Pinning isn't possible with the addressing convention
/// `FactRef.object` actually uses (`term_matches_fact_object`, `graph.rs`):
/// a `NamedNode` object is only ever matched via its `foreign_uri_node`
/// decoding, which the fixed `dmmlStatus/*` vocabulary nodes deliberately
/// aren't wrapped in (they're plain enum-tag constants, not durably-
/// addressed content in their own right -- wrapping them would be a
/// category error, the same way `vocab::status_pending()` isn't either).
/// The wildcard still does real work at the store level (only a subject
/// that currently has *some* `dmmlPetitionStatus` fact can be consumed at
/// all), and every caller pairs it with an explicit `current_value` check
/// first for the specific-value guarantee and a legible error message --
/// same two-layer pattern the old mechanism's `resolve_petition_delta`
/// already used (an application-level status check, not just relying on
/// the validator/store to reject a bad transition).
fn dmml_status_factref(graph: &WorldGraph, node: &NamedNode) -> Result<ConsumeRef, String> {
    let uri = vocab::foreign_uri_from_node(node)
        .ok_or("node is not a durable-addressed (foreign_uri_node) identity")?;
    let _ = graph; // kept as a parameter for symmetry with a future same-repo scope check
    Ok(ConsumeRef::Fact(FactRef {
        commit: StrongRef {
            uri: uri.clone(),
            cid: "local".to_string(),
        },
        subject: uri,
        predicate: vocab::dmml_petition_status().as_str().to_string(),
        object: None,
    }))
}

/// Proposes a reply to `petition`: validates `json` parses as well-formed
/// commune content (reusing `parse_commune_json`'s validation, the same
/// check the old mechanism runs) but does **not** apply it -- the parsed
/// `Delta` is discarded once validation succeeds, and the raw `json` is
/// stored inert on a fresh `PetitionReply` node instead. See
/// `vocab::petition_reply_content`'s own doc comment for why proposing and
/// applying are two separate steps: a reply is something the petition's
/// own raiser has to actively accept (`accept_petition_dmml`), not
/// something a replier's commit can apply to the petitioner's world
/// unilaterally -- the data-sovereignty property the wire protocol is
/// actually for.
///
/// `consumes` a wildcarded `FactRef` against `petition`'s own current
/// `dmmlPetitionStatus` fact (see `dmml_status_factref`), so a second
/// reply to an already-replied petition is rejected by the same
/// referential guard a real double-spend attempt would hit, not just an
/// application-level check -- though the explicit `current_value` check
/// below runs first, for a clearer error message before the store
/// round-trip.
pub fn reply_petition_dmml(
    graph: &mut WorldGraph,
    petition: &NamedNode,
    json: &str,
    source: &str,
) -> Result<NamedNode, String> {
    let status = graph.current_value(petition, &vocab::dmml_petition_status());
    if status != Some(Term::NamedNode(vocab::dmml_status_raised())) {
        return Err(
            "petition is not awaiting a reply -- already replied, resolved, or expired"
                .to_string(),
        );
    }
    let room = graph
        .object(petition, &vocab::petition_concerns())
        .and_then(as_node)
        .ok_or("petition has no concerns room")?;
    // Validate-only: confirms `json` would parse as real commune content
    // before it's ever stored. The resulting `Delta` is intentionally
    // discarded -- applying it is `accept_petition_dmml`'s job, later.
    parse_commune_json(graph, &room, json)?;

    let consumes_ref = dmml_status_factref(graph, petition)?;

    let reply_id = graph.fresh("petition-reply/");
    let reply = vocab::foreign_uri_node(reply_id.as_str());
    let quads = vec![
        Quad::new(
            reply.clone(),
            vocab::rdf_type(),
            Term::NamedNode(vocab::class_petition_reply()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            reply.clone(),
            vocab::replies_to(),
            Term::NamedNode(petition.clone()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        Quad::new(
            reply.clone(),
            vocab::petition_reply_content(),
            Term::Literal(lit_str(json)),
            oxigraph::model::GraphName::DefaultGraph,
        ),
        // The petition's own status moves to `replied` in this same
        // commit, alongside the reply's existence -- one atomic step, not
        // a race between "the reply exists" and "the petition knows it".
        Quad::new(
            petition.clone(),
            vocab::dmml_petition_status(),
            Term::NamedNode(vocab::dmml_status_replied()),
            oxigraph::model::GraphName::DefaultGraph,
        ),
    ];
    let produces = quads
        .iter()
        .map(|q| format!("{q} ."))
        .collect::<Vec<_>>()
        .join("\n");
    let commit = Commit {
        consumes: vec![consumes_ref],
        produces,
        predicate: "petition.reply".to_string(),
        via: None,
        responds_to: None,
        created_at: graph.now_ms().to_string(),
    };
    graph
        .apply_commit(source, commit)
        .map_err(|e| e.to_string())?;
    Ok(reply)
}

/// Accepts `petition`'s current reply: applies its stored content for real
/// (the same validated `parse_commune_json` -> `WorldGraph::commit` path
/// `resolve_petition_delta` already uses) and flips the DMML status to
/// `dmmlStatus/resolved`. Two commits, not one, same reasoning
/// `resolve_petition_delta`'s own doc comment already gives for the old
/// mechanism: world-content application (the old `Delta`/`WorldGraph::
/// commit` path -- see this module's vocab doc comment for why only the
/// state machine's own transitions move onto `Commit`) goes first, so a
/// validator rejection leaves the petition exactly as replied as it
/// started; the DMML status flip (`Commit`/`apply_commit`, `predicate:
/// "petition.accept"`) only happens once that's confirmed to have landed.
/// The graph itself is left updated for the caller to read from (e.g.
/// `Game::accept_petition_dmml`'s own post-commit render), same as
/// `resolve_petition_delta`'s caller already does.
pub fn accept_petition_dmml(
    graph: &mut WorldGraph,
    petition: &NamedNode,
    now_ms: u64,
) -> Result<(), String> {
    let status = graph.current_value(petition, &vocab::dmml_petition_status());
    if status != Some(Term::NamedNode(vocab::dmml_status_replied())) {
        return Err(
            "petition has no pending reply to accept -- not yet replied, already resolved, \
             or expired"
                .to_string(),
        );
    }
    let reply = graph
        .subjects(&vocab::replies_to(), &Term::NamedNode(petition.clone()))
        .into_iter()
        .next()
        .ok_or("petition is replied but no PetitionReply node references it -- inconsistent state")?;
    let content = graph
        .object(&reply, &vocab::petition_reply_content())
        .and_then(|t| as_string(&t))
        .ok_or("reply has no stored content")?;
    let room = graph
        .object(petition, &vocab::petition_concerns())
        .and_then(as_node)
        .ok_or("petition has no concerns room")?;

    let apply_delta = parse_commune_json(graph, &room, &content)?;
    graph.set_now(now_ms);
    graph
        .commit("player", apply_delta)
        .map_err(|e| e.to_string())?;

    let consumes_ref = dmml_status_factref(graph, petition)?;
    let quad = Quad::new(
        petition.clone(),
        vocab::dmml_petition_status(),
        Term::NamedNode(vocab::dmml_status_resolved()),
        oxigraph::model::GraphName::DefaultGraph,
    );
    let commit = Commit {
        consumes: vec![consumes_ref],
        produces: format!("{quad} ."),
        predicate: "petition.accept".to_string(),
        via: None,
        responds_to: None,
        created_at: graph.now_ms().to_string(),
    };
    graph
        .apply_commit("player", commit)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn resolve_object(obj: &CommuneObject, ids: &HashMap<String, NamedNode>) -> Result<Term, String> {
    match obj.kind.as_str() {
        "node" => {
            let r = obj.node_ref.as_ref().ok_or("node object missing 'ref'")?;
            let node = ids
                .get(r)
                .cloned()
                .ok_or_else(|| format!("triple references unknown object '{r}'"))?;
            Ok(Term::NamedNode(node))
        }
        "literal" => {
            let value = obj.value.clone().ok_or("literal object missing 'value'")?;
            let term = match obj.datatype.as_deref().unwrap_or("string") {
                "string" => Term::Literal(lit_str(value)),
                "bool" => Term::Literal(lit_bool(
                    value
                        .parse()
                        .map_err(|_| format!("'{value}' is not a bool"))?,
                )),
                "float" => Term::Literal(lit_float(
                    value
                        .parse()
                        .map_err(|_| format!("'{value}' is not a float"))?,
                )),
                "integer" => Term::Literal(lit_int(
                    value
                        .parse()
                        .map_err(|_| format!("'{value}' is not an integer"))?,
                )),
                other => return Err(format!("unknown datatype '{other}'")),
            };
            Ok(term)
        }
        other => Err(format!("unknown object kind '{other}'")),
    }
}
