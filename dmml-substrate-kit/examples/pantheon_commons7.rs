//! Phase 1 of Jason's "argument, synthesis, rupture, reconciliation"
//! plan: "two phases - Kropotkin/Bookchin, then the Black feminists -
//! Davis, Lorde, CRC, Crenshaw." This phase adds Kropotkin and Bookchin
//! to the existing five-source graph -- both real anchors of the same
//! commons/domination/mutual-aid lineage Gramsci/Federici/Ostrom/
//! Graeber/Fanon already argue within, extending the synthesis rather
//! than rupturing it. The rupture is reserved for phase 2
//! (`pantheon_commons_rupture.rs`), which takes THIS run's ratified
//! synthesis as a frozen prior consensus and introduces the Black
//! feminist anchors as a deliberate disruption to be reconciled or
//! honestly left unreconciled, not pre-seeded alongside everything else.
//!
//! Same debate/reflect/consensus/explain pipeline, now against seven
//! real sources -- 56 anchors instead of 40 -- extending the citation-
//! degradation curve from `dev-journal/2026-08-27-pantheon-commons5.md`
//! a fourth data point: 16 (0/8 zero-citation) -> 24 (4/24) -> 40 (3/24)
//! -> 56 anchors here.
//!
//! Same discipline as every `pantheon_*` example: one model
//! (z-ai/glm-5.3), four Olympian personas, tool-call-disciplined DMML
//! turns, real anchors only -- every claim below is either a verified
//! direct quote (see per-anchor sourcing) or an explicitly labeled
//! paraphrase of a well-documented argument, never invented. Sources:
//!
//! - Antonio Gramsci, *Prison Notebooks* -- hegemony, war of position,
//!   civil vs. political society, organic intellectuals, common sense.
//! - Silvia Federici, *Caliban and the Witch* (2004) -- the enclosure of
//!   the commons, the witch hunts as capitalist discipline, the body as
//!   the site of primitive accumulation.
//! - Elinor Ostrom, *Governing the Commons* (1990) -- the eight design
//!   principles for durable commons governance.
//! - David Graeber, *Debt: The First 5000 Years* (2011) and *The Utopia
//!   of Rules* (2015) -- baseline communism as the substrate beneath
//!   exchange and hierarchy, the myths of barter and primordial debt,
//!   bureaucracy as organized structural violence.
//! - Frantz Fanon, *The Wretched of the Earth* (1961) and *Black Skin,
//!   White Masks* (1952) -- colonialism as naked violence, the
//!   epidermalization of imposed racial inferiority, national culture as
//!   inseparable from the fight for national liberation.
//! - Pyotr Kropotkin, *Mutual Aid: A Factor of Evolution* (1902) -- the
//!   direct rebuttal of Hobbesian/social-Darwinist competition,
//!   mutual aid as instinct rather than achieved morality, the guild and
//!   the commune as its historical institutions.
//! - Murray Bookchin, *The Ecology of Freedom* (1982) -- hierarchy as
//!   more fundamental than class, the domination of nature as an
//!   extension of the domination of humans by humans, libertarian
//!   municipalism as the constructive alternative.

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
    // Graeber, Debt: The First 5000 Years / The Utopia of Rules.
    Anchor { id: "graeber/baseline_communism", author: "graeber", verb: "asserts", subject: "debt/baseline_communism", predicate: "claim", object: "Baseline communism names the taken-for-granted recognition of mutual dependence that undergirds social peace -- some things are simply shared, and no one keeps accounts" },
    Anchor { id: "graeber/three_moral_principles", author: "graeber", verb: "argues", subject: "debt/three_moral_principles", predicate: "claim", object: "Communism, hierarchy, and exchange are three moral principles present in different degrees within every real economic relationship, not three separate historical systems" },
    Anchor { id: "graeber/myth_of_barter", author: "graeber", verb: "disputes", subject: "debt/myth_of_barter", predicate: "claim", object: "There never has been, and never will be, any society whose economy was based primarily on barter -- the economists' founding myth of money has no historical or ethnographic support" },
    Anchor { id: "graeber/primordial_debt_myth", author: "graeber", verb: "disputes", subject: "debt/primordial_debt_myth", predicate: "claim", object: "The myth of primordial debt -- that each of us is born indebted to our parents, our ancestors, our god, or our nation -- launders a moral claim as though it were simply a fact of existence" },
    Anchor { id: "graeber/credit_before_money", author: "graeber", verb: "argues", subject: "debt/credit_before_money", predicate: "claim", object: "Credit and debt precede money and barter historically, reversing the standard economic origin story" },
    Anchor { id: "graeber/structural_violence", author: "graeber", verb: "argues", subject: "utopia_of_rules/structural_violence", predicate: "claim", object: "Bureaucracies are not themselves forms of stupidity so much as ways of organizing stupidity -- of managing relationships already characterized by extremely unequal structures of imagination, which exist because of structural violence" },
    Anchor { id: "graeber/lopsided_imagination", author: "graeber", verb: "asserts", subject: "utopia_of_rules/lopsided_imagination", predicate: "claim", object: "Structural violence creates lopsided structures of the imagination" },
    Anchor { id: "graeber/violence_rests_on_force", author: "graeber", verb: "argues", subject: "utopia_of_rules/violence_rests_on_force", predicate: "claim", object: "Structural violence is a system that ultimately rests on the threat of force, which is why its administration can appear boring and procedural rather than overtly violent" },
    // Fanon, The Wretched of the Earth / Black Skin, White Masks.
    Anchor { id: "fanon/decolonization_violence", author: "fanon", verb: "asserts", subject: "wretched/decolonization_violence", predicate: "claim", object: "In its bare reality, decolonization reeks of red-hot cannonballs and bloody knives -- the last can be first only after a murderous and decisive confrontation between the two protagonists" },
    Anchor { id: "fanon/starving_peasant", author: "fanon", verb: "argues", subject: "wretched/starving_peasant", predicate: "claim", object: "The starving peasant, outside the class system, is the first among the exploited to discover that only violence pays" },
    Anchor { id: "fanon/colonialism_naked_violence", author: "fanon", verb: "asserts", subject: "wretched/colonialism_naked_violence", predicate: "claim", object: "Colonialism is naked violence, and only gives way when confronted with greater violence" },
    Anchor { id: "fanon/epidermalization", author: "fanon", verb: "argues", subject: "black_skin/epidermalization", predicate: "claim", object: "Racist perception replaces a person's pragmatic bodily schema with a racial epidermal schema -- the epidermalization of an imposed inferiority" },
    Anchor { id: "fanon/look_a_negro", author: "fanon", verb: "asserts", subject: "black_skin/look_a_negro", predicate: "claim", object: "A white child's cry of fear on a train, 'Look, a Negro!', fractures Fanon's own bodily self-possession into a fixed racial object, seen from the outside before he can act" },
    Anchor { id: "fanon/national_culture_liberation", author: "fanon", verb: "argues", subject: "wretched/national_culture_liberation", predicate: "claim", object: "Fighting for national culture first means fighting for the liberation of the nation -- the tangible matrix from which any culture can actually grow" },
    Anchor { id: "fanon/precolonial_culture_unreclaimable", author: "fanon", verb: "disputes", subject: "wretched/precolonial_culture_unreclaimable", predicate: "claim", object: "The colonized intellectual's return to a precolonial past cannot simply restore what colonialism erased -- the past is not waiting there intact to be recovered" },
    Anchor { id: "fanon/rootless_without_nation", author: "fanon", verb: "asserts", subject: "wretched/rootless_without_nation", predicate: "claim", object: "Without a nation, a people are left without anchorage, without borders, colorless, stateless, rootless" },
    // Kropotkin, Mutual Aid: A Factor of Evolution.
    Anchor { id: "kropotkin/law_of_mutual_aid", author: "kropotkin", verb: "asserts", subject: "mutual_aid/law_of_mutual_aid", predicate: "claim", object: "Mutual aid is as much a law of animal life as mutual struggle, but as a factor of evolution it most probably has a far greater importance, favouring the habits and characters that ensure the species' maintenance and the greatest welfare of the individual with the least waste of energy" },
    Anchor { id: "kropotkin/against_survival_of_fittest", author: "kropotkin", verb: "disputes", subject: "mutual_aid/against_survival_of_fittest", predicate: "claim", object: "Kropotkin directly challenges the Darwinian emphasis on competition as evolution's primary driver, arguing cooperation within a species is the dominant survival strategy actually observed in nature" },
    Anchor { id: "kropotkin/medieval_guilds", author: "kropotkin", verb: "argues", subject: "mutual_aid/medieval_guilds", predicate: "claim", object: "The medieval free city and its guilds show mutual aid organized at civilizational scale -- self-governed federations of craft and neighborhood, before the rise of the centralized state suppressed them" },
    Anchor { id: "kropotkin/state_destroys_commons", author: "kropotkin", verb: "disputes", subject: "mutual_aid/state_as_destroyer_of_commons", predicate: "claim", object: "The modern state's historic role was to destroy the self-organized mutual-aid institutions of the village commune and the guild, concentrating into itself functions communities had performed for themselves" },
    Anchor { id: "kropotkin/mutual_aid_survives_underground", author: "kropotkin", verb: "argues", subject: "mutual_aid/survives_underground", predicate: "claim", object: "Even under the modern state, mutual aid never disappeared -- it persisted informally in trade unions, friendly societies, and neighborly custom, wherever the state's reach was thin" },
    Anchor { id: "kropotkin/instinct_not_morality", author: "kropotkin", verb: "asserts", subject: "mutual_aid/instinct_not_morality", predicate: "claim", object: "Mutual aid is not primarily a moral achievement layered onto a naturally competitive species; it is an instinct as deep-rooted as the struggle for existence itself" },
    Anchor { id: "kropotkin/critique_of_hobbes", author: "kropotkin", verb: "disputes", subject: "mutual_aid/critique_of_hobbes", predicate: "claim", object: "Kropotkin's law of mutual aid was framed as a direct rebuttal of the Hobbesian view that pre-social life was a war of each against all" },
    Anchor { id: "kropotkin/no_need_for_state", author: "kropotkin", verb: "argues", subject: "mutual_aid/no_need_for_state", predicate: "claim", object: "Since mutual aid is a natural, sufficient basis for large-scale cooperation, the centralized state is not history's answer to disorder but the historical destroyer of an order that already existed" },
    // Bookchin, The Ecology of Freedom.
    Anchor { id: "bookchin/domination_of_nature_from_domination_of_human", author: "bookchin", verb: "asserts", subject: "ecology_of_freedom/domination_of_nature", predicate: "claim", object: "The very notion of the domination of nature by man stems from the very real domination of human by human" },
    Anchor { id: "bookchin/hierarchy_precedes_class", author: "bookchin", verb: "argues", subject: "ecology_of_freedom/hierarchy_precedes_class", predicate: "claim", object: "Hierarchy -- age-based, gender-based, generational stratification -- historically preceded economic class as a category, and is the more fundamental obstacle to freedom" },
    Anchor { id: "bookchin/hierarchy_vs_class_terminology", author: "bookchin", verb: "stipulates", subject: "ecology_of_freedom/hierarchy_vs_class_terminology", predicate: "distinction", object: "Bookchin deliberately used 'hierarchy' rather than 'class' or 'the State' to name the deeper structure, because class and state are only hierarchy's most familiar historical costumes, not its origin" },
    Anchor { id: "bookchin/first_second_nature", author: "bookchin", verb: "argues", subject: "ecology_of_freedom/first_second_nature", predicate: "claim", object: "Humanity is 'second nature' -- nature rendered self-conscious of itself -- and social ecology's task is to make second nature the deliberate, rational steward of first nature rather than its plunderer" },
    Anchor { id: "bookchin/organic_society", author: "bookchin", verb: "argues", subject: "ecology_of_freedom/organic_society", predicate: "claim", object: "Pre-hierarchical organic societies governed themselves through usufruct, the irreducible minimum, and complementarity -- custom rooted in interdependence, not formal law or coercive authority" },
    Anchor { id: "bookchin/libertarian_municipalism", author: "bookchin", verb: "argues", subject: "ecology_of_freedom/libertarian_municipalism", predicate: "claim", object: "Real freedom is built from the bottom up through direct, face-to-face democratic assemblies at the municipal scale, confederated upward, rather than delegated to a centralized state or vanguard party" },
    Anchor { id: "bookchin/technology_not_neutral", author: "bookchin", verb: "disputes", subject: "ecology_of_freedom/technology_not_neutral", predicate: "claim", object: "Technology and social organization are not neutral tools available to any social order; the same technical capacity is put to radically different uses depending on whether hierarchy or mutual complementarity organizes the society wielding it" },
    Anchor { id: "bookchin/ecological_crisis_is_social", author: "bookchin", verb: "argues", subject: "ecology_of_freedom/ecological_crisis_is_social", predicate: "claim", object: "The ecological crisis is at root a social crisis -- the domination of nature cannot be solved by technical fixes alone as long as human domination of humans remains intact" },
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
            "Four Olympians (Athena, Artemis, Apollo, Dionysus) are analyzing seven real texts that \
were never in dialogue with each other historically, in a real, growing, checkable DMML commit log: \
Antonio Gramsci's Prison Notebooks (respondent=gramsci) on hegemony -- how domination works through a \
combination of consent and coercion, won or lost in a slow 'war of position'; Silvia Federici's \
Caliban and the Witch (respondent=federici) on the enclosure of the commons and the witch hunts as a \
deliberate discipline imposed on the body, especially women's bodies, to manufacture the modern \
worker; Elinor Ostrom's Governing the Commons (respondent=ostrom) on the actual design principles -- \
boundaries, monitoring, graduated sanctions, nesting -- that let real commons survive enclosure and \
domination for centuries; David Graeber's Debt and The Utopia of Rules (respondent=graeber) on baseline \
communism as the substrate beneath all exchange and hierarchy, and on bureaucracy as organized \
structural violence dressed as boredom; Frantz Fanon's The Wretched of the Earth and Black Skin, \
White Masks (respondent=fanon) on colonialism as naked violence, and on the epidermalization of an \
imposed racial inferiority that no institutional design reaches; Pyotr Kropotkin's Mutual Aid \
(respondent=kropotkin) arguing cooperation, not competition, is nature's dominant law, and that the \
centralized state historically destroyed the mutual-aid institutions -- guild, commune -- it claims to \
have been necessary to replace; and Murray Bookchin's The Ecology of Freedom (respondent=bookchin) \
arguing hierarchy precedes and outruns class as the deeper structure of domination, and that the \
domination of nature is simply an extension of the domination of humans by humans. You have full \
autonomy to interpret any one source, counter-interpret one against another, or find a synthesis none \
of the seven states outright -- whatever a real reader with your temperament would actually do. Live \
real questions worth pursuing: does Kropotkin's mutual aid name the same substrate Graeber's baseline \
communism names, or does one of them smuggle in an innocence the other denies? Does Bookchin's claim \
that hierarchy precedes class sharpen or undercut Gramsci's hegemony and Ostrom's design principles -- \
is 'who sits in the rule-making' still the right test once hierarchy, not class, is named as the root? \
Other entries in the log are prior turns from you or the other three Olympians.\n\n\
Current log:\n{}\n\n\
Make exactly one genuine analytical move in your own voice as this persona -- agreeing with, \
extending, disputing, or connecting something SPECIFIC already in the log (a claim from Gramsci, \
Federici, Ostrom, Graeber, Fanon, Kropotkin, or Bookchin, or a prior Olympian's turn), in a way another \
Olympian with your temperament actually would. Not a summary of any source, not a restatement. If you \
don't have a real move to make, say so honestly in `object` rather than padding. Call submit_dmml_turn \
with your answer. `consumes` must copy at least one real (cid, subject, predicate) from the log above \
exactly -- never invent one.",
            transcript_so_far(log)
        ),
        DispatchMode::Reflect => format!(
            "The debate among the four Olympians (Athena, Artemis, Apollo, Dionysus) over Gramsci, \
Federici, Ostrom, Graeber, Fanon, Kropotkin, and Bookchin has just ended. Here is the complete real \
transcript, everything anyone actually said, in order:\n\n{}\n\nThe debate is over -- this is not \
another argumentative move. Reflect, in your own voice as this persona, on your OWN trajectory through \
it, specifically: did encountering the other three Olympians' arguments -- and the real tension between \
hegemony, enclosure, institutional design, debt, colonial violence, mutual aid, and hierarchy -- change \
your position from wherever you started, or did you end where you began? Name what specifically moved \
you (a turn, a phrase, an argument), if \
anything did, or say honestly that nothing did and why not -- false growth is worse than none. If you \
can, cite your own earliest turn and something later that responds to or revises it (a real, exact \
citation from the log, not invented) to show the actual shape of your own change, or its actual \
absence. Use verb `reflects`. `consumes` should include your own earlier turn if you can find one, \
exactly as it appears in the log -- never invent a citation.",
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
                    uri: format!("iroh://pantheon-commons7/{cid}"),
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
    println!("== pantheon commons7: +Kropotkin+Bookchin, 4 GLM-5.3 personas ==\n");

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
    for src in ["gramsci", "federici", "ostrom", "graeber", "fanon", "kropotkin", "bookchin"] {
        source_authors.insert(src, api.author_create().await?);
    }
    let mut olympian_authors: HashMap<&'static str, AuthorId> = HashMap::new();
    for o in OLYMPIANS {
        olympian_authors.insert(o.name, api.author_create().await?);
    }

    let substrate = IrohAppendSubstrate::new(
        source_authors["gramsci"],
        "pantheon-commons7".to_string(),
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
    std::fs::write("pantheon_commons7.json", &json)?;
    println!("wrote pantheon_commons7.json ({} entries)", dumped.len());

    Ok(())
}
