//! Live demonstration for a design conversation, not part of the crate's
//! real test suite: runs real JSON through the real `update_from_json`
//! entry point to show (a) what GRAMMAR.md's documented commit shape
//! actually does when fed to the real parser, and (b) the real
//! sibling-commit-drops-a-fact mistake `from_json.rs`'s own `UpdateInput`
//! doc comment already names as an observed failure mode.

fn main() {
    println!("=== 1. GRAMMAR.md's own documented commit shape, fed to the real parser ===\n");
    let grammar_doc_shaped = r#"
    {
      "update": [{
        "commits": [{
          "consumes": [],
          "produces": "_:claim1 <https://written-world.example/predicate/claim> \"the crisis consists in the old dying and the new not yet born\" .\n",
          "predicate": "asserts",
          "created_at": "2026-08-31T00:00:00Z"
        }]
      }]
    }
    "#;
    match serde_json::from_str::<dmml::from_json::UpdateInput>(grammar_doc_shaped) {
        Ok(_) => println!("PARSED (unexpected)"),
        Err(e) => println!("REJECTED at the JSON-deserialize step, before any DMML-level validation:\n  {e}\n"),
    }

    println!("=== 2. A minimal, valid commit against the REAL CommitInput shape ===\n");
    let real_shape_valid = r#"
    {
      "update": [{
        "commits": [{
          "verb": "asserts",
          "declares": [{"kind": "relation", "name": "claim"}],
          "facts": [{
            "subject": "notebooks/interregnum",
            "predicate": "claim",
            "object": {"kind": "str", "value": "the crisis consists in the old dying and the new not yet born"}
          }],
          "consumes": [],
          "refs": {}
        }]
      }]
    }
    "#;
    match dmml::from_json::update_from_json(real_shape_valid) {
        Ok(doc) => println!("ACCEPTED: {} batch(es) built.\n", doc.batches.len()),
        Err(e) => println!("REJECTED (unexpected): {e:?}\n"),
    }

    println!("=== 3. The real documented agent mistake: a fact split across sibling commits in one batch ===\n");
    println!("(from_json.rs's own UpdateInput doc comment: 'a model split `player/1 holds\nkey/1` into one commit and `player/1 holds key/2` into a sibling commit in\nthe same batch, silently dropping key/1 at materialization' -- reconstructed\nhere against the real, current validator, not just quoted from the comment.)\n");
    let split_fact_batch = r#"
    {
      "update": [{
        "commits": [
          {
            "verb": "asserts",
            "declares": [{"kind": "relation", "name": "holds"}],
            "facts": [{"subject": "player/1", "predicate": "holds", "object": {"kind": "node", "value": "key/1"}}],
            "consumes": [],
            "refs": {}
          },
          {
            "verb": "asserts",
            "declares": [{"kind": "relation", "name": "holds"}],
            "facts": [{"subject": "player/1", "predicate": "holds", "object": {"kind": "node", "value": "key/2"}}],
            "consumes": [],
            "refs": {}
          }
        ]
      }]
    }
    "#;
    match dmml::from_json::update_from_json(split_fact_batch) {
        Ok(_) => println!("ACCEPTED (would silently drop key/1 at materialization if this validator didn't exist)"),
        Err(e) => println!("REJECTED by the real validator:\n  {e:?}"),
    }
}
