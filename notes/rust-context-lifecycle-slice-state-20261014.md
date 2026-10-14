# Context-lifecycle slice state — 2026-10-14

## Status

Observation note, written at HEAD `cfafee4` while development was paused for
quota. It carries no authority: it accepts nothing, admits no scope, retires no
obligation, and is not a plan amendment. Every claim below either quotes a
repository file or is attributed to its source. Read the cited files before
acting on anything here; they are current and this note will age.

Two inputs that are not repository evidence, recorded as such: **the user states
S8 was completed**, and that the UI-contract gap was found afterwards in a
separate UI session. This note did not verify S8 completion.

## Where the slices stand

The canonical sequence is S0–S10 in
[`experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md`](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md).
Three slices are in play.

**S8 — one retained workflow through the complete host path.** Completed (user
statement). A post-completion correction is in progress because the completed
state could not meet the UI consumer's contracts; that was not designed for
initially. The correction is recorded by the UI side in
[`rust/crates/terminal-ui/docs/task-observation.md`](../rust/crates/terminal-ui/docs/task-observation.md),
prepared 2026-10-13 from UI baseline `d4716fc`, which states it is "the UI
consumer's proposed contract and implementation scope, not an accepted S8
protocol change", that "the user authorized this reconciliation", and that "S8
owner acknowledgement, exact source handoff and backend qualification remain
pending; board message 1659 requests that handoff". The S8 owner has since been
implementing it.

**S9 — recovery/migration integration and rollback qualification.** Depends on
S7 and S8. Active and late-stage: the board shows Candidate3 `08aa694e`
superseded by Candidate4 `906db9b0`, with R4/R7 remediation following a root
readback at 2026-10-14T03:13Z that required changes.

**S10 — replacement qualification and operational handoff.** Depends on S9.
Last slice: freeze the integrated candidate, complete the provider-disabled
production-composition gate, execute the selected review, and have all T01–T37
carry "explicit evidence or an honestly unresolved applicability/coverage
decision".

## The UI correction used a route S8's plan had already authorized

The S8 plan's public-contract row in
[`docs/rust-context-lifecycle-s8-implementation-plan.md`](../docs/rust-context-lifecycle-s8-implementation-plan.md)
says: "Existing wire v1's qualified Unknown projection for internal Yielded
remains truthful. If the consumer needs exact Yielded, add a versioned supported
projection rather than silently changing v1."

That is what happened. `task.observe.v2` and the complete command-result v2 are
additions; v1 remains the supported grant contraction readback, and
[`docs/rust-retained-task-host-contract.md`](../docs/rust-retained-task-host-contract.md)
describes TO2 read-value validation as shared by host, UI and scoped proof
receipts through passive validators that "confer no authority". So the gap was
closed by extension rather than by breaking the accepted protocol.

## Open item: TO2 has no acceptance anchor

TO2 has implementation and tests. It does not appear in S8's acceptance mapping,
which lists M4, T02, T10–T22, T27, T29 and T30, and no test in T01–T37 covers
independent task observation (T37 is dependency and profile reproducibility).

S10 requires that "all T01–T37 have explicit evidence or an honestly unresolved
applicability/coverage decision; omission is not a pass and cannot weaken a
required replacement claim." Scope that exists only as code and tests therefore
arrives at the final sweep as neither covered nor consciously excluded, which
that sentence refuses to treat as a pass.

Cheapest resolution while the S8 owner still holds the context: record TO2
either as an amendment row in S8's acceptance mapping or as a deliberate S10
coverage decision. Reconstructing the intent later is the expensive path. This
note does not decide which, and neither choice is implied by observing the gap.

## The decision that actually gates S9

Two current files say the same thing, and it is a human decision:

- [`docs/rust-development.md`](../docs/rust-development.md): "Import versus an
  explicitly authorized archive/clean start remains a decision for the
  human/state owner."
- [`docs/rust-context-lifecycle-s8-implementation-plan.md`](../docs/rust-context-lifecycle-s8-implementation-plan.md):
  "S9 retains actual saved-state import versus authorized archive/clean start,
  current-custody rollback, selected permanent deployment/startup, old
  executable refusal and historical-codec qualification. **No actual state
  choice has been made.**"

The implementation plan adds that T25 and candidate-current rollback evidence
"remain required by the existing acceptance contract until that owner accepts an
exact amendment; an archive choice cannot silently retire unresolved
obligations."

So S9's remediation cycles can close without it, and S9 cannot. Making the call
before the next run lets S9 close against a known target instead of
accumulating candidates around an open question.

## Suggested order when development resumes

1. Make the import-versus-archive-or-clean-start decision, or state explicitly
   that it is deferred and what that defers with it.
2. Give TO2 an acceptance anchor while the S8 owner still has the context.
3. Finish the S8 UI correction and the S9 R4/R7 remediation.
4. Then S10, where the T01–T37 sweep is the work rather than a formality.

## Related measurement

The cost of the runtime these slices replace is measured in
[`metrics/experiments/context-replacement-20261014/`](../metrics/experiments/context-replacement-20261014/)
and summarized in [`metrics/efficiency-ledger.md`](../metrics/efficiency-ledger.md)
under Per-call cost: 277 context replacements across 61 sessions, all
ceiling-driven, with both sides of the replacement inequality quantified. That
package is evidence about the current route, not about these slices.
