//! Direct comparison, not parallel implementation: `dmml::identity::
//! predicate_iri` must produce the byte-identical IRI `dmml_runtime::
//! vocab::dynamic_predicate` does for the same local name -- carried
//! over from written-world's own `dmml`/`engine` cross-check test when
//! this repo was split out (`engine` is `dmml-runtime` here).

#[test]
fn matches_dmml_runtime_vocab_dynamic_predicate_for_ordinary_names() {
    for local in ["isA", "state", "holds", "craftedBy", "lit_by", "worn-smooth-from"] {
        let dmml_iri = dmml::identity::predicate_iri(local);
        let runtime_iri = dmml_runtime::vocab::dynamic_predicate(local)
            .expect("a well-formed local name should always produce a valid IRI")
            .as_str()
            .to_string();
        assert_eq!(dmml_iri, runtime_iri, "predicate_iri('{local}') should match dmml_runtime::vocab::dynamic_predicate exactly");
    }
}

#[test]
fn strips_the_same_characters_dmml_runtime_vocab_strips() {
    // dmml_runtime::vocab::dynamic_predicate strips everything outside
    // [A-Za-z0-9_-]; predicate_iri needs to strip identically, not just
    // produce *a* valid IRI, or the two would silently diverge on any
    // predicate name containing punctuation a model might plausibly emit.
    let messy = "a weird predicate! (with punctuation)";
    let dmml_iri = dmml::identity::predicate_iri(messy);
    let runtime_iri = dmml_runtime::vocab::dynamic_predicate(messy)
        .expect("stripping to non-empty should still succeed")
        .as_str()
        .to_string();
    assert_eq!(dmml_iri, runtime_iri);
}
