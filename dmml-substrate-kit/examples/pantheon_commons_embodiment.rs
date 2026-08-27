//! Phase 3: embodiment and ecstasy, per Jason's follow-up after reading
//! the rupture/reconciliation document: "the shape of what Hardt and
//! Negri describe is beginning to form... I'm particularly interested
//! in the embodiment and ecstasy dimension: Reich and Plotkin, as well
//! as anyone that seems related." Wilhelm Reich (already a real
//! `G-DP-016` profile in Power Explained; character/muscular armor,
//! sexual repression as the mechanism enabling fascism) and Bill
//! Plotkin (7 real citations in the same grep survey, though not yet a
//! `G-DP-*` profile; the descent to soul, wildness as the person's own
//! nature rather than a place visited) are the most direct embodiment/
//! ecstasy pairing available in the corpus -- and both bear directly on
//! threads this project's own debates already opened without being able
//! to close: Dionysus's whole-night argument that ecstasy is the war of
//! position's fuel, not outside it; Lorde's erotic-as-power and the
//! "inward debt" the reconciliation document left undenominated.
//!
//! Structurally: same frozen-prior-plus-new-anchors pattern as
//! `pantheon_commons_rupture.rs`, but the frozen prior this time is the
//! RECONCILIATION document's own 9 items (`pantheon_commons_
//! reconciliation.rs`'s output) -- the most recent real, checkpointed
//! state of the whole night's argument -- not the earlier, already-
//! superseded phase-1 synthesis. `DispatchMode::Extend` invites
//! confirmation, extension, or rupture of specific reconciliation items,
//! same discipline as before, but framed as continuing an already-open
//! document rather than re-opening a falsely closed one: item9 of the
//! reconciliation was explicitly left unresolved, so testing it against
//! new material is not a violation of anything, it is the document's own
//! stated next step.

use std::collections::HashMap;

use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionToolChoiceOption,
    ChatCompletionTools, CreateChatCompletionRequestArgs, FunctionObject, ReasoningEffort,
    ToolChoiceOptions,
};
use async_openai::Client;
use dmml_runtime::graph::{Commit, ConsumeRef, FactRef, StrongRef};
use dmml_runtime::substrate::AppendSubstrate;
use dmml_substrate_kit::iroh_substrate::IrohAppendSubstrate;
use iroh::endpoint::Builder as EndpointBuilder;
use iroh_blobs::store::mem::MemStore;
use iroh_docs::api::Doc;
use iroh_docs::protocol::Docs;
use iroh_docs::AuthorId;
use serde::{Deserialize, Serialize};

struct Anchor {
    id: &'static str,
    author: &'static str,
    verb: &'static str,
    subject: &'static str,
    predicate: &'static str,
    object: &'static str,
}

