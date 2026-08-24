//! Produces a build artifact: a deterministic, human-readable transcript
//! of everything the demiurge (and the player exploring its output) has
//! committed. Every line traces back to exactly one `WorldGraph::commit`
//! call -- this is that audit trail made durable, not just queryable
//! in-session via the `transcript` command.
//!
//! Usage: cargo run -p engine --example demiurge_transcript [output-path]

use std::env;
use std::fs;

use dmml_runtime::direction::Direction;
use dmml_runtime::Game;

/// A fixed, brute-forced seed rather than a random one: the artifact
/// should be reproducible across builds, not a different world every time
/// CI runs it.
fn find_seed_with_multiple_locks(min_locks: usize) -> u64 {
    for seed in 0..2000u64 {
        let count = Direction::ALL
            .into_iter()
            .filter(|dir| Game::new(seed).handle(dir.word()).contains("sealed"))
            .count();
        if count >= min_locks {
            return seed;
        }
    }
    panic!("no seed found with {min_locks}+ locked exits from start in range 0..2000");
}

fn main() {
    let out_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "demiurge-transcript.txt".to_string());

    // At least two independent locked-door patterns, so the transcript
    // actually shows the second one crystallizing rather than just the
    // bootstrap and one uncrystallized pattern.
    let seed = find_seed_with_multiple_locks(2);
    let mut game = Game::new(seed);

    for dir in Direction::ALL {
        game.handle(dir.word());
    }

    let transcript = game.handle("transcript");
    let kinds = game.handle("kinds");

    let out = format!(
        "written-world demiurge transcript\nseed = {seed}\n\n{transcript}\n--- kinds at end of run ---\n{kinds}\n"
    );

    fs::write(&out_path, &out).unwrap_or_else(|e| panic!("failed to write {out_path}: {e}"));
    println!("Wrote {out_path} ({} bytes)", out.len());
}
