## Invariant catalog

| ID | Owner | Applies | Class | Condition | Causal parent / invalid state | Enforcement | Mechanisms | Relations | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `INV-001` | human | all | authority | Human owns contract changes. | Silent changes are unauthorized. | human | `MECH-ONE` |  | fixture |

## Current machinery catalog

| ID | Current component | Affordance | Primary invariant edges |
| --- | --- | --- | --- |
| `MECH-ONE` | fixture | validates | enforces `INV-001` |
