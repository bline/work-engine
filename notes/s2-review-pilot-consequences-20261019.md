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

## 4. Narrowing the candidate addition: name the state, not just the observation

Added 2026-10-20 after the candidate in section 2 was correctly challenged. The
challenge: requiring a *discriminating observation* makes a judgment locally
**challengeable**, but not locally **complete**. `opus-5` did enumerate the
synchronization paths it considered and still omitted `recv`. How would a system
that did not already know about `recv` notice the enumeration was short?

That is a fair hit, and "collapses the regress" was too strong. But in this case
the answer is mechanical, and the worked example is worth keeping.

**Measured in the pinned snapshot:**

| set | sites | enumerable by |
|---|---|---|
| registry-lock acquisitions in `executor.rs` | **9** - `:227`, `:254`, `:271`, `:282`, `:293`, `:315`, `:353`, `:367`, `:413` | one grep |
| `results.pending` lock acquisitions | **2** - `:150` (`recv`, reader), `:496` (publisher) | one grep |

`opus-5`'s five named holders (`close_gate`, `request_abort`, `close_and_drain`,
`acknowledge_result`, `owned_count`, plus `submit` itself) **cover all nine
registry sites**. Its enumeration was complete and correct **for the set it
named**. It executed the right method on the **wrong set**.

**So the defect is a property-to-state binding error, not an incomplete search.**
The claim was about a result becoming *observable*. Results live in
`results.pending`. The accessor set of that state has two members and is trivially
enumerable. A warrant assembled from *registry*-lock holders therefore exhibits a
visible mismatch between the state its claim ranges over and the state it
enumerated - **detectable without prior knowledge that `recv` exists.**

**Revised candidate, narrower and mechanically checkable.** A positive
concern-scoped judgment should name:

1. the **property** it establishes;
2. the **state** that property ranges over;
3. the **accessor set** of that state which it enumerated, and the basis for
   treating that set as exhaustive at the subject revision.

Given (1) and (2), (3) is checkable by a deterministic tool. What remains
irreducibly a matter of judgment is only whether (1) is the right property for
the governing contract - which is the frame question, and is far smaller than
verifying the whole judgment.

**This does not collapse the regress; it narrows it.** The claim is not that
every judgment becomes self-certifying. It is that the non-mechanical residue
shrinks from "is this conclusion sound" to "is this the property the contract
requires", and the gap between the two is exactly where `opus-5` failed: it
answered a true question (is acknowledgement serialized) that was not the
contract's question (can work start, and a result become visible, before
registration).

Cost and limits, stated: this requires three additional fields on every positive
judgment, some concerns will not have a single well-defined state their property
ranges over, and the accessor-set check is only as complete as the tool's notion
of an access - dynamic dispatch, callbacks and indirect state reach can defeat a
purely syntactic enumeration. That last limit is real and means the mechanical
check is a **necessary** condition, never a sufficient one.

Still a candidate. The owning proposal is `formed` with
`implementation_authorized: false`, and this note does not edit it.

## 4b. Correction to section 4: my accessor enumeration was incomplete

2026-10-20. Section 4 states that the set relevant to the F4 property "has
exactly two members - `:150` (`recv`, reader), `:496` (publisher)" and calls it
"two lines of grep". **Both claims are wrong.**

There are **four** accessors of `results.pending`, three of them readers that
leak a full `TaskResult`:

| site | accessor | visibility | leaks a result? |
|---|---|---|---|
| `:150` | `ResultReceiver::recv` | `pub` | yes |
| `:160` | `ResultReceiver::pending` | `pub` | yes |
| `:233` | `OwnedExecutor::pending_results` | `pub` | yes |
| `:496` | publisher, inside the spawned collector | - | writes |

My grep required `pending` and `.lock()` on the same line; Rust method chains
span lines, so `:158` and `:230` were invisible to it.

This was found by the `B` arm of generation 3, which also supplied a **better
exhaustiveness basis** than mine: `ResultState`, its `pending` field,
`ResultRecord` and `ResultReceiver`'s fields carry no `pub`/`pub(crate)`
modifier and `lib.rs:16-20` re-exports only type names, so **module privacy
confines every accessor to `executor.rs`** and a grep of that one file is
exhaustive - an argument from the language's visibility rules rather than from a
text pattern.

