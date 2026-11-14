# Work Engine Philosophy

Authority: explicitly non-normative. These documents explain why the
[design doctrine](DESIGN.md) exists. They do not add implementation requirements.

Scope: the reasoning behind product doctrine and its runtime projections.

Load when: a design decision or review needs the reasoning behind a contract.
Choose the relevant topic; startup, planning, or review does not by itself require
reading the entire rationale corpus.

## Rationale by question

| Document | Consult for | Original sections |
| --- | --- | --- |
| [Model judgment](docs/rationale/model-judgment.md) | Judgment, procedures, and instruction pressure | 1, 2, 6, 7, 15, 16 |
| [Structural anchors and logical continuity](docs/rationale/structural-anchors.md) | Invariants, causal explanations, and stale conclusions | 3, 5, 8–10 |
| [Capability and interface design](docs/rationale/capability-design.md) | Mutable machinery, affordances, and action-space design | 4, 11–13 |
| [Context and instruction loading](docs/rationale/context-and-instruction-loading.md) | Context lifetime and the documentation stack | 14, 17 |
| [Production-path claims](docs/rationale/production-path-claims.md) | Execution history, custody, observation, and establishment | 18 |

## 19. A compact statement of the philosophy

Work Engine aims to give models:

- a small, stable set of structural anchors;
- clear causal explanations for those anchors;
- an accurate description of the machinery currently available;
- the relevant state and evidence;
- explicit objectives and consequences;
- freedom to choose the method.

The intended result is neither unrestricted agency nor procedural automation.

It is a model-centered system in which:

> **Structure defines the valid world. Knowledge describes the current world. Judgment chooses the path through it.**
