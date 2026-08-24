//! Independent verification of `dmml::machine::parse_machine_body`/
//! `resolve_transition` against `MACHINE_SPEC.md`'s worked examples --
//! written from the spec text, not derived from the implementation.

use dmml::machine::{
    parse_machine_body, resolve_transition, Effect, GuardClause, MachineBody, Pattern,
    PatternHop, PatternTerm, TransitionDecl,
};

fn transitions(body: &MachineBody) -> Vec<&TransitionDecl> {
    body.transitions.iter().collect()
}

#[test]
fn game_go_movement_gate_example() {
    let src = "
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }

  transition traverse(dest) {
    guard: EXISTS(self state unlocked)
    guard: EXISTS(?room hasEdge self connectsTo $dest)
  }
";
    let body = parse_machine_body(src).expect("should parse");

    assert_eq!(body.states.len(), 2);
    assert_eq!(body.states[0].ident, "locked");
    assert_eq!(body.states[1].ident, "unlocked");

    let ts = transitions(&body);
    assert_eq!(ts.len(), 2);

    let unlock = ts[0];
    assert_eq!(unlock.ident, "unlock");
    assert!(unlock.params.is_empty());
    assert_eq!(unlock.from, Some("locked".to_string()));
    assert_eq!(unlock.to, Some("unlocked".to_string()));
    assert!(unlock.effects.is_empty());
    assert_eq!(unlock.guards.len(), 1);
    assert!(!unlock.guards[0].negated);
    assert_eq!(
        unlock.guards[0].exists.pattern,
        Pattern {
            anchor: PatternTerm::Node("player".to_string()),
            hops: vec![PatternHop {
                predicate: "holds".to_string(),
                term: PatternTerm::Node("key/7".to_string()),
            }],
        }
    );

    let traverse = ts[1];
    assert_eq!(traverse.ident, "traverse");
    assert_eq!(traverse.params, vec!["dest".to_string()]);
    assert_eq!(traverse.from, None);
    assert_eq!(traverse.to, None);
    assert!(traverse.effects.is_empty());
    assert_eq!(traverse.guards.len(), 2);
    assert!(!traverse.guards[0].negated);
    assert_eq!(
        traverse.guards[0].exists.pattern,
        Pattern {
            anchor: PatternTerm::SelfRef,
            hops: vec![PatternHop {
                predicate: "state".to_string(),
                term: PatternTerm::Node("unlocked".to_string()),
            }],
        }
    );
    assert!(!traverse.guards[1].negated);
    assert_eq!(
        traverse.guards[1].exists.pattern,
        Pattern {
            anchor: PatternTerm::Var("room".to_string()),
            hops: vec![
                PatternHop {
                    predicate: "hasEdge".to_string(),
                    term: PatternTerm::SelfRef,
                },
                PatternHop {
                    predicate: "connectsTo".to_string(),
                    term: PatternTerm::Param("dest".to_string()),
                },
            ],
        }
    );
}

#[test]
fn negation_example_door_guarded_unless_occupied() {
    let src = "
  state guarded

  transition enter {
    guard: not EXISTS(guardPost/3 occupiedBy ?guard)
  }
";
    let body = parse_machine_body(src).expect("should parse");
    assert_eq!(body.states.len(), 1);
    assert_eq!(body.states[0].ident, "guarded");

    let ts = transitions(&body);
    assert_eq!(ts.len(), 1);
    let enter = ts[0];
    assert_eq!(enter.ident, "enter");
    assert!(enter.params.is_empty());
    assert_eq!(enter.from, None);
    assert_eq!(enter.to, None);
    assert!(enter.effects.is_empty());
    assert_eq!(enter.guards.len(), 1);
    assert!(enter.guards[0].negated);
    assert_eq!(
        enter.guards[0].exists.pattern,
        Pattern {
            anchor: PatternTerm::Node("guardPost/3".to_string()),
            hops: vec![PatternHop {
                predicate: "occupiedBy".to_string(),
                term: PatternTerm::Var("guard".to_string()),
            }],
        }
    );
}

#[test]
fn explicit_effect_list_example() {
    let src = "
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    effect: retract locked, assert unlocked
  }
";
    let body = parse_machine_body(src).expect("should parse");
    let ts = transitions(&body);
    assert_eq!(
        ts[0].effects,
        vec![
            Effect::Retract("locked".to_string()),
            Effect::Assert("unlocked".to_string()),
        ]
    );
}

#[test]
fn empty_body_is_ok_not_error() {
    assert_eq!(parse_machine_body(""), Ok(MachineBody::default()));
    assert_eq!(parse_machine_body("   \n  "), Ok(MachineBody::default()));
}

#[test]
fn unconditional_no_op_transition_is_rejected() {
    let src = "
  transition noop { }
";
    assert!(parse_machine_body(src).is_err());
}

/// Not a worked example in the spec -- generalization test for the
/// `from`/`to`-only sugar case (no author-written `guard:`/`effect:`
/// lines at all).
#[test]
fn resolve_transition_desugars_from_to_with_no_explicit_guards_or_effects() {
    let src = "
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
  }
";
    let body = parse_machine_body(src).expect("should parse");
    let decl = &body.transitions[0];
    assert!(decl.guards.is_empty());
    assert!(decl.effects.is_empty());

    let (guards, effects): (Vec<GuardClause>, Vec<Effect>) = resolve_transition(decl);
    assert_eq!(guards.len(), 1);
    assert!(!guards[0].negated);
    assert_eq!(
        guards[0].exists.pattern,
        Pattern {
            anchor: PatternTerm::SelfRef,
            hops: vec![PatternHop {
                predicate: "state".to_string(),
                term: PatternTerm::Node("locked".to_string()),
            }],
        }
    );
    assert_eq!(
        effects,
        vec![
            Effect::Retract("locked".to_string()),
            Effect::Assert("unlocked".to_string()),
        ]
    );
}

/// `to` present without `from` must NOT trigger the sugar-effect append
/// (MACHINE_SPEC.md's exact-AND condition on step 4).
#[test]
fn resolve_transition_to_without_from_appends_nothing() {
    let decl = TransitionDecl {
        ident: "weird".to_string(),
        params: vec![],
        from: None,
        to: Some("unlocked".to_string()),
        guards: vec![],
        effects: vec![],
        span: dmml::machine::Span { start: 0, end: 0 },
    };
    let (guards, effects) = resolve_transition(&decl);
    assert!(guards.is_empty());
    assert!(effects.is_empty());
}

/// `resolve_transition` must not desugar `parse_machine_body`'s own
/// output -- `guards`/`effects` on the parsed `TransitionDecl` hold only
/// what was literally written (checked already above), and calling
/// `resolve_transition` is a separate, explicit step.
#[test]
fn parse_machine_body_never_auto_desugars() {
    let src = "
  state locked
  state unlocked

  transition unlock {
    from: locked
    to: unlocked
    guard: EXISTS(player holds key/7)
  }
";
    let body = parse_machine_body(src).expect("should parse");
    let decl = &body.transitions[0];
    assert_eq!(decl.guards.len(), 1, "only the literal guard, no from-sugar");
    assert!(decl.effects.is_empty(), "no to-sugar until resolve_transition runs");
}
