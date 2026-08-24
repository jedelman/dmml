//! A spike, not a feature: generates a small labyrinth deterministically,
//! walks it, and dumps exactly what a sighted mover would perceive at each
//! step (room text + available-action labels, nothing extra) to JSON on
//! stdout. Feeding this transcript to a model playing Mercury, Asker of
//! Questions, is a separate, non-Rust step -- deliberately kept out of
//! this engine-side crate, which stays I/O-free.
//!
//! Usage: cargo run -p engine --example mercury_labyrinth > /tmp/labyrinth.json

use dmml_runtime::direction::Direction;
use dmml_runtime::Game;
use serde::Serialize;

#[derive(Serialize)]
struct Step {
    arrived_via: Option<String>,
    room_text: String,
    action_labels: Vec<String>,
}

fn snapshot(game: &Game, arrived_via: Option<&str>) -> Step {
    Step {
        arrived_via: arrived_via.map(|s| s.to_string()),
        room_text: game.look(),
        action_labels: game
            .available_actions()
            .into_iter()
            .map(|a| a.label)
            .collect(),
    }
}

/// Attempts `dir` from wherever the player currently stands; returns
/// whether it actually moved them. `look()` text alone can't tell you
/// this: attempting a *locked* door still deposits a fresh mechanism item
/// into the room you're standing in (the origin of that locked edge), so
/// a blocked attempt changes `look()`'s output too. `player_room()`'s own
/// identity is the only thing that only changes on an actual move.
fn try_move(game: &mut Game, dir: Direction) -> bool {
    let before = game.player_room();
    game.handle(dir.word());
    game.player_room() != before
}

/// A fixed, brute-forced seed so this transcript is reproducible: at least
/// 3 open exits from the Threshold (room enough to actually wander) and at
/// least 1 locked one (a genuine "why is this sealed" for Mercury to ask
/// about), same search style as `demiurge_transcript.rs`'s own seed hunt.
fn find_labyrinth_seed() -> u64 {
    for seed in 0..3000u64 {
        let mut probe = Game::new(seed);
        let mut open = 0;
        let mut locked = 0;
        for dir in Direction::ALL {
            if try_move(&mut probe, dir) {
                open += 1;
                probe.handle(dir.opposite().word());
            } else {
                locked += 1;
            }
        }
        if open >= 3 && locked >= 1 {
            return seed;
        }
    }
    panic!("no seed found with >=3 open and >=1 locked exit from start in range 0..3000");
}

fn main() {
    let seed = find_labyrinth_seed();
    let mut game = Game::new(seed);
    let mut steps = vec![snapshot(&game, None)];

    // Ring 1: try every direction from the Threshold; enter and record
    // whatever's open, then return before trying the next one, so the
    // walk is always resumed from a known place.
    let mut ring1_open: Vec<Direction> = Vec::new();
    for dir in Direction::ALL {
        if try_move(&mut game, dir) {
            ring1_open.push(dir);
            steps.push(snapshot(&game, Some(dir.word())));
            game.handle(dir.opposite().word());
        }
    }
    // The Threshold's own action labels only reflect every exit (open or
    // sealed) once all of them have been attempted above -- refresh it.
    steps[0] = snapshot(&game, None);

    // Ring 2: from the first two ring-1 rooms, go one step further if any
    // direction besides the way back is open -- just enough depth for a
    // real "how does this room relate to that one" question, not a full
    // maze.
    let mut extended = 0;
    for &dir in &ring1_open {
        if extended >= 2 {
            break;
        }
        game.handle(dir.word());
        for dir2 in Direction::ALL {
            if dir2 == dir.opposite() {
                continue;
            }
            if try_move(&mut game, dir2) {
                steps.push(snapshot(&game, Some(dir2.word())));
                game.handle(dir2.opposite().word());
                extended += 1;
                break;
            }
        }
        game.handle(dir.opposite().word());
    }

    let out = serde_json::json!({
        "seed": seed,
        "steps": steps,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
