//! A way to exercise the full commune pipeline (prompt -> model -> parse ->
//! validate -> commit) without a live Cloudflare deploy or credentials:
//! substitute any model for Workers AI by hand, feeding its JSON output
//! through the exact same `dmml_runtime::commune::parse_commune_delta` +
//! `WorldGraph::commit` path the real client runs.
//!
//! Usage:
//!   cargo run -p engine --example commune_dry_run -- context [seed]
//!     Prints the JSON context (room facts + declared vocabulary) that
//!     `/api/commune` would receive for the player's starting room --
//!     feed this to whatever's standing in for the model.
//!
//!   cargo run -p engine --example commune_dry_run -- apply <delta-file> [seed]
//!     Reads a JSON delta from `delta-file` (the model's response) and
//!     applies it via `Game::apply_commune_delta`, exactly as the web
//!     client would. Prints whether it committed, and if so, the
//!     resulting room render and declared-relations view.

use std::env;
use std::fs;
use std::process::ExitCode;

use dmml_runtime::Game;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str);

    match mode {
        Some("context") => {
            let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let game = Game::new(seed);
            println!("{}", game.commune_context());
            ExitCode::SUCCESS
        }
        Some("apply") => {
            let Some(delta_path) = args.get(2) else {
                eprintln!("usage: commune_dry_run apply <delta-file> [seed]");
                return ExitCode::FAILURE;
            };
            let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
            let json = match fs::read_to_string(delta_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("failed to read {delta_path}: {e}");
                    return ExitCode::FAILURE;
                }
            };

            let mut game = Game::new(seed);
            match game.apply_commune_delta(&json) {
                Ok(room_text) => {
                    println!("COMMITTED\n");
                    println!("-- room after commit --\n{room_text}\n");
                    println!(
                        "-- declared relation vocabulary --\n{}",
                        game.handle("relations")
                    );
                    println!("\n-- full transcript --\n{}", game.handle("transcript"));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    println!("REJECTED: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!(
                "usage:\n  commune_dry_run context [seed]\n  commune_dry_run apply <delta-file> [seed]"
            );
            ExitCode::FAILURE
        }
    }
}