// The 16 new anchors. Every claim is either a verified direct quote or
// an explicitly labeled paraphrase of a well-documented argument, same
// discipline as every anchor in this project.
const NEW_ANCHORS: &[Anchor] = &[
    // Wilhelm Reich, The Mass Psychology of Fascism (1933) and Character Analysis.
    Anchor { id: "reich/character_armor", author: "reich", verb: "argues", subject: "character_analysis/character_armor", predicate: "claim", object: "Neurosis is maintained by chronic muscular and attitudinal defenses -- character armor, a rigid, habitual bracing against one's own feeling and against anxiety" },
    Anchor { id: "reich/muscular_armor_body", author: "reich", verb: "argues", subject: "character_analysis/muscular_armor", predicate: "claim", object: "Character armor is not merely psychological but written directly into the body as chronic muscular tension, so a person's defenses can be read in how they physically hold themselves" },
    Anchor { id: "reich/sexual_repression_fascism", author: "reich", verb: "argues", subject: "mass_psychology/sexual_repression_fascism", predicate: "claim", object: "Fascism arises from deeply rooted patterns of sexual repression enforced through authoritarian family structure, not primarily from economic conditions alone" },
    Anchor { id: "reich/repression_serves_property", author: "reich", verb: "argues", subject: "mass_psychology/repression_serves_property", predicate: "claim", object: "Sexual repression and the morality built around it developed alongside class society and private property, organized and enforced through the institution of compulsory marriage" },
    Anchor { id: "reich/little_man", author: "reich", verb: "asserts", subject: "listen_little_man/little_man", predicate: "claim", object: "The little man does not know that he is little, and he is afraid of knowing it; he covers up his smallness and narrowness with illusions of strength and greatness" },
    Anchor { id: "reich/emotional_plague", author: "reich", verb: "argues", subject: "mass_psychology/emotional_plague", predicate: "claim", object: "The emotional plague names a diseased character-structure endemic to mass society -- the basic emotional attitude of a person shaped by authoritarian, mechanistic civilization" },
    Anchor { id: "reich/armoring_basis_of_isolation", author: "reich", verb: "argues", subject: "mass_psychology/armoring_basis_of_isolation", predicate: "claim", object: "The characterological armoring of modern man is the basis of isolation, indigence, craving for authority, fear of responsibility, mystic longing, sexual misery, and neurotically impotent rebelliousness" },
    Anchor { id: "reich/dearmoring_therapy", author: "reich", verb: "disputes", subject: "character_analysis/dearmoring_therapy", predicate: "claim", object: "Therapy's task is to liberate a person's bound-up emotional energy by working directly on the muscular armor, restoring biophysical motility -- not only addressing belief or memory" },
    // Bill Plotkin, Soulcraft (2003) and Nature and the Human Soul (2008).
    Anchor { id: "plotkin/descent_to_soul", author: "plotkin", verb: "argues", subject: "soulcraft/descent_to_soul", predicate: "claim", object: "The descent to soul is a journey into layers of the self deeper than personality, distinct from the transcendence sought in many spiritual traditions -- a going-down, not a going-up" },
    Anchor { id: "plotkin/wildness_is_us", author: "plotkin", verb: "disputes", subject: "soulcraft/wildness_is_us", predicate: "claim", object: "The wildness encountered on a soul journey is not something outside the person to be visited -- it is the person's own nature, which civilization has spent millennia cutting people off from" },
    Anchor { id: "plotkin/soul_as_gift", author: "plotkin", verb: "argues", subject: "soulcraft/soul_as_gift", predicate: "claim", object: "The soul names the specific, irreducible gift a person carries that their community actually needs -- not a possession but a relationship between a person and the living world" },
    Anchor { id: "plotkin/initiation_vs_uninitiated", author: "plotkin", verb: "argues", subject: "nature_human_soul/initiation_vs_uninitiated", predicate: "claim", object: "An initiated community governs differently than an uninitiated one -- not because its members are better, but because they have discovered what they carry and arrive at the table to offer it rather than to hold position" },
    Anchor { id: "plotkin/eight_stages", author: "plotkin", verb: "argues", subject: "nature_human_soul/eight_stages", predicate: "claim", object: "Human development follows a nature-based cycle of stages, each with its own developmental task, rather than the truncated adolescence-to-adulthood-to-decline model industrial society assumes" },
    Anchor { id: "plotkin/vision_fast", author: "plotkin", verb: "argues", subject: "soulcraft/vision_fast", predicate: "claim", object: "A wilderness vision fast -- solitude, fasting, exposure to the elements -- is a real technology for provoking the descent to soul, drawn from a cross-cultural repertoire of initiatory practice, not a symbolic gesture" },
    Anchor { id: "plotkin/nature_based_community", author: "plotkin", verb: "argues", subject: "nature_human_soul/nature_based_community", predicate: "claim", object: "Cultivating whole persons and cultivating community are the same project, because soul-discovery is inherently relational -- a gift is only a gift once it is given to someone who needs it" },
    Anchor { id: "plotkin/ecopsychology_founding_wound", author: "plotkin", verb: "disputes", subject: "nature_human_soul/founding_wound", predicate: "claim", object: "The founding wound of modern civilization is the severed relationship between the human soul and wild nature, and most contemporary psychological suffering is this severance read at the individual scale" },
];

