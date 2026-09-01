# DMML Surface-syntax MACHINE authoring-compliance checkpoint

9 (model, scenario) dispatches scored against the real `DMML.Surface.parseMachineSurface` production parser.

## Pass rate by model

| model | accepted | rejected | pass rate |
|---|---|---|---|
| google/gemini-3.7-flash | 3 | 0 | 100% |
| moonshotai/kimi-k2.5 | 3 | 0 | 100% |
| z-ai/glm-5.3-flash | 3 | 0 | 100% |

## Pass rate by scenario

| scenario | accepted | rejected |
|---|---|---|
| Machine: guard pattern anchored on a parameter, not self, with two hops | 3 | 0 |
| Machine: single guarded transition | 3 | 0 |
| Machine: three states, two transitions, one negated guard | 3 | 0 |
