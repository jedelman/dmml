//! Phase 2 of Jason's "argument, synthesis, rupture, reconciliation"
//! plan. Phase 1 (`pantheon_commons7.rs`) produced a ratified 7-source
//! synthesis (Gramsci, Federici, Ostrom, Graeber, Fanon, Kropotkin,
//! Bookchin). This phase does NOT pre-seed new anchors alongside the
//! old ones the way every prior scale-up did -- it seeds the ratified
//! phase-1 synthesis itself as frozen, citable commits (author =
//! `synthesis_phase1`, one per statement), then seeds 32 new anchors
//! from four Black feminist sources absent from phase 1: Angela Davis
//! (*Women, Race and Class*, 1981), Audre Lorde (*Sister Outsider*,
//! 1984), the Combahee River Collective Statement (1977), and Kimberle
//! Crenshaw ("Demarginalizing the Intersection of Race and Sex", 1989).
//!
//! The debate mode is deliberately different from every prior
//! `pantheon_*` run: `DispatchMode::Rupture` explicitly asks each
//! Olympian to test the new material against SPECIFIC numbered items in
//! the frozen phase-1 synthesis -- does this new source confirm,
//! extend, or actually break a claim the group already ratified? -- not
//! a free-floating new argument. This is the structural difference
//! between "synthesis" (this repo's other `pantheon_*consensus.rs`
//! files, closing an open debate) and "rupture" (opening a CLOSED one
//! back up with material it did not have when it closed).
//!
//! `pantheon_commons_reconciliation.rs` is the counterpart: it takes
//! THIS run's transcript and attempts a new ratification, with explicit
//! room to report irreconcilable points honestly rather than force a
//! tidier synthesis than the material supports.

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