**What this does to the candidate.** It survives and is strengthened; its
demonstration does not. I used the two-member grep as the worked example for
"given the property and the state, the accessor set is deterministically
checkable", and the example was produced by a defective check. The lesson is
that the **exhaustiveness basis is the load-bearing field**, not the list: a
list without one is an assertion, and mine had none.

## 5. Refinement: the binding is to a causal surface, not a single state

Added 2026-10-20. Section 4's three-field schema (property / state / accessor
set) is **insufficient for the very example that motivated it**, and the contract
text shows why.

**The contract states invariant 1 as an ordering, twice, in temporal terms:**

> line 17: "eagerly register entry and sink **before scheduling**"
>
> line 31: "The supervisor holds the registry/result sink. **Work cannot start
> until registry insertion and sink creation finish.**"

Two states are named - `registry` insertion **and** sink creation - and the
requirement is a *happens-before* relation against a third event, work starting.
A property over one state and its accessor set cannot express that.

**What `opus-5` actually did, in the invariant's own words.** It quoted line 31
verbatim, then wrote: "holding a `std::sync::Mutex` does not stop the task
spawned at line 448 from calling the provider." That sentence **is** the
violation of the sentence it quoted - work starts before registry insertion
finishes. It then cleared the invariant by answering a different question, whether
a consumer could act on an unregistered result, and answered that one correctly
for the registry half.

So the failure is not a missed access path and not faulty reasoning. It is a
**correspondence failure between the property the contract required and the
proposition the evidence established.**

### Two distinct completenesses, and only one of them was satisfied

| completeness | question | how established |
|---|---|---|
| **evidence** | were all relevant accesses within the declared domain enumerated? | deterministic enumeration, exact-revision inspection, explicit unresolved references |
| **warrant** | does that evidence establish the property the contract requires? | causal and temporal reasoning, counterexample, semantic judgment |

`opus-5` satisfied evidence completeness for the domain it chose - its five named
holders cover all nine registry-lock sites - and failed warrant completeness. An
exhaustive enumeration of the wrong domain is still exhaustive, and still
irrelevant. This is why a list of inspected functions is not a sufficient
requirement.

### Why this is compiler work rather than another review layer

The ordering obligations are **already in the contract text**, in explicit
temporal language, at two separate lines. They do not need to be inferred. A
compiler could derive the proof obligations directly - registration completes
before work starts; sink creation completes before work starts; a published result
is not observable before both - and hand the reviewer the remaining semantic
question instead of requiring it to choose which property matters.

That is a cheaper intervention than adding a reviewer, and it addresses the
observed failure rather than a hypothesised one. Compiler implementation is step 4
of `post-s10-direction-20261019.md` and is **not authorized**; this is evidence
for that step, not a grant.

### Observation on effort, which cuts against a tempting conclusion

"More thinking would not have helped" is the natural reading, and the data does
not support it. Thinking tokens against F4 outcome, all four arms at `xhigh`:

| arm | thinking tokens | F4 outcome |
|---|---|---|
| `haiku-5-5` | 84,593 | **MATCH** |
| `sonnet-5-5` | 46,448 | UNRESOLVED-DECLARED |
| `opus-5` | 29,984 | FALSE CLEARANCE |
| `opus-5-5` | 25,775 | FALSE CLEARANCE |

Perfectly monotonic across four arms: more thinking, better F4 outcome, with the
two clearances at the bottom. **Confound stated:** thinking volume is entangled
with model identity, and a smaller model may emit more tokens for the same work,
so this does not isolate reasoning quantity from model choice. n=1 per cell. But
any claim that effort was irrelevant here must contend with a clean monotonic
ordering in the opposite direction.

## Qualification against contract, 2026-10-21 (revision `7f72a17b`, pushed)

The source-currentness check is confirmed: all four patterns are present at
`7f72a17b`, the tip of `origin/rescue-detached-20260911`. Qualifying them against
the clauses they would violate changes three of the four. **One qualifies cleanly,
one qualifies on a clause I had not cited, one is partly a contract referral, one
fails qualification as stated.** Verified by reading each containing function at
that revision.

### Concern 2 qualifies, and against a second clause as well

