# Section IX: a fifth citation posture, and a real doubling (2026-08-25)

`dmml/examples/benjamin_section_ix.rs` models Section IX's two moves.

A fifth citation posture: Pirandello is cited with an explicit scope-
limit ("limited to the negative aspects... and to the silent film only"),
structurally the same shape as Section III's Riegl/Wickhoff commit
(consumes both a claim and a stated scope, count 2) — but a genuinely
different logical relationship. Riegl/Wickhoff's incompleteness LICENSED
Benjamin's next move (their gap is what he goes on to fill). Pirandello's
narrowness is instead DEFENDED as not mattering ("the sound film did not
change anything essential") — a citation whose limit doesn't open a door,
it just needs clearing before the argument can lean on it. Same shape,
different function — worth keeping straight rather than treating "cites a
scope-limited source" as one undifferentiated pattern.

A real doubling, modeled as one commit producing two facts rather than
two separately-argued claims: "the aura that envelops the actor vanishes,
and with it the aura of the figure he portrays." One cause (the camera
substituted for the public), two co-produced effects (the actor's own
aura and the portrayed character's aura, Macbeth named explicitly).
Checked by predicate name rather than raw `produces.len()` — the lowering
step adds an `rdf:type` triple per subject, the same artifact Section V's
mirror check already ran into, applied correctly this time without
re-deriving it.

Then a return to Valery's straightforward, endorsed posture (the unnamed
"experts," Arnheim 1932) before the montage/multiple-takes material
(window/scaffold, the gunshot-startle example), closing on "beautiful
semblance" — flagged with an explicit `verificationStatus: "unverified"`
fact, since this is very possibly a real technical term from German
Idealist aesthetics (Schiller/Hegel's *schoner Schein*) and not yet
checked, same deferred discipline as Section IV's Mallarme flag.
