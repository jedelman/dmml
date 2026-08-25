# Section VI, and a real semantics found by actually running it (2026-08-25)

`dmml/examples/benjamin_section_vi.rs` models Section VI's actual shape:
one continuous five-stage chain (not a genealogy-then-pivot like Section
IV) — cult value's last refuge in the presence of the human face
(portrait photography) INVERTS into exhibition value's first decisive
win via the face's absence (Atget's deserted Paris streets — "photographed
them like scenes of crime"), which produces a genuinely new interpretive
mechanism (captions, checked against a painting-title baseline to confirm
the text's explicit claim that they differ "altogether"), intensifying
further into film's sequence-dependent meaning.

Building the first check for this file surfaced something none of the
five prior Benjamin files had actually tested: querying `current_value`
for an EARLIER fact's own key, inside the FULL combined log, after that
fact had been cited downstream by a commit that produces a DIFFERENT
subject. `Materialized::from_identified_commits` (confirmed in
`dmml/src/interpret.rs`'s own doc comment) retracts the exact `(subject,
predicate)` key a commit `consumes`, unconditionally, before applying
that commit's own `produces`. Atget's commit consumes
`(artwork/early_photograph, humanPresence)` but produces a different key
entirely — `(artwork/atget_photograph, humanPresence)`. Result: in the
combined log, portrait's own fact reads back as `None`, even though
`pantheon.rs`'s and every later file's "materialized alone, still real"
checks are equally true here and prove the underlying commit is
untouched.

This is a genuine "cite-and-spend" semantics for `consumes`, not a plain
same-key overwrite. `pantheon.rs`'s Nyx never exposed it because Nyx both
consumed AND produced the identical key (`sky/1, origin`) — the
retraction was invisible, masked by the immediate re-assertion at the
same key. Every file since (`editorial_loop.rs`, the milieu file, the
Section II-V files) checked persistence only by re-materializing a single
commit alone; none had queried an earlier, differently-keyed fact's own
key inside a full combined log after downstream citation. This file is
the first one built that way, and it found real behavior rather than
assuming the pattern from earlier files generalized.

Practical upshot for future files in this series (and worth flagging for
the papers, since Section 4 and its worked examples make claims in this
territory): "the original fact remains real and citable" is true and
checked correctly everywhere in this series — but only ever demonstrated
via isolated re-materialization, never via a combined-log query on the
cited fact's own key. Those are different properties. The first (durable,
independently re-derivable) holds throughout. The second (visible at its
own key inside a combined materialization after being cited) does NOT
hold whenever the citing commit produces a different key — which is the
normal case for an argumentative chain like this essay's, as opposed to
`pantheon.rs`'s deliberately-contrived same-key rivalry.
