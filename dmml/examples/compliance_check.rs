//! Ground-truth oracle for an authoring-compliance checkpoint (see
//! `compliance/` at the repo root): runs a captured model reply through
//! the REAL, current production authoring boundary
//! (`from_json::update_from_json`) -- not `dmml-hs`, not any
//! approximation of it. This is deliberate: the checkpoint exists to
//! measure whether the real thing being considered for retirement
//! (`from_json.rs`) is even something light models can author against
//! today, so the oracle has to be the real thing, not a stand-in for it.
//!
//! Reads newline-delimited JSON records from stdin, one per (model,
//! scenario) dispatch:
//!
//! ```json
//! {"id": "s1", "model": "z-ai/glm-5.3-flash", "scenario": "mint-a-room", "reply": "<raw model text>"}
//! ```
//!
//! Writes one newline-delimited JSON verdict per input record to stdout:
//!
//! ```json
//! {"id": "s1", "model": "...", "scenario": "...", "extracted": true, "outcome": "accepted", "batches": 1, "commits": 2, "machines": 0, "errors": []}
//! ```
//!
//! `outcome` is one of `"accepted"`, `"rejected"`, or `"unparseable"`
//! (no fenced or bare JSON found at all in the reply). Never panics on
//! malformed input -- a reply that fails to extract or parse is a real,
//! expected outcome this checkpoint exists to count, not a bug.

use dmml::from_json;
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.expect("failed to read stdin");
        if line.trim().is_empty() {
            continue;
        }
        let record: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("skipping malformed input line: {e}");
                continue;
            }
        };
        let id = record.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let model = record.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let scenario = record.get("scenario").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let reply = record.get("reply").and_then(|v| v.as_str()).unwrap_or("");

        let verdict = check_reply(reply);

        let line_out = serde_json::json!({
            "id": id,
            "model": model,
            "scenario": scenario,
            "fenced": verdict.fenced,
            "outcome": verdict.outcome,
            "batches": verdict.batches,
            "commits": verdict.commits,
            "machines": verdict.machines,
            "errors": verdict.errors,
        });
        writeln!(out, "{line_out}").expect("failed to write stdout");
    }
}

struct Verdict {
    /// Whether a fenced code block was actually found in the reply --
    /// separate from whether the resulting candidate text turned out to
    /// be valid JSON. A model that skips the fence but still emits valid
    /// JSON scores `fenced: false, outcome: "accepted"`; both facts are
    /// worth keeping distinct in the checkpoint report.
    fenced: bool,
    outcome: &'static str,
    batches: usize,
    commits: usize,
    machines: usize,
    errors: Vec<String>,
}

fn check_reply(reply: &str) -> Verdict {
    // Prefer a fenced code block (how a chat-style reply is expected to
    // carry JSON); fall back to treating the whole reply as JSON for a
    // model that skipped the fence entirely -- still a real, scorable
    // case, not something to discard as invalid input.
    let (fenced, candidate) = match from_json::extract_fenced_block(reply) {
        Some((_, fenced_text)) => (true, fenced_text),
        None => (false, reply.trim().to_string()),
    };

    if candidate.is_empty() {
        return Verdict {
            fenced,
            outcome: "unparseable",
            batches: 0,
            commits: 0,
            machines: 0,
            errors: vec!["no fenced or bare JSON content found in reply".to_string()],
        };
    }

    match from_json::update_from_json(&candidate) {
        Ok(update) => {
            let commits = update.batches.iter().map(|b| b.commits.len()).sum();
            let machines = update.batches.iter().map(|b| b.machines.len()).sum();
            Verdict {
                fenced,
                outcome: "accepted",
                batches: update.batches.len(),
                commits,
                machines,
                errors: vec![],
            }
        }
        Err(from_json::UpdateFromJsonError::Json(e)) => Verdict {
            fenced,
            outcome: "unparseable",
            batches: 0,
            commits: 0,
            machines: 0,
            errors: vec![format!("invalid JSON: {e}")],
        },
        Err(from_json::UpdateFromJsonError::Invalid(errs)) => Verdict {
            fenced,
            outcome: "rejected",
            batches: 0,
            commits: 0,
            machines: 0,
            errors: errs.into_iter().map(|e| e.to_string()).collect(),
        },
    }
}
