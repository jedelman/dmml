//! Shared infrastructure for this crate's three Datalog modules
//! (`datalog_guard`, `datalog_effects`, `datalog_referential_integrity`),
//! extracted after it had drifted into three near-identical copies (two
//! byte-identical `SymbolTable`s and two byte-identical `FIXED_POINT_
//! SCALE`/`quantize`/`dequantize` blocks) -- exactly the kind of thing
//! that silently drifts if one copy ever gets a fix the others don't.
//! Not a claim that the three modules should merge into one: `datalog_
//! guard`'s unbounded, negation-based gating and `datalog_effects`'s
//! bounded-chain, effects-aware gating answer a similarly-named question
//! by deliberately different means (see `datalog_guard`'s own module doc
//! for why) -- only the plumbing underneath was actually duplicate.

use std::collections::HashMap;

use oxigraph::model::NamedNode;

/// Interns strings to small `u32` symbols, since `crepe` fact fields
/// must be `Copy` and a `NamedNode`/`String` isn't. Interns and resolves
/// by string; a caller storing `NamedNode`s reconstructs one from the
/// resolved string via `NamedNode::new(&s).expect(...)` -- always safe
/// since the string only ever originated from an already-valid
/// `NamedNode`'s own `.as_str()`.
#[derive(Default)]
pub(crate) struct SymbolTable {
    by_str: HashMap<String, u32>,
    by_sym: Vec<String>,
}

impl SymbolTable {
    pub(crate) fn intern(&mut self, s: &str) -> u32 {
        if let Some(&sym) = self.by_str.get(s) {
            return sym;
        }
        let sym = self.by_sym.len() as u32;
        self.by_str.insert(s.to_string(), sym);
        self.by_sym.push(s.to_string());
        sym
    }

    pub(crate) fn resolve(&self, s: u32) -> &str {
        &self.by_sym[s as usize]
    }

    /// Convenience for the common case of a symbol that was always
    /// interned from a `NamedNode`'s own `.as_str()` -- reconstructing it
    /// always succeeds, since the string only ever came from an
    /// already-valid `NamedNode` in the first place.
    pub(crate) fn resolve_node(&self, s: u32) -> NamedNode {
        NamedNode::new(self.resolve(s)).expect("interned symbol always round-trips a valid IRI")
    }
}

/// Fixed-point scale shared by every module that needs `f32` arithmetic
/// inside a `crepe!` block: fact fields must be `Copy`+`Ord`-friendly,
/// which rules out `f32` directly (no total order), so every float this
/// crate's Datalog rules compare or clamp is quantized to `i64` at this
/// scale first.
pub(crate) const FIXED_POINT_SCALE: f64 = 1_000_000.0;

/// `quantize(f32::MIN)`/`quantize(f32::MAX)` saturate to `i64::MIN`/
/// `i64::MAX` (Rust float->int casts saturate), which is exactly the
/// "unbounded" default wanted wherever a value has no declared range.
pub(crate) fn quantize(v: f32) -> i64 {
    (v as f64 * FIXED_POINT_SCALE).round() as i64
}

pub(crate) fn dequantize(v: i64) -> f32 {
    (v as f64 / FIXED_POINT_SCALE) as f32
}
