//! What a sense-machine's operation yields: structured, format-agnostic
//! data -- never prose, never markup. `render.rs` ships the one renderer
//! this crate needs (`Percept -> String`, for `cli/` and as the reference
//! everything else is checked against); a web frontend renders the
//! identical structure into HTML, and any future frontend (a TUI, WebGL,
//! generated images) renders it into whatever it needs, all from the same
//! `Percept` a sense-machine's operation produced. Nothing here has an
//! opinion about presentation -- that's the whole point of hoisting this
//! out of `render_room`'s old hardcoded string-building.
//!
//! `key` on a `PerceptField` is a stable, machine-readable tag ("items",
//! "exits", "unexplored") that a renderer switches on to decide layout --
//! never prose itself. The *content* of a field (an already-composed
//! sentence, a name, a count) is what varies; the key naming the field
//! doesn't, so every renderer can agree on what "exits" means without
//! parsing prose back apart.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Percept {
    /// "room" | "map" | "roomSummary" so far -- what a renderer switches
    /// on to pick a layout/template for this percept's fields.
    pub kind: String,
    pub title: String,
    pub fields: Vec<PerceptField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerceptField {
    pub key: String,
    pub value: PerceptValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PerceptValue {
    Text(String),
    List(Vec<String>),
    /// A percept nested inside another -- a map's rooms, each perceived
    /// (and gated by senses) the same way anything else is, not a
    /// bespoke summary type living outside this vocabulary.
    Nested(Vec<Percept>),
}

impl Percept {
    pub fn new(kind: impl Into<String>, title: impl Into<String>) -> Self {
        Percept {
            kind: kind.into(),
            title: title.into(),
            fields: Vec::new(),
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: PerceptValue) -> Self {
        self.fields.push(PerceptField {
            key: key.into(),
            value,
        });
        self
    }

    pub fn field(&self, key: &str) -> Option<&PerceptValue> {
        self.fields.iter().find(|f| f.key == key).map(|f| &f.value)
    }
}
