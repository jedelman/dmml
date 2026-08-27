//! The "triple" -- Jason's follow-up to the Power Explained bibliography
//! survey: grep-verified citation density across the book's 1020 gesture
//! files put Gramsci (26), Federici (22), and Ostrom (17) at the top,
//! each with a real `G-DP-*` dramatis-personae profile in that repo.
//! This is the same debate/reflect/consensus/explain pipeline proven
//! tonight on Benjamin/Adorno, run against a real third source triangle
//! instead of a pair: hegemony (Gramsci), enclosure of the body
//! (Federici), and the durable commons (Ostrom) -- three arguments about
//! power and collective life that were never in dialogue with each
//! other historically, but bear directly on the same questions.
//!
//! Same discipline as `pantheon_olympians.rs`: one model (z-ai/glm-5.3),
//! four Olympian personas, tool-call-disciplined DMML turns, real
//! anchors only -- every claim below is either a verified direct quote
//! (see per-anchor sourcing) or an explicitly labeled paraphrase of a
//! well-documented argument, never invented. Sources:
//!
//! - Antonio Gramsci, *Prison Notebooks* (Quaderni del Carcere, written
//!   1929-1935; *Selections from the Prison Notebooks*, International
//!   Publishers, 1971 translation) -- hegemony, war of position, civil
//!   vs. political society, organic intellectuals, common sense.
//! - Silvia Federici, *Caliban and the Witch: Women, the Body and
//!   Primitive Accumulation* (Autonomedia, 2004) -- the enclosure of the
//!   commons, the witch hunts as capitalist discipline, the body as the
//!   site of primitive accumulation.
//! - Elinor Ostrom, *Governing the Commons: The Evolution of
//!   Institutions for Collective Action* (Cambridge University Press,
//!   1990) -- the eight design principles for durable commons
//!   governance, verified via secondary sources quoting the principles
//!   directly (two below are exact quotes; the rest are the
//!   well-documented paraphrase of each numbered principle, same
//!   register as the book's own G-DP-018 entry on Ostrom).

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
    author: &'static str, // "gramsci", "federici", or "ostrom"
    verb: &'static str,
    subject: &'static str,
    predicate: &'static str,
    object: &'static str,
}

