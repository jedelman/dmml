//! Parsing stays dumb on purpose: split into a verb and an object phrase,
//! nothing more. Whether a verb *means* anything is no longer this
//! module's business -- that's resolved against the graph in `game.rs`,
//! by querying which machines are actually equipped and afford it.

use crate::direction::Direction;

pub struct Command {
    pub verb: String,
    pub object: String,
}

pub fn parse(input: &str) -> Command {
    let input = input.trim().to_lowercase();
    let mut words = input.split_whitespace();
    let mut verb = words.next().unwrap_or("").to_string();
    let object: String = words.collect::<Vec<_>>().join(" ");

    if let Some(dir) = Direction::parse(&verb) {
        return Command {
            verb: "go".to_string(),
            object: dir.word().to_string(),
        };
    }

    verb = match verb.as_str() {
        "l" => "look".to_string(),
        "i" | "inv" => "inventory".to_string(),
        "x" => "examine".to_string(),
        "get" => "take".to_string(),
        other => other.to_string(),
    };

    Command { verb, object }
}

pub const HELP_TEXT: &str = "\
Commands: look, go <direction> (or just north/south/east/west/up/down),
take <item>, drop <item>, inventory, examine <item>, pull <item>, quit.
conjure <item> (mint a new item into this room -- via the Commit/
apply_commit path, not the ordinary Delta one; see graph.rs).
Introspection: assemblage [item] (dump its raw triples),
kinds (list crystallized and crystallizing patterns),
relations (list self-declared relation/attribute predicates and their use counts),
map (every room you've perceived and visited, if you have a way to perceive it),
transcript (every commit so far, in order, tagged by who proposed it),
save [path] (write the transcript to a file -- terminal frontend only).
reach <at-uri> (link this room to a foreign atproto record -- revisiting
it later may surface narrated drift if that record has changed).";
