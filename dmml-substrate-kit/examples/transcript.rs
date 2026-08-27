//! Renders a `pantheon_*` conversation JSON dump (the shape produced by
//! `pantheon_conversation.rs`/`pantheon_olympians.rs`) as a readable
//! script: speaker name, round header, the claim, and which prior
//! speakers it's actually citing -- resolved from real `consumes`
//! entries, not inferred from prose.
//!
//! Usage: `cargo run --example transcript -- <path-to-dump.json>`
//!
//! Deliberately a pure formatter: it doesn't re-verify citations (the
//! generating program already did that -- see `pantheon_olympians.rs`'s
//! own `[WARNING] ... dropped` discipline) or judge content quality.
//! It exists so a real multi-round run can be *read*, in order, instead
//! of grepped out of raw JSON or a scrollback log.

use std::collections::HashMap;
use std::fmt::Write as _;

use serde::Deserialize;

#[derive(Deserialize)]
struct DumpedTurn {
    cid: String,
    respondent: String,
    #[serde(default)]
    round: u32,
    verb: String,
    subject: String,
    #[allow(dead_code)]
    predicate: String,
    object: String,
    #[serde(default)]
    consumes: Vec<(String, String, String)>,
}

fn display_name(respondent: &str) -> String {
    let mut chars = respondent.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => respondent.to_string(),
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: transcript <path-to-dump.json>"))?;
    let raw = std::fs::read_to_string(&path)?;
    let turns: Vec<DumpedTurn> = serde_json::from_str(&raw)?;

    // Older dumps (pantheon_conversation.rs, and pantheon_olympians.rs
    // runs before `round` was tracked) don't carry a real round number --
    // fall back to treating every non-anchor turn as round 1 rather than
    // guessing a grouping that isn't actually recorded.
    let has_real_rounds = turns.iter().any(|t| t.round > 0);

    let cid_to_respondent: HashMap<&str, &str> = turns
        .iter()
        .map(|t| (t.cid.as_str(), t.respondent.as_str()))
        .collect();

    let mut out = String::new();
    let mut current_round: Option<u32> = None;

    for t in &turns {
        let round = if has_real_rounds { t.round } else if t.round == 0 { 0 } else { 1 };
        if current_round != Some(round) {
            if round == 0 {
                writeln!(out, "=== SOURCE MATERIAL ===\n")?;
            } else {
                writeln!(out, "\n=== ROUND {round} ===\n")?;
            }
            current_round = Some(round);
        }

        let speaker = display_name(&t.respondent);
        writeln!(out, "{} ({}, {}):", speaker.to_uppercase(), t.verb, t.subject)?;

        if !t.consumes.is_empty() {
            let mut cited: Vec<String> = t
                .consumes
                .iter()
                .map(|(cid, subj, _pred)| {
                    let who = cid_to_respondent
                        .get(cid.as_str())
                        .map(|r| display_name(r))
                        .unwrap_or_else(|| "an unresolved citation".to_string());
                    format!("{who} on \"{subj}\"")
                })
                .collect();
            cited.dedup();
            writeln!(out, "  [citing: {}]", cited.join("; "))?;
        } else if round > 0 {
            writeln!(out, "  [citing: nothing verified -- a free-standing turn]")?;
        }

        writeln!(out, "  \"{}\"\n", t.object)?;
    }

    print!("{out}");
    Ok(())
}