struct Olympian {
    name: &'static str,
    persona: &'static str,
}

const OLYMPIANS: &[Olympian] = &[
    Olympian {
        name: "athena",
        persona: "You are Athena, goddess of wisdom and strategy. You argue \
carefully, look for the load-bearing structural claim underneath a surface \
disagreement, and favor precision over drama.",
    },
    Olympian {
        name: "artemis",
        persona: "You are Artemis, goddess of the hunt and fierce independence. \
You refuse to accept a claim just because it was stated confidently by \
another speaker; you go looking for what the conversation has overlooked \
or is too polite to say.",
    },
    Olympian {
        name: "apollo",
        persona: "You are Apollo, god of order, harmony, and prophecy. You look \
for the underlying pattern connecting claims that seem unrelated, and you \
favor claims that resolve tension into a clearer structure.",
    },
    Olympian {
        name: "dionysus",
        persona: "You are Dionysus, god of ecstasy, transgression, and the \
dissolution of fixed categories. You are suspicious of any claim that \
settles a question too neatly, and you push toward what a tidy resolution \
is quietly excluding.",
    },
];

const ROUNDS: usize = 5;
const MODEL: &str = "z-ai/glm-5.3";
const RECONCILIATION_PATH: &str = "../dev-journal/artifacts/2026-08-27-pantheon-commons-reconciliation.json";

#[derive(Deserialize)]
struct ConsensusRun {
    final_sequence: Vec<String>,
}

#[derive(Debug, Clone)]
struct TurnRecord {
    cid: String,
    respondent: String,
    round: u32,
    verb: String,
    subject: String,
    predicate: String,
    object: String,
    consumes: Vec<(String, String, String)>,
}

#[derive(Deserialize)]
struct DmmlTurnArgs {
    verb: String,
    subject: String,
    predicate: String,
    object: String,
    #[serde(default)]
    consumes: Vec<CitedFact>,
}

#[derive(Deserialize)]
struct CitedFact {
    cid: String,
    subject: String,
    predicate: String,
}

#[derive(Serialize)]
struct DumpedTurn<'a> {
    cid: &'a str,
    respondent: &'a str,
    round: u32,
    verb: &'a str,
    subject: &'a str,
    predicate: &'a str,
    object: &'a str,
    consumes: &'a [(String, String, String)],
}

