//! A real, small piece of DMML content walked through the whole local
//! pipeline: parse -> lower -> validate -> identity -> materialize ->
//! machine wiring. Not a test (nothing here is a spec worked example);
//! this is a demo of "author DMML, get a real CID and a real answer to
//! a guard question back," run with `cargo run -p dmml --example
//! build_something`.

use dmml::ast::TopLevelItem;
use dmml::interpret::Materialized;
use dmml::machine::{may_fire, parse_all_machines, EvalContext};
use dmml::validate::validate_declarations;
use dmml::lower;
use dmml_substrate_kit::atproto_cid as identity;

const SRC: &str = r#"
machine edge/12 {
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }
}

commit mints {
  declare relation opensTo
  declare attribute state
  declare relation holds

  room/1 opensTo room/2
  edge/12 state locked
  player holds key/7
}
"#;

fn main() {
    println!("=== source ===\n{SRC}");

    let doc = dmml::parse(SRC).expect("should parse");
    println!(
        "parsed {} top-level item(s): {:?}\n",
        doc.items.len(),
        doc.items
            .iter()
            .map(|item| match item {
                TopLevelItem::Commit(_) => "commit",
                TopLevelItem::Reference(_) => "reference",
                TopLevelItem::Machine(_) => "machine",
            })
            .collect::<Vec<_>>()
    );

    let commit_stmt = doc
        .items
        .iter()
        .find_map(|item| match item {
            TopLevelItem::Commit(c) => Some(c),
            _ => None,
        })
        .expect("the document has a commit");

    validate_declarations(commit_stmt).expect("every predicate should be self-declared");
    println!("validate_declarations: Ok (every predicate self-declared)\n");

    let lowered = lower::lower_commit(commit_stmt);
    println!("lowered produces ({} triples):", lowered.produces.len());
    for t in &lowered.produces {
        println!("  {} {} {:?}", t.subject, t.predicate, t.object);
    }
    println!();

    let cid = identity::compute_cid(&lowered, "2026-08-18T00:00:00Z");
    println!("computed CID: {cid}\n");

    let world = Materialized::from_commits(&[lowered]);
    println!(
        "materialized world: {} (subject, predicate) pairs currently holding a value\n",
        world.len()
    );

    let machines = parse_all_machines(&doc).expect("machine bodies should parse");
    println!("declared machines: {:?}\n", machines.keys().collect::<Vec<_>>());

    let ctx = EvalContext {
        self_node: "edge/12".to_string(),
        params: Default::default(),
    };
    let result = may_fire(&machines["edge/12"], "unlock", &ctx, &world);
    println!("may_fire(edge/12, \"unlock\", ...) = {result:?}");
    assert_eq!(result, Some(true), "guards should hold: state is locked, player holds key/7");

    println!("\n=== done: a real DMML commit, validated, CID'd, and its machine's transition confirmed fireable ===");
}
