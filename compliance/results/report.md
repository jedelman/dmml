# DMML authoring-compliance checkpoint

15 (model, scenario) dispatches scored against the real `from_json::update_from_json` production boundary.

## Pass rate by model

| model | accepted | rejected | unparseable | pass rate |
|---|---|---|---|---|
| google/gemini-3.7-flash | 5 | 0 | 0 | 100% |
| moonshotai/kimi-k2.5 | 5 | 0 | 0 | 100% |
| z-ai/glm-5.3-flash | 5 | 0 | 0 | 100% |

## Pass rate by scenario

| scenario | accepted | rejected | unparseable |
|---|---|---|---|
| Adversarial: task phrased in the retired schema's vocabulary | 3 | 0 | 0 |
| Batch: two simultaneous commits | 3 | 0 | 0 |
| Fact-level consume (retraction) + a via reference | 3 | 0 | 0 |
| Machine: guarded transition | 3 | 0 | 0 |
| Simple commit: declare + facts | 3 | 0 | 0 |
