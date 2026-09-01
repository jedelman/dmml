# DMML informed-vs-blind authoring checkpoint

6 dispatches scored against the real `DMML.Surface.parseCommitSurface` parser. Question: does a materialized world snapshot prevent predicate-name drift?

## Predicate chosen for (shrine/threshold, ?, offering/incense), by condition

| model | condition | outcome | predicate chosen | matches existing ('accepts')? |
|---|---|---|---|---|
| google/gemini-3.7-flash | blind | accepted | acceptsOffering | no |
| google/gemini-3.7-flash | informed | accepted | accepts | yes |
| z-ai/glm-5.3-flash | blind | accepted | accepts | yes |
| z-ai/glm-5.3-flash | informed | accepted | accepts | yes |
| moonshotai/kimi-k2.5 | blind | accepted | acceptsOffering | no |
| moonshotai/kimi-k2.5 | informed | rejected | - | n/a |

## Summary

- Blind condition: 2 distinct predicate name(s) chosen across 3 accepted replies: ['accepts', 'acceptsOffering']
- Informed condition: 1 distinct predicate name(s) chosen across 2 accepted replies: ['accepts']
- All informed replies matched the existing 'accepts' relation: True

## Redeclared an already-declared predicate anyway

- google/gemini-3.7-flash / informed: redeclared ['accepts']
- z-ai/glm-5.3-flash / blind: redeclared ['accepts']