// The 32 new anchors. Every claim is either a verified direct quote or
// an explicitly labeled paraphrase of a well-documented argument, same
// discipline as every anchor in this project.
const NEW_ANCHORS: &[Anchor] = &[
    // Angela Davis, Women, Race and Class (1981).
    Anchor { id: "davis/slave_women_field_equals", author: "davis", verb: "argues", subject: "women_race_class/field_labor_equality", predicate: "claim", object: "During slavery, women labored alongside men in the cotton and tobacco fields as their functional equals -- the sexual division of domestic labor under slavery was not hierarchical, since survival required both" },
    Anchor { id: "davis/reproductive_capacity_as_instrument", author: "davis", verb: "argues", subject: "women_race_class/reproductive_capacity", predicate: "claim", object: "Once the international slave trade was abolished, slaveholders placed a premium on enslaved women's reproductive capacity -- in the slaveholder's eyes, enslaved women were not mothers but instruments guaranteeing the growth of the slave labor force" },
    Anchor { id: "davis/domestic_labor_only_meaningful_labor", author: "davis", verb: "argues", subject: "women_race_class/domestic_labor_for_the_community", predicate: "claim", object: "Domestic labor performed within the slave quarters was the only labor the enslaved community could perform for itself rather than for the master, making it the germ of resistance as much as of subsistence" },
    Anchor { id: "davis/double_shift", author: "davis", verb: "argues", subject: "women_race_class/double_shift", predicate: "claim", object: "After emancipation, Black women bore a double exploitation -- paid labor for white employers by day, unpaid domestic labor for their own families by night -- a pattern white feminism's domestic-labor critique routinely erased" },
    Anchor { id: "davis/white_feminism_racial_blindspot", author: "davis", verb: "disputes", subject: "women_race_class/white_feminism_blindspot", predicate: "claim", object: "The 19th and 20th century women's movements, when led by white middle-class women, repeatedly subordinated or betrayed the demands of Black and working-class women in order to secure their own gains" },
    Anchor { id: "davis/rape_as_control", author: "davis", verb: "argues", subject: "women_race_class/rape_as_political_control", predicate: "claim", object: "The systematic rape of enslaved women functioned as an instrument of political domination and terror, not an incidental abuse -- controlling Black women's bodies was inseparable from controlling the enslaved labor force as a whole" },
    Anchor { id: "davis/myth_of_the_black_rapist", author: "davis", verb: "disputes", subject: "women_race_class/myth_of_the_black_rapist", predicate: "claim", object: "The 20th-century myth of the Black rapist was manufactured to justify lynching, functioning as political terror against the whole Black community rather than a response to any real pattern of crime" },
    Anchor { id: "davis/housework_industrialization", author: "davis", verb: "argues", subject: "women_race_class/industrialize_housework", predicate: "claim", object: "Housework should be industrialized and socialized -- cooking, cleaning, and childcare converted into a public, waged, collective service -- rather than perpetually re-privatized within individual households" },
    // Audre Lorde, Sister Outsider (1984).
    Anchor { id: "lorde/masters_tools", author: "lorde", verb: "asserts", subject: "sister_outsider/masters_tools", predicate: "claim", object: "The master's tools will never dismantle the master's house" },
    Anchor { id: "lorde/erotic_as_power", author: "lorde", verb: "argues", subject: "sister_outsider/erotic_as_power", predicate: "claim", object: "The erotic is not sexuality alone but a deep source of power and knowledge, a capacity for feeling fully that patriarchal and racist society trains women, and especially Black women, to distrust and suppress" },
    Anchor { id: "lorde/difference_as_resource", author: "lorde", verb: "disputes", subject: "sister_outsider/difference_as_resource", predicate: "claim", object: "Differences among women -- race, class, sexuality, age -- are not a threat to feminist solidarity to be smoothed over, but a genuine resource, and pretending they don't exist is itself a tool of domination" },
    Anchor { id: "lorde/silence_will_not_protect_you", author: "lorde", verb: "asserts", subject: "sister_outsider/silence_will_not_protect_you", predicate: "claim", object: "Your silence will not protect you" },
    Anchor { id: "lorde/anger_as_information", author: "lorde", verb: "argues", subject: "sister_outsider/anger_as_information", predicate: "claim", object: "Women's anger, especially Black women's anger at racism within feminism itself, is a legitimate and useful source of information and energy, not a failure of sisterhood to be managed away" },
    Anchor { id: "lorde/not_a_single_issue", author: "lorde", verb: "disputes", subject: "sister_outsider/not_a_single_issue", predicate: "claim", object: "There is no such thing as a single-issue struggle, because oppressions are interlocking and people do not live single-issue lives" },
    Anchor { id: "lorde/outsider_within", author: "lorde", verb: "argues", subject: "sister_outsider/outsider_within", predicate: "claim", object: "Writing from the position of multiple simultaneous outsider identities -- Black, lesbian, mother, warrior, poet -- is not a disqualification from theory but a source of clearer sight" },
    Anchor { id: "lorde/poetry_not_luxury", author: "lorde", verb: "argues", subject: "sister_outsider/poetry_not_luxury", predicate: "claim", object: "Poetry is not a luxury for women, but a vital necessity of existence -- the way the formless, the still-unnamed, first gets brought into the light of thought" },
    // Combahee River Collective Statement (1977).
    Anchor { id: "crc/interlocking_oppressions", author: "crc", verb: "asserts", subject: "combahee/interlocking_oppressions", predicate: "claim", object: "The collective commits to struggling against racial, sexual, heterosexual, and class oppression together, on the ground that the major systems of oppression are interlocking, and their synthesis creates the actual conditions of Black women's lives" },
    Anchor { id: "crc/identity_politics_coined", author: "crc", verb: "asserts", subject: "combahee/identity_politics_coined", predicate: "claim", object: "The statement is among the first documents to name and define identity politics -- the belief that the most profound and potentially most radical politics come directly out of one's own identity, rather than out of working to end someone else's oppression" },
    Anchor { id: "crc/not_biological_determinism", author: "crc", verb: "disputes", subject: "combahee/not_biological_determinism", predicate: "claim", object: "The collective explicitly rejects biological determinism or separatism as a political strategy, distinguishing their identity-grounded politics from any claim that oppression is rooted in fixed biological categories" },
    Anchor { id: "crc/black_feminism_not_reducible", author: "crc", verb: "disputes", subject: "combahee/black_feminism_not_reducible", predicate: "claim", object: "Black feminism cannot be understood as an offshoot of white feminism or a mere addition of race to existing feminist theory -- it is its own, independently necessary politics" },
    Anchor { id: "crc/socialist_but_not_only_class", author: "crc", verb: "argues", subject: "combahee/socialist_but_not_only_class", predicate: "claim", object: "The collective identifies as socialist, believing work must be organized for the collective benefit of the workers who produce goods and services, but insists racial and sexual oppression are not simply byproducts of capitalism" },
    Anchor { id: "crc/skepticism_of_lesbian_separatism", author: "crc", verb: "disputes", subject: "combahee/skepticism_of_lesbian_separatism", predicate: "claim", object: "The collective is critical of lesbian separatism as a strategy, arguing it leaves behind working-class and Black male family and community ties in a way white lesbian separatists are not forced to reckon with" },
    Anchor { id: "crc/coalition_not_purity", author: "crc", verb: "argues", subject: "combahee/coalition_not_purity", predicate: "claim", object: "The collective commits to coalition work with other progressive organizations rather than insisting on ideological purity, even while centering their own specific analysis" },
    Anchor { id: "crc/personal_is_political_grounded", author: "crc", verb: "argues", subject: "combahee/personal_is_political_grounded", predicate: "claim", object: "The statement grounds 'the personal is political' in the specific, materially dangerous conditions of Black women's daily lives, not as an abstract feminist slogan" },
    // Kimberle Crenshaw, "Demarginalizing the Intersection of Race and Sex" (1989).
    Anchor { id: "crenshaw/traffic_metaphor", author: "crenshaw", verb: "asserts", subject: "demarginalizing/traffic_metaphor", predicate: "claim", object: "Discrimination, like traffic through an intersection, may flow in one direction, and it may flow in another; if an accident happens in an intersection, it can be caused by cars traveling from any number of directions" },
    Anchor { id: "crenshaw/degrx_v_general_motors", author: "crenshaw", verb: "argues", subject: "demarginalizing/degraffenreid_v_gm", predicate: "claim", object: "In DeGraffenreid v. General Motors, Black women plaintiffs were denied standing to claim discrimination as Black women specifically, because the court could recognize race discrimination or sex discrimination but not their intersection" },
    Anchor { id: "crenshaw/single_axis_framework", author: "crenshaw", verb: "disputes", subject: "demarginalizing/single_axis_framework", predicate: "claim", object: "Antidiscrimination law and feminist theory are both dominated by a single-axis framework that treats race and gender as separate, mutually exclusive categories of experience and analysis" },
    Anchor { id: "crenshaw/most_privileged_member_problem", author: "crenshaw", verb: "argues", subject: "demarginalizing/most_privileged_member_problem", predicate: "claim", object: "When a group's experience is defined by its most privileged members, the specific harms facing those at the intersection of multiple subordinated identities become structurally invisible to both antiracist and feminist frameworks" },
    Anchor { id: "crenshaw/intersectionality_not_additive", author: "crenshaw", verb: "disputes", subject: "demarginalizing/intersectionality_not_additive", predicate: "claim", object: "Intersectionality is not simply race-discrimination-plus-sex-discrimination added together; it names a qualitatively distinct position and experience that neither category alone can capture" },
    Anchor { id: "crenshaw/antidiscrimination_law_limits", author: "crenshaw", verb: "argues", subject: "demarginalizing/antidiscrimination_law_limits", predicate: "claim", object: "Antidiscrimination law's own doctrinal structure, built to process one ground of discrimination at a time, actively produces the erasure of intersectional claims rather than merely failing to notice them" },
    Anchor { id: "crenshaw/feminist_theory_blindspot", author: "crenshaw", verb: "disputes", subject: "demarginalizing/feminist_theory_blindspot", predicate: "claim", object: "Mainstream feminist theory of the period, built largely from white women's experience, replicated the same single-axis blindness as antidiscrimination law, treating Black women's experience as a variation on a white norm rather than its own site of analysis" },
    Anchor { id: "crenshaw/coalition_reframe", author: "crenshaw", verb: "argues", subject: "demarginalizing/coalition_reframe", predicate: "claim", object: "The goal is not just adding Black women to existing frameworks but restructuring the frameworks themselves so intersectional experience becomes visible as a starting point, not an afterthought" },
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
const PHASE1_CONSENSUS_PATH: &str = "../dev-journal/artifacts/2026-08-27-pantheon-commons7-consensus.json";

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
                        "description": "at least one EXACT (cid, subject, predicate) copied from the log you were shown -- never invent one. Prefer citing at least one phase1_synthesis item AND one new anchor when making a rupture claim, to keep the citation concrete.",
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
    Rupture,
    Reflect,
}

