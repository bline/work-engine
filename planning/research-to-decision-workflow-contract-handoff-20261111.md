# Next phase handoff: Research-to-Decision Workflow Contract (2026-11-11)

Status: **handoff, not authorization.** The next phase needs its own owner-approved objective and budget.

## Objective

**Design the complete Research-to-Decision Workflow Contract before implementing A4.**

Direction: [Research Knowledge Lifecycle and Decision Workflow — v0.1](research-knowledge-lifecycle-and-decision-workflow-v0.1-20261111.md).

The phase should determine:

1. Durable evidence and assessment objects.
2. Uncertainty representation and maintenance.
3. Evidence aging and revalidation.
4. Decision-specific knowledge consumption.
5. Historical decisions and correction behaviour.
6. Applicable authorization boundaries.
7. Supervisor behaviour when evidence is insufficient.
8. Feedback from independently verified execution.
9. The minimum working Rust implementation that demonstrates the complete lifecycle.

## Starting state

- Shared branch `rescue-detached-20260911`. The integrated Rust research register (R-1b.1..R-1b.6, O3 ACCEPT
  28759344) is at merge `6f3d6c71`.
- R-2 (Rust anchors) is preserved and **not integrated**:
  [checkpoint record](../records/research-register-r2-qualification-checkpoint-20261111.md). The F-01 repair is at
  `0d021609`. Independent acceptance was not completed; the coordinator concluded ACCEPT WITH RESERVATIONS.
- A4 implementation is paused. Its owner decision table from the R-2 reconcile is an input for the
  authorization and card questions above.

## Blockers (actual)

| Blocker | Blocks | Owner |
|---|---|---|
| Q-B1: diff implementation provenance and licensing | Pushing or distributing R-2; integrating R-2 | Owner / legal |
| Q-R2-2: document and rename anchor behaviour | Relying on anchors across renames in A4 | Owner |
| Completed independent acceptance of R-2 (check 8 at least) | Integrating R-2 as independently qualified | Owner (needs a separately authorized recheck) |
| D-2, D-3, D-8 and the other reserved A4 items | A4 card implementation | Owner |
| This contract | A4 implementation (by the owner's 2026-11-11 change of approach) | The design phase |

## Deferred improvements (not blockers)

- R-2 testing: end-to-end missing *packed* revision commit; post-validation ambiguity beyond unit level.
- R-2 resource bounds (FO3-01): no built-in time/byte/decision limits; Myers worst case O((N+M)D).
- CI: the history test needs full repository history (Q-B5); the workspace-wide `cargo check` has not been
  completed in review.
- Review infrastructure: under the Pi proxy's worst-case reservation (about 1.54 USD at the recheck's context size),
  a 2.00 cap left about 0.52 usable. Size future review caps for the reservation, not only for expected spend.

## Not to do automatically

Do not start A4 implementation, resume R-2 development, integrate R-2, or launch reviewers without a new owner
authorization.
