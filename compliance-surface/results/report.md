# DMML Surface-syntax authoring-compliance checkpoint

15 (model, scenario) dispatches scored against the real `DMML.Surface.parseCommitSurface` production parser.

## Pass rate by model

| model | accepted | rejected | pass rate |
|---|---|---|---|
| google/gemini-3.7-flash | 5 | 0 | 100% |
| moonshotai/kimi-k2.5 | 5 | 0 | 100% |
| z-ai/glm-5.3-flash | 5 | 0 | 100% |

## Pass rate by scenario

| scenario | accepted | rejected |
|---|---|---|
| Adversarial: task phrased in the retired schema's vocabulary | 3 | 0 |
| Consumes-only commit, no new facts | 3 | 0 |
| Fact-level consume (retraction) + a via reference | 3 | 0 |
| One commit, mixed value types | 3 | 0 |
| Simple commit: declare + facts | 3 | 0 |