const ANCHORS: &[Anchor] = &[
    // Gramsci, Prison Notebooks.
    Anchor { id: "gramsci/interregnum", author: "gramsci", verb: "asserts", subject: "notebooks/interregnum", predicate: "claim", object: "The crisis consists precisely in the fact that the old is dying and the new cannot be born; in this interregnum a great variety of morbid symptoms appear" },
    Anchor { id: "gramsci/spontaneous_consent", author: "gramsci", verb: "argues", subject: "notebooks/hegemony_consent", predicate: "claim", object: "The spontaneous consent given by the great masses of the population to the general direction imposed on social life by the dominant fundamental group is historically caused by the prestige which the dominant group enjoys because of its position and function in the world of production" },
    Anchor { id: "gramsci/civil_political_society", author: "gramsci", verb: "stipulates", subject: "notebooks/civil_political_society", predicate: "distinction", object: "Civil society, the ensemble of organisms commonly called private, corresponds to hegemony; political society, or the State, corresponds to direct domination or command" },
    Anchor { id: "gramsci/war_of_position", author: "gramsci", verb: "argues", subject: "notebooks/war_of_position", predicate: "claim", object: "In politics, the war of position, once won, is decisive definitively" },
    Anchor { id: "gramsci/organic_intellectuals", author: "gramsci", verb: "argues", subject: "notebooks/organic_intellectuals", predicate: "claim", object: "Every social group, coming into existence on the original terrain of an essential function in the world of economic production, creates together with itself, organically, one or more strata of intellectuals which give it homogeneity and awareness of its own function" },
    Anchor { id: "gramsci/common_sense", author: "gramsci", verb: "argues", subject: "notebooks/common_sense", predicate: "claim", object: "Common sense is not something rigid and immobile, but is continually transforming itself" },
    Anchor { id: "gramsci/philosophy_of_praxis", author: "gramsci", verb: "connects", subject: "notebooks/philosophy_of_praxis", predicate: "claim", object: "Philosophy is inseparable from the practical work of transforming common sense into a coherent, critical conception of the world -- his own coded term for Marxism, used to evade the prison censor" },
    Anchor { id: "gramsci/subaltern_history", author: "gramsci", verb: "asserts", subject: "notebooks/subaltern_classes", predicate: "claim", object: "The history of subaltern social groups is necessarily fragmented and episodic, legible mainly through the traces the dominant group's own history leaves behind" },
    // Federici, Caliban and the Witch.
    Anchor { id: "federici/enclosure_diet", author: "federici", verb: "argues", subject: "caliban/enclosure_subsistence", predicate: "claim", object: "Deprived of the commons that gave access to wood for fuel, to berries and herbs, and to small game and grazing land, peasants' diets declined significantly and starvation increased" },
    Anchor { id: "federici/enclosure_social", author: "federici", verb: "argues", subject: "caliban/enclosure_social_space", predicate: "claim", object: "Loss of the commons eliminated social space and unraveled family and community ties, hitting women hardest since they were less able to take to the roads for work and lacked independent means of subsistence" },
    Anchor { id: "federici/women_reclaim", author: "federici", verb: "asserts", subject: "caliban/women_reclaim_commons", predicate: "claim", object: "Women tore down hedges and fences and reclaimed the commons, they engaged in non-reproductive sex and led peasant revolts" },
    Anchor { id: "federici/witch_hunt_discipline", author: "federici", verb: "disputes", subject: "caliban/witch_hunts_as_discipline", predicate: "claim", object: "The witch hunts were not the dying superstitions of a feudal order but a deliberate tool to discipline and shape the emerging working class, integral to the transition to capitalism" },
    Anchor { id: "federici/witch_hunt_divide", author: "federici", verb: "argues", subject: "caliban/witch_hunts_divide_proletariat", predicate: "claim", object: "The witch hunts set the proletariat against itself, disciplining resistance from within rather than only through violence imposed from above" },
    Anchor { id: "federici/reproductive_labor", author: "federici", verb: "argues", subject: "caliban/reproductive_labor", predicate: "claim", object: "Women were constituted as reproductive laborers: not only bearing children but invisibly, and without pay, reproducing the conditions that let men go to work" },
    Anchor { id: "federici/marx_flaw", author: "federici", verb: "disputes", subject: "caliban/marx_primitive_accumulation", predicate: "claim", object: "Marx's account of primitive accumulation is flawed by its failure to see that the process required transforming the body into a work-machine and subjugating women to the reproduction of the workforce" },
    Anchor { id: "federici/body_self_ownership", author: "federici", verb: "argues", subject: "caliban/body_self_ownership", predicate: "claim", object: "The battle against the rebel body, and the conflict between body and mind, are essential conditions for the development of labor power and self-ownership, two central principles of modern social organization" },
    // Ostrom, Governing the Commons.
    Anchor { id: "ostrom/boundaries", author: "ostrom", verb: "stipulates", subject: "commons/design_principle_1_boundaries", predicate: "claim", object: "Clearly defined boundaries determine who holds rights to withdraw resource units from the common-pool resource" },
    Anchor { id: "ostrom/congruence", author: "ostrom", verb: "stipulates", subject: "commons/design_principle_2_congruence", predicate: "claim", object: "Appropriation and provision rules are congruent with local conditions rather than imposed uniformly" },
    Anchor { id: "ostrom/collective_choice", author: "ostrom", verb: "stipulates", subject: "commons/design_principle_3_collective_choice", predicate: "claim", object: "Most individuals affected by the operational rules can participate in modifying those rules" },
    Anchor { id: "ostrom/monitoring", author: "ostrom", verb: "stipulates", subject: "commons/design_principle_4_monitoring", predicate: "claim", object: "Monitors who actively audit resource conditions and appropriator behavior are accountable to the appropriators, or are the appropriators themselves" },
    Anchor { id: "ostrom/graduated_sanctions", author: "ostrom", verb: "stipulates", subject: "commons/design_principle_5_sanctions", predicate: "claim", object: "Appropriators who violate operational rules are likely to be assessed graduated sanctions, depending on the seriousness and context of the offense, by other appropriators, by officials accountable to these appropriators, or both" },
    Anchor { id: "ostrom/conflict_resolution", author: "ostrom", verb: "stipulates", subject: "commons/design_principle_6_conflict", predicate: "claim", object: "Appropriators have rapid access to low-cost local arenas to resolve conflicts among themselves or with officials" },
    Anchor { id: "ostrom/nested_enterprises", author: "ostrom", verb: "stipulates", subject: "commons/design_principle_8_nesting", predicate: "claim", object: "Appropriation, provision, monitoring, enforcement, conflict resolution, and governance activities are organised in multiple layers of nested enterprises" },
    Anchor { id: "ostrom/no_panacea", author: "ostrom", verb: "disputes", subject: "commons/no_panacea", predicate: "claim", object: "There is no panacea: successful commons governance depends on institutional diversity fit to local context, not a single universal design" },
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
const MODEL: &str = "z-ai/glm-5.3-flash";

#[derive(Debug, Clone)]
struct TurnRecord {
    cid: String,
    respondent: String,
    round: u32, // 0 for the seeded anchors, else the real round number
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
                    "verb": {"type": "string", "enum": ["argues", "questions", "extends", "disputes", "connects", "reflects"]},
                    "subject": {"type": "string", "description": "short slug for what this turn is about"},
                    "predicate": {"type": "string", "description": "short camelCase predicate naming the claim"},
                    "object": {"type": "string", "description": "the actual claim, one or two sentences, in your own voice"},
                    "consumes": {
                        "type": "array",
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one",
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
    Argue,
    Reflect,
}

async fn dispatch(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    log: &[TurnRecord],
    mode: DispatchMode,
) -> anyhow::Result<DmmlTurnArgs> {
    let user_msg = match mode {
        DispatchMode::Argue => format!(
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) are analyzing three real texts that \
were never in dialogue with each other historically, in a real, growing, checkable DMML commit log: \
Antonio Gramsci's Prison Notebooks (respondent=gramsci) on hegemony -- how domination works through a \
combination of consent and coercion, won or lost in a slow 'war of position'; Silvia Federici's \
Caliban and the Witch (respondent=federici) on the enclosure of the commons and the witch hunts as a \
deliberate discipline imposed on the body, especially women's bodies, to manufacture the modern \
worker; and Elinor Ostrom's Governing the Commons (respondent=ostrom) on the actual design principles \
-- boundaries, monitoring, graduated sanctions, nesting -- that let real commons survive enclosure and \
domination for centuries. You have full autonomy to interpret any one source, counter-interpret one \
against another, or find a synthesis none of the three states outright -- whatever a real reader with \
your temperament would actually do. A live real question worth pursuing: does Ostrom's institutional \
durability answer Gramsci's hegemony (a war of position the commons can actually win) or does Federici's \
account of the body's enclosure show a domain no institutional design principle reaches? Other entries \
in the log are prior turns from you or the other three Olympians.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona -- agreeing with, \
extending, disputing, or connecting something SPECIFIC already in the log (a claim from Gramsci, \
Federici, or Ostrom, or a prior Olympian's turn), in a way another Olympian with your temperament \
actually would. Not a summary of any source, not a restatement. If you don't have a real move to \
make, say so honestly in `object` rather than padding. Call submit_dmml_turn with your answer. \
`consumes` must copy at least one real (cid, subject, predicate) from the log above exactly -- \
never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The debate among the four Olympians (Athena, Artemis, Apollo, Dionysus) over Gramsci, \
Federici, and Ostrom has just ended. Here is the complete real transcript, everything anyone actually \
said, in order:\n\n{}\n\nThe debate is over -- this is not another argumentative move. Reflect, in your \
own voice as this persona, on your OWN trajectory through it, specifically: did encountering the other \
three Olympians' arguments -- and the real tension between hegemony, enclosure, and institutional \
design -- change your position from wherever you started, or did you end where you began? Name what \
specifically moved you (a turn, a phrase, an argument), if anything did, or say honestly that nothing \
did and why not -- false growth is worse than none. If you can, cite your own earliest turn and \
something later that responds to or revises it (a real, exact citation from the log, not invented) to \
show the actual shape of your own change, or its actual absence. Use verb `reflects`. `consumes` \
should include your own earlier turn if you can find one, exactly as it appears in the log -- never \
invent a citation.",
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
                    uri: format!("iroh://pantheon-commons/{cid}"),
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
    println!("== pantheon commons: Gramsci + Federici + Ostrom, 4 GLM-5.3 personas ==\n");

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
    for src in ["gramsci", "federici", "ostrom"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["gramsci"],
        "pantheon-commons".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} real anchor claims (gramsci + federici + ostrom) --", ANCHORS.len());
    for a in ANCHORS {
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
            match dispatch(&client, olympian, &log, DispatchMode::Argue).await {
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

    // Reflection round -- same act as pantheon_olympians.rs's, applied
    // to a real three-source debate instead of two.
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
    std::fs::write("pantheon_commons.json", &json)?;
    println!("wrote pantheon_commons.json ({} entries)", dumped.len());

    Ok(())
}
