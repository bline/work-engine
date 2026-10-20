# Consequences of the S2 review pilot, 2026-10-19

Two things that fall out of Test R (`metrics/probes/claude-s2-review-independence-20261019-protocol.md`,
results at `94bde53`) and belong against **current state** rather than inside the
frozen historical benchmark. The benchmark stays untouched so the research
evidence and the production obligation do not contaminate each other.

Status of this note: **observations and a proposed obligation, not authorization.**
Nothing here accepts a finding, reopens a slice, or changes any campaign.

## 1. Four verified concerns survive into the accepted revision and current HEAD

Found by the Claude reviewer arms on S2 candidate 2 (`7fdf392f`), verified by me
against source, and confirmed still present at the **accepted** C5 revision
`4071955` **and at current `HEAD`**:

| concern | evidence | plan relation |
|---|---|---|
| The deadline invariant is wired nowhere in the owned execution path. `MonotonicDeadline::status` is defined at `clock.rs:69` and called from neither `executor.rs` nor `process.rs`; `ClockSource::sample` is dead code. | zero matches for `MonotonicDeadline`/`DeadlineStatus` in `executor.rs` at both revisions | **Directly challenges plan invariant 4**: "Check absolute deadlines after awaits and before final success. A late close is recorded as late without extending proof authority." |
| `close_and_drain` never requests cancellation, though `request_abort` exists at `executor.rs:270`. | zero `abort()` calls inside `close_and_drain` at both revisions | **Directly challenges plan invariant 6**, which requires "request supported cancellation". |
| `ResultReceiver::recv` is non-consuming **and** head-of-line: `pending` is a `BTreeMap`, so `.values().next()` always yields the lowest key. | `executor.rs:116` `pending: Mutex<BTreeMap<..>>`; `recv` at `:147` | Contract significance **undecided**. An adjacent comment suggests the non-consuming part is deliberate; the head-of-line consequence is probably not. |
| `TaskExit` emits `local_task_ended: true` and `child_exit_code: None` as literal constants. | `executor.rs:487-489` at both revisions | Reports unavailable measurements as observations - the `DESIGN.md` §12 distinction. Possibly deferred scope, but it is not labelled as such. |

The first two are the consequential ones: they are invariants the accepted slice
claims to satisfy.

**Why they survived.** Review findings become remediation obligations. The C3/C4
dispositions scope their `repair_boundary` to the files the findings named, so the
remediation loop only ever addressed `process.rs` and `executor.rs` for the
*reported* defects. These four were never reported, so `no_open_findings` at C5
was true and told us nothing about them.

**`no_open_findings` is a statement about the known finding set, not a
correctness claim.** That was already arguable in principle; it is now
demonstrated with four instances in shipped code.

Proposed next step, for the owner: treat these as a **newly discovered
conformance obligation against the current revision**, assessed on their own
evidence, rather than as a reopening of S2's historical acceptance. The accepted
record is a true record of what was known.

## 2. A gap in the concern-scoped review judgments proposal

`proposals/adaptive-specialized-review/concern-scoped-review-judgments/` is
`lifecycle_state: formed`, `implementation_authorized: false`, decision owner the
user. Nothing below changes that.

Its protected boundary "Positive review meaning remains bounded"
(`proposal.md:148-157`) already anticipates most of what this pilot found:

> a no-finding result can otherwise be misread as complete inspection, universal
> correctness, approval, or acceptance... Absence of a finding does not imply a
> judgment for an unlisted concern or subject.

Section 1 above is direct formation evidence for that boundary.

**But the pilot exposed something the boundary does not cover.** It governs the
**absence** of a finding. The `opus-5` arm's failure was not an absence: it was
an affirmative positive judgment that was **wrong**. It placed the
registration-ordering defect under "What the candidate gets right (checked, not
assumed)" and argued there was no observable accepted-but-unregistered window,
reasoning from a true premise about `acknowledge_result` to a false conclusion
about publication, which travels through a different mutex.

A `ReviewJudgment` recording that clearance would have been **perfectly
well-formed**: exact concern, exact subject `7fdf392f`, evidence cutoff,
relied-on evidence, exclusions, limitations, producer, disposition positive. Every
required property satisfied, and the content false.

So the required-property list makes a positive judgment **attributable and
scoped**. It does not make it **testable**. Scope-bounding prevents over-reading
a correct judgment; it cannot detect an incorrect one.

**Candidate addition, recorded as a candidate and not a decision:** a positive
concern-scoped judgment should be required to carry the *discriminating
observation* that would have falsified it - the specific state, ordering or input
whose absence the reviewer checked - so that a later consumer can re-test the
judgment rather than only re-attribute it. Opus's clearance would have had to
name the publication path it believed was gated, which is exactly where it was
wrong.

This is stronger than "reviewers should be careful" because it has a falsifier,
and it costs something real: it is additional required content on every positive
judgment, and some concerns may not admit a single discriminating observation.
Whether that cost is worth paying is the decision owner's call, and adding this
to the proposal's `formation-evidence.md` is their call too - this note does not
edit a formed proposal.

## 3. The harness defect, diagnosed

The 10 permission denials that hampered `opus-5` were **not** git-specific.
`--permission-mode dontAsk` denies Bash generically for commands outside its
allowlist; Sonnet's first `git diff --stat` happened to pass and Opus's variants
did not. Verified fix, preflighted at a cost of $0.0037: `--permission-mode
bypassPermissions` with `--disallowedTools Write Edit NotebookEdit`, which ran the
exact baseline diff with zero denials and left the tree clean. Write detection
continues to rest on the disposable per-arm copy plus a post-hoc
`git status --porcelain`, as before.

Any further arm must preflight its exact commands before freezing.