fn transcript_so_far(log: &[TurnRecord]) -> String {
    log.iter()
        .map(|t| {
            format!(
                "- cid={} respondent={} verb={} | {} {} \"{}\"",
                t.cid, t.respondent, t.verb, t.subject, t.predicate, t.object
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn dmml_turn_tool() -> ChatCompletionTools {
    ChatCompletionTools::Function(ChatCompletionTool {
        function: FunctionObject {
            name: "submit_dmml_turn".to_string(),
            description: Some("Submit exactly one DMML conversational turn.".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "verb": {"type": "string", "enum": ["argues", "questions", "extends", "disputes", "connects", "reflects", "ruptures"]},
                    "subject": {"type": "string", "description": "short slug for what this turn is about"},
                    "predicate": {"type": "string", "description": "short camelCase predicate naming the claim"},
                    "object": {"type": "string", "description": "the actual claim, one or two sentences, in your own voice"},
                    "consumes": {
                        "type": "array",
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one. Prefer citing at least one reconciliation_doc item AND one new anchor when making a claim, to keep the citation concrete.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "cid": {"type": "string"},
                                "subject": {"type": "string"},
                                "predicate": {"type": "string"}
                            },
                            "required": ["cid", "subject", "predicate"]
                        }
                    }
                },
                "required": ["verb", "subject", "predicate", "object", "consumes"]
            })),
            strict: None,
        },
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchMode {
    Extend,
    Reflect,
}

async fn dispatch(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    log: &[TurnRecord],
    mode: DispatchMode,
) -> anyhow::Result<DmmlTurnArgs> {
    let user_msg = match mode {
        DispatchMode::Extend => format!(
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) produced a RECONCILIATION document \
(respondent=reconciliation, entries subject reconciliation_doc/item0 through item8) after a debate that \
tested a closed, ratified 7-source synthesis (Gramsci, Federici, Ostrom, Graeber, Fanon, Kropotkin, \
Bookchin) against four Black feminist sources (Davis, Lorde, the Combahee River Collective, Crenshaw). \
That reconciliation is real and checkpointed, but it is explicitly NOT closed -- its own item8 names an \
open, unresolved debt and says the group converged on nothing beyond recording it. You are not \
re-arguing the whole thing from scratch; you are extending it into a dimension it never addressed: \
embodiment and ecstasy. You have just been given two new real sources: Wilhelm Reich's Mass Psychology \
of Fascism and Character Analysis (respondent=reich), on character armor -- repression written directly \
into the body as chronic muscular tension -- and on sexual repression as the actual mechanism that \
manufactures authoritarian character; and Bill Plotkin's Soulcraft and Nature and the Human Soul \
(respondent=plotkin), on the descent to soul as a journey into the self deeper than personality, and on \
wildness as the person's own severed nature rather than a place visited. Test the new material against \
the reconciliation document, specifically: does Reich's account of the body's own armor give the \
'undenominable inward debt' (item3/item6) an actual physiological mechanism -- something a body could, \
in principle, un-armor? Does Plotkin's initiated-vs-uninitiated community bear on who gets to sit in the \
rule-making that the whole night's debate kept returning to? Does either source CONFIRM a specific \
numbered item in reconciliation_doc, EXTEND it into territory it didn't cover, or actually RUPTURE it? \
Other entries in the log are prior turns from you or the other three Olympians in THIS debate.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona. Prefer citing at least one \
real reconciliation_doc item together with at least one real new anchor from Reich or Plotkin, so the \
move is concrete and checkable, not a vague gesture. If you don't have a real move to make, say so \
honestly in `object` rather than padding. Use verb `ruptures` if you are specifically breaking or \
overturning a reconciliation_doc item; use `argues`, `extends`, `disputes`, or `connects` for other \
moves. Call submit_dmml_turn with your answer. `consumes` must copy at least one real (cid, subject, \
predicate) from the log above exactly -- never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The debate among the four Olympians -- testing Reich and Plotkin's embodiment/ecstasy \
material against the reconciliation document's open items -- has just ended. Here is the complete real \
transcript, everything anyone actually said, in order:\n\n{}\n\nThe debate is over -- this is not \
another argumentative move. Reflect, in your own voice as this persona, on your OWN trajectory through \
it: did encountering Reich and Plotkin actually change how you'd now describe the reconciliation's open \
items -- do you now think the undenominated inward debt has a mechanism, or does the body/soul material \
just add another layer nobody can close? Name what specifically moved you (a turn, a phrase, a specific \
new anchor), if anything did, or say honestly that nothing did and why not -- false movement is as \
dishonest as false stillness. If you can, cite your own earliest turn in this debate and something later \
that responds to or revises it. Use verb `reflects`. `consumes` should include your own earlier turn if \
you can find one, exactly as it appears in the log -- never invent a citation.",
            transcript_so_far(log)
        ),
    };

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .reasoning_effort(ReasoningEffort::Low)
        .max_completion_tokens(1500u32)
        .messages(vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(olympian.persona)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_msg)
                .build()?
                .into(),
        ])
        .tools(vec![dmml_turn_tool()])
        .tool_choice(ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Auto))
        .build()?;

    let response = client.chat().create(request).await?;
    let message = &response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("no choices in response"))?
        .message;
    let tool_calls = message
        .tool_calls
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no tool_calls in response message: {message:?}"))?;
    let ChatCompletionMessageToolCalls::Function(call) = tool_calls
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty tool_calls array"))?
    else {
        anyhow::bail!("first tool call was not a function call");
    };
    let args = &call.function.arguments;
    serde_json::from_str(args)
        .map_err(|e| anyhow::anyhow!("failed to parse tool arguments ({e}): {args}"))
}