`close_and_drain` (`executor.rs:343`) performs invariant 6's sequence as: close
the gate, snapshot `registry.tasks.keys()`, then `observe_terminal()` under
`timeout_at`. `request_abort` exists at `executor.rs:324` and is called from
`tests/owned_execution.rs:367` only. Invariant 6's "**request supported
cancellation**" is a mandatory clause, the mechanism exists, and the shutdown
sequence does not invoke it: a task that will not terminate is reported
`unresolved` and left running.

The same function fails a second invariant-6 clause. The invariant requires
recording "close attempted/resolved/rejected/timed-out/late"; the implementation
collapses them - `Ok(false) | Err(_) => unresolved.push(attempt)` merges "observed
non-terminal" with "deadline expired", and `TaskDrainReport` has no field able to
distinguish them. This is the same state-collapse that grounds invariant 6's
standing INSUFFICIENT disposition, now located in one expression.

### Concern 1 narrowed: part defect, part contract referral

**Correction.** I wrote that "the deadline invariant is wired nowhere in the owned
execution path". The dead-seam half is right - `MonotonicDeadline`,
`DeadlineStatus` and `ClockSource::sample` have no caller outside
`tests/clock_framing.rs` - but the conclusion overstates. `close_and_drain`
enforces an absolute deadline by construction:
`tokio::time::timeout_at(tokio::time::Instant::from_std(deadline))` is anchored to
an absolute instant, so `Ok(true) => joined` cannot be reached after the deadline.
Error class: reading a typed seam's absence as the absence of the behaviour it
would implement. The check that would have caught it: read the function body
rather than grepping for the type's name.

What is genuinely uncovered is invariant 4's "before final success" on the
**ordinary** completion path (`executor.rs:505-545`), which publishes a
`TaskResult` with `finished_wall`/`finished_mono` and no deadline comparison at
all. And invariant 4's other clause - "typed wall time for S1 authority checks" -
is **unrealisable inside the accepted boundary** per E4 conflict 1, since no
authority expiry reaches S2. So this concern splits: an ordinary-path check that
is actionable now, and an authority-expiry requirement that needs a contract
referral, not an implementation fix.

### Concern 3 resolved: no invariant violated, hazard stands

The non-consuming read is deliberate and documented. `acknowledge_result`
(`executor.rs:289`) carries "Call only after S4 has committed the result or its
unresolved disposition" and is the consuming path; `pending()` carries "A missed
or dropped notification cannot erase an owned result". That is an acknowledge
protocol, and it is what invariant 3's "never drop required evidence silently"
asks for. No evidence is lost: an unreachable result is still returned by
`pending()`.

The head-of-line consequence is unchanged and is a liveness hazard, not a
contract violation. `recv` returns `pending.values().next()`, the lowest
`AttemptId`, so until S4 acknowledges that attempt every later completion is
unreachable through a method named `recv`, and because `recv` returns early
without consuming a notification the notification queue drifts from the pending
set. File as an API/design clarification, not a defect.

### Concern 4 fails qualification as stated

**Correction.** I wrote that `TaskExit` "reports unavailable measurements as
observations". Printing the containing function refutes it for both fields.
`local_task_ended: true` is assigned inside the `outer` task after
`inner.await`, where `inner` is the inner task's `JoinHandle`
(`executor.rs:286`); every branch - value, `is_cancelled`, panic - is reached only
once that task has ended. The literal is therefore true by position.
`child_exit_code: None` is assigned in `OwnedExecutor`'s task path, which has no
child process, so `None` is the accurate value rather than an unavailable one.

Error class: scoring a literal as a fabricated observation without reading what
the surrounding control flow had already established - rule 4, print the
containing object.

A narrower §12 concern survives at a different layer. This is the only producer of
these fields (`local_task_ended: true` appears nowhere else in `rust/`), and
`lifecycle-service/src/actor.rs:2252-2253` forwards them to
`task_results.child_exit_code`, a nullable `INTEGER`
(`lifecycle-store/migrations/0002_service.sql:10`). A reader of that column cannot
distinguish "no child existed" from "child exit unknown". That is a persisted
representation question for the store/service owner, not an executor bug.

### Intake consequence

**One bug report, not four.** Concern 2 is the single clean candidate, carrying
both invariant-6 clauses. Concern 1's ordinary-path check is actionable and can
accompany it or stand alone; its authority-expiry half and concern 4's schema
question are referrals; concern 3 is a clarification request.
