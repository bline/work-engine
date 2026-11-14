# Work Engine Design

Authority: binding product doctrine. The scoped files below jointly own the
contracts previously collected here; this index does not create a second policy
layer. [PHILOSOPHY.md](PHILOSOPHY.md) indexes the non-normative rationale.

Scope: product invariants, authority boundaries, and design rules.

Load when: use this index to find doctrine relevant to the current decision.
[AGENTS.md](AGENTS.md) owns repository startup and recovery loading.

## Doctrine by decision

| Document | Load when | Original sections |
| --- | --- | --- |
| [Core contracts](docs/contracts/core.md) | Every agent at entry and recovery: authority, invariants, judgment, route revision | Purpose, 1.1, 4, 10, 11, 16, 17 |
| [System design](docs/contracts/system-design.md) | Choosing architecture, semantic owners, boundaries, or product structure | 1.2–1.5, 2.2, 9, 13–15 |
| [Agent instruction design](docs/contracts/instruction-design.md) | Authoring or reviewing instructions, skills, role prompts, or loading rules | 2.1, 2.3, 3.1–3.8, 5 |
| [Evidence and claims](docs/contracts/evidence-and-claims.md) | Selecting or relying on evidence, writing receipts, defining production-path claims | 3.9, 6, 12 |
| [Context and review lifetimes](docs/contracts/context-and-review.md) | Coordinating agents, conducting review, retaining or replacing context | 7, 8 |

All these contracts retain their authority when applicable. Selective loading
reduces irrelevant context; it does not waive an invariant. Consult the scoped
owner before a decision governed by it. A link supplies navigation, rather than
a recursive instruction to load its entire dependency tree.

Runtime role instructions project the relevant contracts and their causal
reasons. Detailed role execution stays with the owning skill or campaign.
Repository-specific [context recovery](docs/contracts/context-recovery.md) and
[shared coordination](docs/agent-coordination.md) apply as AGENTS.md specifies.