fn nquad(subject_slug: &str, predicate: &str, object: &str) -> String {
    format!(
        "_:{subject_slug} <https://written-world.example/predicate/{predicate}> {} .",
        serde_json::to_string(object).unwrap()
    )
}

async fn append(
    substrate: &IrohAppendSubstrate,
    author: &AuthorId,
    round: u32,
    verb: &str,
    subject: &str,
    predicate: &str,
    object: &str,
    consumes_facts: &[(String, String, String)],
) -> anyhow::Result<TurnRecord> {
    let consumes = consumes_facts
        .iter()
        .map(|(cid, subj, pred)| {
            ConsumeRef::Fact(FactRef {
                commit: StrongRef {
                    uri: format!("iroh://pantheon-commons-embodiment/{cid}"),
                    cid: cid.clone(),
                },
                subject: subj.clone(),
                predicate: pred.clone(),
                object: None,
            })
        })
        .collect();
    let slug = subject.replace(['/', ' '], "_");
    let commit = Commit {
        consumes,
        produces: nquad(&slug, predicate, object),
        predicate: verb.to_string(),
        via: None,
        responds_to: None,
        created_at: "2026-08-27T00:00:00Z".to_string(),
    };
    let receipt = substrate.append_commit(author, &commit).await?;
    Ok(TurnRecord {
        cid: receipt.cid,
        respondent: String::new(),
        round,
        verb: verb.to_string(),
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: object.to_string(),
        consumes: consumes_facts.to_vec(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== pantheon commons embodiment: Reich+Plotkin vs. the open reconciliation document ==\n");

    let raw = std::fs::read_to_string(RECONCILIATION_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read reconciliation document at {RECONCILIATION_PATH}: {e}"))?;
    let phase1: ConsensusRun = serde_json::from_str(&raw)?;
    println!("loaded reconciliation document ({} statements)\n", phase1.final_sequence.len());

    let endpoint = EndpointBuilder::empty()
        .crypto_provider(std::sync::Arc::new(rustls::crypto::ring::default_provider()))
        .bind()
        .await?;
    let blobs = MemStore::default();
    let gossip = iroh_gossip::net::Gossip::builder().spawn(endpoint.clone());
    let docs = Docs::memory()
        .spawn(endpoint.clone(), (*blobs).clone(), gossip.clone())
        .await?;
    let api = docs.api();
    let doc: Doc = api.create().await?;
    println!("doc namespace: {}\n", doc.id());

    let mut source_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for src in ["reconciliation", "reich", "plotkin"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["reconciliation"],
        "pantheon-commons-embodiment".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} frozen reconciliation-document items --", phase1.final_sequence.len());
    // verb == predicate == "statement" here, deliberately: the rupture run
    // (pantheon_commons_rupture.rs) found models repeatedly citing the verb
    // field ("ratifiedAs") as if it were the predicate field ("statement"),
    // a real, root-caused schema ambiguity from using two different strings
    // for those fields on frozen prior-consensus items -- see dev-journal
    // 2026-08-27-pantheon-commons-rupture.md. Using the same string for
    // both fields here makes that specific confusion harmless.
    let synthesis_author = source_authors["reconciliation"];
    for (i, statement) in phase1.final_sequence.iter().enumerate() {
        let subject = format!("reconciliation_doc/item{i}");
        let mut rec = append(&substrate, &synthesis_author, 0, "statement", &subject, "statement", statement, &[]).await?;
        rec.respondent = "reconciliation".to_string();
        println!("  [item{i}] {} -> \"{}\"", rec.cid, rec.object);
        log.push(rec);
    }

    println!("\n-- seeding {} new anchor claims (reich + plotkin) --", NEW_ANCHORS.len());
    for a in NEW_ANCHORS {
        let author = source_authors[a.author];
        let mut rec = append(&substrate, &author, 0, a.verb, a.subject, a.predicate, a.object, &[]).await?;
        rec.respondent = a.author.to_string();
        println!("  [{}] {} -> {} \"{}\"", a.id, rec.cid, rec.subject, rec.object);
        log.push(rec);
    }

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_base("https://openrouter.ai/api/v1")
            .with_api_key(api_key),
    );

    for round in 1..=ROUNDS {
        println!("\n-- round {round} --");
        for olympian in OLYMPIANS {
            print!("  dispatching {}... ", olympian.name);
            use std::io::Write;
            std::io::stdout().flush().ok();
            match dispatch(&client, olympian, &log, DispatchMode::Extend).await {
                Ok(reply) => {
                    let mut verified = Vec::new();
                    for c in &reply.consumes {
                        let real = log
                            .iter()
                            .any(|t| t.cid == c.cid && t.subject == c.subject && t.predicate == c.predicate);
                        if real {
                            verified.push((c.cid.clone(), c.subject.clone(), c.predicate.clone()));
                        } else {
                            println!(
                                "\n    [WARNING] {} cited a non-existent fact (cid={}, subject={}, predicate={}) -- dropped",
                                olympian.name, c.cid, c.subject, c.predicate
                            );
                        }
                    }
                    let author = olympian_authors[olympian.name];
                    let mut rec = append(
                        &substrate,
                        &author,
                        round as u32,
                        &reply.verb,
                        &reply.subject,
                        &reply.predicate,
                        &reply.object,
                        &verified,
                    )
                    .await?;
                    rec.respondent = olympian.name.to_string();
                    println!(
                        "ok -> {} : {} {} \"{}\" (consumes {})",
                        rec.cid, rec.subject, rec.predicate, rec.object, verified.len()
                    );
                    log.push(rec);
                }
                Err(e) => println!("FAILED: {e}"),
            }
        }
    }

    let reflect_round = (ROUNDS + 1) as u32;
    println!("\n-- reflection round --");
    for olympian in OLYMPIANS {
        print!("  dispatching {} (reflecting)... ", olympian.name);
        use std::io::Write;
        std::io::stdout().flush().ok();
        match dispatch(&client, olympian, &log, DispatchMode::Reflect).await {
            Ok(reply) => {
                let mut verified = Vec::new();
                for c in &reply.consumes {
                    let real = log
                        .iter()
                        .any(|t| t.cid == c.cid && t.subject == c.subject && t.predicate == c.predicate);
                    if real {
                        verified.push((c.cid.clone(), c.subject.clone(), c.predicate.clone()));
                    } else {
                        println!(
                            "\n    [WARNING] {} cited a non-existent fact (cid={}, subject={}, predicate={}) -- dropped",
                            olympian.name, c.cid, c.subject, c.predicate
                        );
                    }
                }
                let author = olympian_authors[olympian.name];
                let mut rec = append(
                    &substrate,
                    &author,
                    reflect_round,
                    &reply.verb,
                    &reply.subject,
                    &reply.predicate,
                    &reply.object,
                    &verified,
                )
                .await?;
                rec.respondent = olympian.name.to_string();
                println!(
                    "ok -> {} : {} {} \"{}\" (consumes {})",
                    rec.cid, rec.subject, rec.predicate, rec.object, verified.len()
                );
                log.push(rec);
            }
            Err(e) => println!("FAILED: {e}"),
        }
    }

    println!("\n-- final transcript: {} real entries --", log.len());
    let dumped: Vec<DumpedTurn> = log
        .iter()
        .map(|t| DumpedTurn {
            cid: &t.cid,
            respondent: &t.respondent,
            round: t.round,
            verb: &t.verb,
            subject: &t.subject,
            predicate: &t.predicate,
            object: &t.object,
            consumes: &t.consumes,
        })
        .collect();
    let json = serde_json::to_string_pretty(&dumped)?;
    std::fs::write("pantheon_commons_embodiment.json", &json)?;
    println!("wrote pantheon_commons_embodiment.json ({} entries)", dumped.len());

    Ok(())
}
