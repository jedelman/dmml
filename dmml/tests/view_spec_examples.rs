//! Independent verification of `dmml::view`'s serde shapes against
//! `VIEW_SPEC.md`'s worked examples -- written from the spec text, not
//! derived from the implementation. Checks both directions: that the
//! documented JSON deserializes into the expected Rust value, and that
//! the Rust value serializes back to equivalent JSON.

use dmml::view::{ActionChoice, ClientEvent, Panel, View};
use serde_json::json;

#[test]
fn example_1_room_view_round_trips() {
    let view = View {
        panels: vec![
            Panel::Narration {
                text: "The heavy door swings open, revealing a narrow stone stairway leading down."
                    .to_string(),
            },
            Panel::List {
                title: "Exits".to_string(),
                items: vec!["down".to_string(), "back to the courtyard".to_string()],
            },
            Panel::Actions {
                choices: vec![
                    ActionChoice {
                        label: "Go down the stairs".to_string(),
                        event: "go down".to_string(),
                    },
                    ActionChoice {
                        label: "Return to the courtyard".to_string(),
                        event: "go courtyard".to_string(),
                    },
                ],
            },
            Panel::Map {
                text: "  [courtyard]\n       |\n   [you are here]\n       |\n     [???]"
                    .to_string(),
            },
        ],
    };

    let expected = json!({
        "panels": [
            { "kind": "narration", "text": "The heavy door swings open, revealing a narrow stone stairway leading down." },
            { "kind": "list", "title": "Exits", "items": ["down", "back to the courtyard"] },
            { "kind": "actions", "choices": [
                { "label": "Go down the stairs", "event": "go down" },
                { "label": "Return to the courtyard", "event": "go courtyard" }
            ] },
            { "kind": "map", "text": "  [courtyard]\n       |\n   [you are here]\n       |\n     [???]" }
        ]
    });

    assert_eq!(serde_json::to_value(&view).unwrap(), expected);
    let parsed: View = serde_json::from_value(expected).unwrap();
    assert_eq!(parsed, view);
}

#[test]
fn example_2_inventory_is_a_single_list_panel() {
    let json_value = json!({ "panels": [
        { "kind": "list", "title": "Inventory", "items": ["a rusted key", "an empty waterskin"] }
    ] });
    let view: View = serde_json::from_value(json_value.clone()).unwrap();
    assert_eq!(
        view,
        View {
            panels: vec![Panel::List {
                title: "Inventory".to_string(),
                items: vec!["a rusted key".to_string(), "an empty waterskin".to_string()],
            }],
        }
    );
    assert_eq!(serde_json::to_value(&view).unwrap(), json_value);
}

#[test]
fn example_3_pure_narration_view_has_no_actions_panel() {
    let json_value = json!({ "panels": [
        { "kind": "narration", "text": "The stranger studies you for a long moment, saying nothing." }
    ] });
    let view: View = serde_json::from_value(json_value).unwrap();
    assert_eq!(view.panels.len(), 1);
    assert!(!view
        .panels
        .iter()
        .any(|p| matches!(p, Panel::Actions { .. })));
}

#[test]
fn example_4_action_event_echoes_the_choice_verbatim() {
    let json_value = json!({ "kind": "action", "event": "go down" });
    let event: ClientEvent = serde_json::from_value(json_value.clone()).unwrap();
    assert_eq!(
        event,
        ClientEvent::Action {
            event: "go down".to_string()
        }
    );
    assert_eq!(serde_json::to_value(&event).unwrap(), json_value);
}

#[test]
fn example_5_free_text_is_a_distinct_event_kind() {
    let json_value = json!({ "kind": "freeText", "text": "look under the stairs" });
    let event: ClientEvent = serde_json::from_value(json_value.clone()).unwrap();
    assert_eq!(
        event,
        ClientEvent::FreeText {
            text: "look under the stairs".to_string()
        }
    );
    assert_eq!(serde_json::to_value(&event).unwrap(), json_value);
}

/// Not a worked example -- generalization: the empty view (zero panels)
/// is a valid, default `View`, not a sentinel/error per the spec's own
/// "Default is the empty view... not a sentinel" doc comment.
#[test]
fn empty_view_is_valid_and_is_the_default() {
    assert_eq!(View::default(), View { panels: vec![] });
    let json_value = json!({ "panels": [] });
    let view: View = serde_json::from_value(json_value.clone()).unwrap();
    assert_eq!(view, View::default());
    assert_eq!(serde_json::to_value(&view).unwrap(), json_value);
}

/// Generalization: free text is representable regardless of what the
/// most recent view contained -- this test only checks the type itself
/// carries no such coupling (a `ClientEvent::FreeText` doesn't
/// reference or require a prior `View` at all).
#[test]
fn free_text_event_is_independent_of_any_view() {
    let event = ClientEvent::FreeText {
        text: "look under the stairs".to_string(),
    };
    let _ = serde_json::to_string(&event).expect("FreeText must always serialize");
}