async fn dispatch(
    client: &Client<OpenAIConfig>,
    olympian: &Olympian,
    log: &[TurnRecord],
    mode: DispatchMode,
) -> anyhow::Result<DmmlTurnArgs> {
    let user_msg = match mode {
        DispatchMode::Rupture => format!(
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) already reached and unanimously \
ratified a synthesis (respondent=synthesis_phase1, entries subject phase1_synthesis/item0 through \
item7) after debating seven real texts: Gramsci, Federici, Ostrom, Graeber, Fanon, Kropotkin, and \
Bookchin. That synthesis is now CLOSED and frozen -- you are not re-arguing it from scratch. Your task \
this time is different: you have just been given four new real sources that synthesis never had access \
to: Angela Davis's Women, Race and Class (respondent=davis) on the double exploitation of Black women's \
labor under slavery and after, and white feminism's historical betrayals of Black women's demands; \
Audre Lorde's Sister Outsider (respondent=lorde) arguing 'the master's tools will never dismantle the \
master's house,' that difference among women is a resource rather than a threat, and that there is no \
such thing as a single-issue struggle; the Combahee River Collective Statement (respondent=crc), which \
coined 'identity politics' and 'interlocking oppression' and insists Black feminism is not reducible to \
white feminism plus race; and Kimberle Crenshaw's 'Demarginalizing the Intersection of Race and Sex' \
(respondent=crenshaw), naming intersectionality and the single-axis framework's structural blindness to \
the women it fails to see at all. Test the new material against the OLD, CLOSED synthesis, specifically: \
does a new source CONFIRM a specific numbered item in phase1_synthesis, EXTEND it into territory it \
didn't cover, or actually RUPTURE it -- expose it as wrong, incomplete in a way that changes its meaning, \
or blind to something it should have named? Naming a real rupture is not a failure of the group's prior \
work; papering over one to protect the old synthesis would be. Other entries in the log are prior turns \
from you or the other three Olympians in THIS rupture debate.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona. Prefer citing at least one \
real phase1_synthesis item together with at least one real new anchor from Davis, Lorde, CRC, or \
Crenshaw, so the rupture (or confirmation, or extension) is concrete and checkable, not a vague gesture. \
If you don't have a real move to make, say so honestly in `object` rather than padding. Use verb \
`ruptures` if you are specifically breaking or overturning a phase1_synthesis item; use `argues`, \
`extends`, `disputes`, or `connects` for other moves. Call submit_dmml_turn with your answer. `consumes` \
must copy at least one real (cid, subject, predicate) from the log above exactly -- never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The rupture debate among the four Olympians -- testing Davis, Lorde, the Combahee River \
Collective, and Crenshaw against the closed, ratified 7-source synthesis -- has just ended. Here is the \
complete real transcript, everything anyone actually said, in order:\n\n{}\n\nThe debate is over -- this \
is not another argumentative move. Reflect, in your own voice as this persona, on your OWN trajectory \
through it: did encountering these four new sources actually change how you'd now describe the old, \
ratified synthesis -- do you now think a specific item in it was wrong, or merely incomplete, or does it \
hold exactly as ratified? Name what specifically moved you (a turn, a phrase, a specific new anchor), if \
anything did, or say honestly that nothing did and why not -- false rupture is as dishonest as false \
harmony. If you can, cite your own earliest turn in this rupture debate and something later that \
responds to or revises it. Use verb `reflects`. `consumes` should include your own earlier turn if you \
can find one, exactly as it appears in the log -- never invent a citation.",
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
                    uri: format!("iroh://pantheon-commons-rupture/{cid}"),
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
    println!("== pantheon commons rupture: Davis+Lorde+CRC+Crenshaw vs. the closed phase-1 synthesis ==\n");

    let raw = std::fs::read_to_string(PHASE1_CONSENSUS_PATH)
        .map_err(|e| anyhow::anyhow!("couldn't read phase-1 consensus at {PHASE1_CONSENSUS_PATH}: {e}"))?;
    let phase1: ConsensusRun = serde_json::from_str(&raw)?;
    println!("loaded phase-1 ratified synthesis ({} statements)\n", phase1.final_sequence.len());

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
    for src in ["synthesis_phase1", "davis", "lorde", "crc", "crenshaw"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["synthesis_phase1"],
        "pantheon-commons-rupture".to_string(),
        doc.clone(),
        (*blobs).clone(),
    );

    let mut log: Vec<TurnRecord> = Vec::new();

    println!("-- seeding {} frozen phase-1 synthesis items --", phase1.final_sequence.len());
    let synthesis_author = source_authors["synthesis_phase1"];
    for (i, statement) in phase1.final_sequence.iter().enumerate() {
        let subject = format!("phase1_synthesis/item{i}");
        let mut rec = append(&substrate, &synthesis_author, 0, "ratifiedAs", &subject, "statement", statement, &[]).await?;
        rec.respondent = "synthesis_phase1".to_string();
        println!("  [item{i}] {} -> \"{}\"", rec.cid, rec.object);
        log.push(rec);
    }

    println!("\n-- seeding {} new anchor claims (davis + lorde + crc + crenshaw) --", NEW_ANCHORS.len());
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
            match dispatch(&client, olympian, &log, DispatchMode::Rupture).await {
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
    std::fs::write("pantheon_commons_rupture.json", &json)?;
    println!("wrote pantheon_commons_rupture.json ({} entries)", dumped.len());

    Ok(())
}
