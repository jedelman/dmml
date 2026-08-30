//! Reusable validator for machine-minting output -- built for the Valar
//! authoring tier (Jason, 2026-08-30: "could we have a Valar agent...
//! mint these machines according to their taste?"), but generic to
//! anything shaped like `{"update": [{"machines": [...], "commits": []}]}`.
//!
//! Unlike `valinor.rs`/`door.rs`/`quarry.rs`/`wall.rs`/`house.rs` (which
//! build a specific hand-designed world and prove specific transitions
//! fire), this takes a JSON file path as its one argument and reports,
//! generically, whether the machines inside it are REAL, valid DMML --
//! parses at all, every state/transition/guard/effect ident checks out,
//! no structural nonsense (a `has_content` violation, a malformed
//! pattern). It does not attempt to fire any transition, since it has no
//! opinion about what world state should exist when a Valar-minted
//! machine is later operated -- that's a separate step, deliberately not
//! this one's job.
//!
//! Usage: `cargo run -p dmml --example validate_machines -- path/to/machines.json`

use dmml::from_json::update_from_json;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: validate_machines <path-to-json>");
        std::process::exit(2);
    });
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("could not read {path}: {e}");
        std::process::exit(2);
    });

    match update_from_json(&json) {
        Ok(update) => {
            let total_machines: usize = update.batches.iter().map(|b| b.machines.len()).sum();
            let total_commits: usize = update.batches.iter().map(|b| b.commits.len()).sum();
            println!("VALID: {total_machines} machine(s), {total_commits} commit(s) across {} batch(es).\n", update.batches.len());
            for batch in &update.batches {
                for m in &batch.machines {
                    let node = m.node.segments.join("/");
                    println!("machine {node}");
                    println!("  states: {}", m.states.iter().map(|s| s.ident.as_str()).collect::<Vec<_>>().join(", "));
                    for t in &m.transitions {
                        let from = t.from.as_deref().unwrap_or("-");
                        let to = t.to.as_deref().unwrap_or("-");
                        let params = if t.params.is_empty() { String::new() } else { format!("({})", t.params.join(", ")) };
                        println!(
                            "  transition {}{params}: {from} -> {to}  [{} guard(s), {} explicit effect(s)]",
                            t.ident,
                            t.guards.len(),
                            t.effects.len(),
                        );
                        for g in &t.guards {
                            let neg = if g.negated { "NOT " } else { "" };
                            let hops: Vec<String> = g
                                .exists
                                .pattern
                                .hops
                                .iter()
                                .map(|h| format!("--{}--> {:?}", h.predicate, h.term))
                                .collect();
                            println!("    guard: {neg}EXISTS({:?} {})", g.exists.pattern.anchor, hops.join(" "));
                        }
                    }
                    println!();
                }
            }
        }
        Err(e) => {
            println!("INVALID:\n{e}");
            std::process::exit(1);
        }
    }
}
