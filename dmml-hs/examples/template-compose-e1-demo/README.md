# template-compose-e1-demo

Tests `DMML.TemplateBank` (`written-world#139`) against real, baked
world content for the first time — the E1 endurance run
(`jedelman/dmml#1`, `compliance-endurance/results/commits`, 208 files,
4 real free models' output over 20 real rounds) — instead of a
hand-crafted toy fixture. Confirms one thing, extends the design one
real way, and honestly surfaces a genuine limitation rather than
hiding it.

```sh
template-compose-e1-demo ../compliance-endurance/results/commits
```

Real output:

```
parsed 177 real commit file(s) as ground truth

=== npc/delver ===
eligible templates: ["miner-at-work"]
  -> npc/delver works the ninefathom seam: first down the ninefathom shaft.

=== herbalist/onn ===
eligible templates: ["herbalist-of-oldroot"]
  -> herbalist/onn tends oldroot: herbalist who reads the ash's warning and seeks the keeper's counsel.

=== npc/keeper ===
eligible templates: []

=== attempting a guard against a literal-valued fact (role = "...") ===
REJECTED AT PARSE TIME (expected):
<surface>:7:23:
  |
7 |     guard self `role` "first down the ninefathom shaft"
  |                       ^^^^
unexpected ""fir"
expecting "self" or '$'
```

## What this confirms

The guard-based eligibility mechanism (`written-world#139`'s second
correction — templates ARE guards, not a bespoke check) holds against
real, messy, four-different-models'-worth of authored content, not just
a two-line hand-written fixture: `npc/delver`'s real `:: a Miner` +
`worksAt mine/ninefathom` facts correctly select `miner-at-work`;
`herbalist/onn`'s real `:: a Herbalist` + `worksAt forest/oldroot`
correctly select `herbalist-of-oldroot`. `npc/keeper` correctly gets
zero eligible templates — not a bug, just an honestly incomplete
catalog (nothing here covers `Keeper`) — exactly the kind of gap the
still-unbuilt governed catalog-expansion pipeline exists to eventually
fill.

## What this extends: `renderTemplateWith`

Grepping the real corpus first (not guessing) found that the bulk of
its descriptive content — `state`, `role`, `purpose`, `description` —
is stored as STRING LITERALS (`. role = "first down the ninefathom
shaft"`), not node references, while relational facts (`worksAt`,
`belongsTo`, `:: a Type`) are node-valued. `DMML.TemplateBank` added
`renderTemplateWith`: `{attr:<predicate>}` markers in template text get
filled from the subject's own live literal fact for that predicate
(`DMML.Materialize.currentValue`) — real content flowing in as
substitution, decided by structure (the guards that made the template
eligible in the first place), never as a second eligibility mechanism.

## The real, honest limitation this run found and proved, not just cited

`DMML.Guard`'s own doc comment already disclosed that literal-valued
facts never participate in a guard walk (faithful to the real crate's
crepe-loader behavior). Tested here for the first time against a real
attempt, not just read off the comment: `guard self \`role\`
"first down the ninefathom shaft"` doesn't just fail to match — it
fails to PARSE. `DMML.Ast.PatternTerm` has no literal case at all
(`TermSelf | TermParam | TermVar | TermNode`), so a guard can structurally
never target a literal-valued fact as a condition. This is exactly why
`renderTemplateWith` treats literal attributes as content, not
eligibility — there was never a version of this design where they
could be conditions, once the real grammar was checked instead of
assumed.

## A real, unrelated bug found and fixed getting the corpus to parse at all

`TIO.readFile` failed on `0121-r11-glm-commit.dmml` with `hGetContents:
invalid argument (cannot decode byte sequence starting from 226)` — NOT
corrupt content: `cat -v` showed a genuine UTF-8 em-dash
(`M-bM-^@M-^T`) in a real model-authored `purpose` string, decoded
wrong only because `TIO.readFile` uses the process locale's encoding,
not explicit UTF-8. Fixed by reading raw bytes
(`Data.ByteString.readFile`) and decoding explicitly
(`Data.Text.Encoding.decodeUtf8`) — the same fix this session's
`DMML.Atproto` client already needed once, for the same underlying
reason.
