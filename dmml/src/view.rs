//! The view protocol -- client language for agent-emitted UX, per
//! `VIEW_SPEC.md` (issue #71 milestone 2). Plain JSON via `serde`, not
//! DMML text: this is the client-facing protocol `SPEC.md` §13 flagged
//! as open, not DMML grammar (see `VIEW_SPEC.md`'s own "why JSON, not a
//! new DMML-adjacent grammar" section for why). Lives here rather than
//! in `client` because both `client` (the browser) and `server` (the
//! Worker, #74) need these types -- `dmml` is the one crate both already
//! depend on. Pure data types -- no `wasm_bindgen` here deliberately, so
//! this module compiles and tests on a native host target without
//! needing `--target wasm32-unknown-unknown`; a thin `#[wasm_bindgen]`
//! wrapper (JSON string in/out), if the browser client ever needs one
//! beyond `client::view`'s existing re-export, belongs in `client/src/
//! lib.rs` alongside `WebGame`, not here.

use serde::{Deserialize, Serialize};

/// One agent-composed panel. `kind` tags the JSON form
/// (`{"kind":"narration","text":"..."}`), matching the discriminated-
/// union convention `dmml::identity`'s `WireConsumeRef` already uses
/// for the same reason -- one flat, self-describing shape per variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Panel {
    Narration { text: String },
    List { title: String, items: Vec<String> },
    Actions { choices: Vec<ActionChoice> },
    Map { text: String },
}

/// One offered choice inside an `Actions` panel. `event` is opaque to
/// the client -- see `VIEW_SPEC.md`'s own note on this field; the
/// client's only obligation is echoing it back unchanged via
/// `ClientEvent::Action` if the player picks it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionChoice {
    pub label: String,
    pub event: String,
}

/// An ordered list of panels -- no positional meaning beyond render
/// order, no panel kind required, no panel kind limited to appearing
/// once. `Default` is the empty view (zero panels), a valid (if
/// unusual) `View` per this spec -- not a sentinel for "no view yet."
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub panels: Vec<Panel>,
}

/// What a client sends back after the player interacts. `FreeText` is
/// always valid, regardless of whether the most recent `View` offered
/// an `Actions` panel at all -- see `VIEW_SPEC.md`'s own note on why an
/// `Actions` panel is a convenience, never the only way to act.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClientEvent {
    Action { event: String },
    FreeText { text: String },
}
