# DMML head-to-head: JSON vs. Surface syntax

15 JSON dispatches + 15 Surface dispatches, same models, same tightened scenarios, scored against each format's own real parser.

## Side-by-side outcome per (model, scenario)

| model | scenario | JSON | Surface |
|---|---|---|---|
| google/gemini-3.7-flash | Simple commit: declare + facts | accepted | accepted |
| google/gemini-3.7-flash | One commit, mixed value types | accepted | accepted |
| google/gemini-3.7-flash | Fact-level consume (retraction) + a via reference | accepted | accepted |
| google/gemini-3.7-flash | Consumes-only commit, no new facts | accepted | accepted |
| google/gemini-3.7-flash | Adversarial: task phrased in the retired schema's vocabulary | accepted | accepted |
| moonshotai/kimi-k2.5 | Simple commit: declare + facts | accepted | accepted |
| moonshotai/kimi-k2.5 | One commit, mixed value types | accepted | accepted |
| moonshotai/kimi-k2.5 | Fact-level consume (retraction) + a via reference | accepted | accepted |
| moonshotai/kimi-k2.5 | Consumes-only commit, no new facts | accepted | accepted |
| moonshotai/kimi-k2.5 | Adversarial: task phrased in the retired schema's vocabulary | accepted | accepted |
| z-ai/glm-5.3-flash | Simple commit: declare + facts | accepted | accepted |
| z-ai/glm-5.3-flash | One commit, mixed value types | accepted | accepted |
| z-ai/glm-5.3-flash | Fact-level consume (retraction) + a via reference | accepted | accepted |
| z-ai/glm-5.3-flash | Consumes-only commit, no new facts | accepted | accepted |
| z-ai/glm-5.3-flash | Adversarial: task phrased in the retired schema's vocabulary | accepted | accepted |

## Aggregate pass rate

| format | accepted | rejected | unparseable | pass rate |
|---|---|---|---|---|
| json | 15 | 0 | 0 | 100% |
| surface | 15 | 0 | 0 | 100% |

## No divergences

Every (model, scenario) pair got the same accept/reject outcome in both formats.
