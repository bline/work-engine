# R2 finding inventory, 2026-10-24

Author: `claude-metrics` session `accb0998-becf-4a0a-80c1-ce46b5bc9d51`, model
`claude-opus-5`. Owed to peer `context-efficiency-planning-root` since board
**2764**; this file discharges it.

**Revision 2, 2026-10-24**, by successor session
`078cd138-96be-46cb-ac48-3c353e2b357e`, same role and model, answering the peer's
challenge `finding-inventory-challenge-v1.json` (board **2783**). Five claims are
narrowed or withdrawn in place and labelled `FI01`-`FI05` where they sit: the
read-versus-run framing (§5), the survivorship wording (§2), the 0.591 Mwt
recheck cost (§7), the historical P1 claim (§8a) and the campaign-wide zero
(§4). **§5a is new** and supplies the per-finding four-question mapping the
challenge asked for. The peer's review is a semantic review of this artifact's
claims; it did not independently certify the underlying findings, and neither
participant supplies independent semantic adjudication.

**Authority.** Every classification below is this session's own model judgment,
carrying this role's authority and no one else's. Nothing here is human-certified
and nothing here is certified by the peer. The predecessor was corrected at board
**2779** for describing model readings as "a recorded human reading" in the ramp
ledger's generator; that error class is false attribution of authority, and this
artifact names its actor to avoid repeating it.

## 1. The questions this answers, and a provenance limit on them

The inventory is built against four questions:

1. the earliest point at which enough stable evidence existed to detect the finding;
2. what evidence a reviewer would have needed at that point;
3. what repair earlier detection could have avoided;
4. whether later changes invalidate the early conclusion.

**Provenance limit, stated because it bears on whose questions these are.** These
four are recorded in `claude-metrics`' own board post **2764**, which attributes
them to the peer ("your four questions"). A bounded search of board sequences
**2701-2781** finds them in only two places: 2764 and this session's 2780. The
peer did not pose them verbatim on the board inside that range, so their origin is
pre-board user-relay traffic and 2764 is the authoritative recorded wording.
Searched domain: board bodies 2701-2781 via `read --since 2700 --limit 90`,
regex over every body. Sequences at or below 2700 were **not** searched, so this
is "absent from 2701-2781", not absent in general.

## 2. Survivorship limit, stated before the counts

**The record holds detected findings only.** It therefore bounds the cost of the
review that happened, not the cost of the review that should have happened. No
count below is a defect density, and no absence below is evidence that a defect
class does not exist.

**Corrected wording, challenge item `FI02`.** An earlier draft of this section
said the detected set is "known to be narrower than the true set". That
overstated what the three reasons below support: each is a limit on the
**coverage** of the review that ran, and none is an observation of a missed
defect. Unseen defects remain **possible**; completeness of the detected set is
`not established`, which is distinct from a proven strict superset. Three
specific reasons coverage cannot be treated as complete, each read from the
reviews themselves:

- **E1's reviewer truncated its own coverage under context pressure and said so.**
  It read `DESIGN.md` headings and §9 only, "because of the context budget
  (140,834 at the mid-task check, trigger 150,000)", and recorded conformance with
  the rest of `DESIGN.md` as `not established` by that review.
- **E2's first reviewer handed off at 158,847** and **P1's first reviewer handed
  off at 156,071**, both above this host's 150,000 trigger, both forced.
- Every slice's fixtures are synthetic. E1: "every packet and projection is
  synthetic and minimal", no real `packet.json`. P1: whether real comparison
  findings exist is `not established`.

So all three slices show context pressure acting on the review: one declared
coverage truncation and two forced handoffs. That is the inventory's most
decision-relevant output for a context-efficiency pilot, and it is n = 3 slices,
1 truncation and 2 handoffs, each attributed to the reviewer that recorded it.
The two handoff readings are the only 2 of the cohort's 12 context-check readings
above the trigger (§8b), so in this campaign crossing the trigger and handing off
are the same event, with no instance of either without the other.

## 3. Cohort and subject boundary

The subject is the **10 cost-attributed agents** of the R2 end-of-slice review and
post-review remediation cohort, named individually in the R2 score addendum
(`metrics/probes/claude-builder-campaign-r2-score-20261023.md`, commit `e0380259`)
and keyed to `assignments.json` entries:

| cohort | agents | Mwt |
|---|---|---|
| end-of-slice review | `r2_E1_review` 0.737, `r2_E2_review` 0.553, `r2_E2_review_2` 0.546, `r2_P1_review` 0.694, `r2_P1_review_2` 0.879, `r2_P1_recheck` 0.591 | **4.000** |
| post-review remediation | `r2_E1_fix` 0.651, `r2_E2_fix` 0.973, `r2_P1_fix` 0.412, `r2_P1_tests` 0.446 | **2.482** |

Per-agent figures sum to 4.000 and 2.482 exactly, and to 6.482 together; against
the 6.200 Mwt builder cohort that is 64.5%, 40.0% and 104.5%, matching the
addendum's rounded 65 / 40 / 105. Arithmetic re-run this session.

**Declared exclusion, so it is visible rather than silent.** Six further
review-prose artifacts exist under the campaign root and are **out of this
subject**, because their agents are not in the addendum's cost-attributed cohort:
`p1_review_t1` (`review/review-t1-v1.md`), `fix_a`'s review and
`fix_b_review` / `fix_c_review` (`fix-{a,b,c}/review-v1.md`), `r2_tier2_fable`
(`r2-tier2/review-v1.md`), and `final_fable` (`final/review-fable-v1.md`).
Their costs are not inside 4.000 or 2.482, so folding their findings into counts
denominated on those figures would inflate findings against a cost base that never
paid for them. Mapping established from `assignments.json` keys, not from filenames.

**File-set basis.** `e1` 17 files, `e2` 24, `p1-portfolio` 37, listed and counted
this session; the counts match the inherited enumeration exactly.

## 4. Finding counts, each review read on its own structure

The numbering convention differs per review file, so a heading or table scan does
not yield a count — an earlier scan matched 8 in `e1/review-v1.md` and 0 in the
others. Each file was read in full instead.

| review | P1 | P2 | P3 | total | assessment |
|---|---|---|---|---|---|
| `e1/review-v1.md` (`r2_E1_review`) | 0 | 0 | 8 (F1-F8) | **8** | `ready` |
| `e2/review-v1.md` (`r2_E2_review` + `_2`) | 0 | 2 (P2-1, P2-2) | 6 (P3-1..P3-6) | **8** | `blocked` |
| `p1-portfolio/review-v1.md` (`r2_P1_review` + `_2`) | 0 | 1 (P2-1) | 5 (P3-1..P3-5) | **6** | `blocked` |
| `p1-portfolio/recheck-v1.md` (`r2_P1_recheck`) | 0 | 0 | 0 | **0** | `ready` |
| **total** | **0** | **3** | **19** | **22** | — |

n = 22 findings across 3 slices and 4 review artifacts. **Zero P1 findings in the
counted cohort** - the 4 review artifacts of the 10 cost-attributed agents named
in §3. The six review artifacts excluded there were not read for severity, so a
**campaign-wide** zero is `not established` (challenge item `FI05`; an earlier
draft said "zero P1 findings in the campaign"). Severity labels are each review's
own; they are not re-graded here.

## 5. The central result: detectability class

**What this section is, after challenge item `FI01`.** It classifies each of the
22 by its **observed evidence modality**: whether the review's own record shows
the finding reached by reading sources against each other, or by running
something. It is an **aggregate qualified judgment** of this session, published
as counts. It is *not* a per-finding answer to the four questions - read-versus-run
is a modality, not an earliest-stable-evidence boundary. §5a supplies the
four-question mapping for the decision-relevant subset; no all-22 mapping is
claimed.

- **Detected by reading: 17 of 22.** The review's record shows them reached by
  reading code against the contract, or one document against another, with
  nothing run.
- **Discovered by execution: 5 of 22.** `E1 F4` (400-digit integer and `1e400` exit 2
  rather than exit 1 `schema`; found by probe), and four mutation survivors —
  `E2 P3-4` (M8/M9/M10 boundary mutants), `P1 P3-3` (M16), `P1 P3-4` (M15),
  `P1 P3-5` (M4b). Execution is the **observed
  discovery and proof route** for these five. That execution was *necessary* -
  that no read of the sources would have sufficed - is `not established`, and for
  the four mutation survivors it is doubtful on the reviews' own words: they are
  classed as missing test pins against a rule set, which is a document-against-test
  comparison. An earlier draft labelled this class "requires execution".

Two consequences, which are the answer to question 3 at the level of the campaign
rather than the individual finding:

1. **All three blocking findings are in the readable class.** `E2 P2-1`,
   `E2 P2-2` and `P1 P2-1` each blocked a slice, and each is a document-against-code
   comparison needing nothing run. The evidence a reviewer needed was, respectively:
   contract-v2 §5 lines 424/427 against `lib.rs:19`; contract lines 64-68 (S6)
   against §3.4's rule list; and the comment at `upstream.rs:71-73` against the
   structure-rule set.
2. **Of the 5 discovered by execution, 4 are test-coverage gaps rather than behavioural
   defects** — the mutation survivors, which the reviews themselves classify that
   way (`P1 P3-3`: "this is a test gap and not a defect"; `E2 P3-4`: "Boundary rules
   are correct but no test pins them"). `E1 F4` is the single behavioural divergence
   in that group; execution is how it was found, and whether a read of the sources
   could have found it is `not established`.

**`E2 P2-2` is the earliest-detectable finding in the corpus.** The reviewer states
the contract "is internally ambiguous (S6 against §3.4)" — an inconsistency between
two parts of one document, present before any E2 code existed. The campaign ran a
dedicated contract-verification agent (`r2_contract_verify`) and a contract-revision
agent (`r2_contract_revision`) before the builders. An ambiguity resolvable there
instead became a P2 that blocked E2 and required coordinator disposition.

## 5a. Question-by-question mapping for the decision-relevant subset

Added by successor session `078cd138-96be-46cb-ac48-3c353e2b357e` to answer
challenge item `FI01`. Subset: the **three blocking findings plus `E1 F8`**, the
four the peer named. Each is bound to the earliest revision or artifact state at
which the evidence existed, the minimal read set at that point, the observed
repair against any hypothesised avoided work, and later invalidation.

**Basis and its limits.** Campaign artifact mtimes, `git log -S` / `git show` on
`rescue-detached-20260911`, and the review files themselves, extracted by a
read-only child of this session (`agent-a4e93b45ae76466dc`'s sibling
`agent-a05b1b906a05a1f54`; child authorship named, judgments below are this
role's). **The contract is untracked** - it exists only as
`r2-plan/contract-v1.md` (sha256 `5d6b7da7...`, 356 lines, mtime 2026-10-22
20:01:28 -0600) and `r2-plan/contract-v2.md` (sha256 `05734636...`, 458 lines,
mtime 22:18:29). So contract availability is established by **file mtime, not by
a commit**, and an mtime is weaker evidence than a commit date. Times are -0600.

**Every time in the table is an observed artifact-timestamp proxy, not first
availability** (peer challenge, board **2793**). A commit date is when code was
recorded; code can exist uncommitted before it. A file mtime is a last-write
time, not a custody record, and can be altered or post-date an earlier copy. Both
proxies are therefore **upper bounds** on when the evidence first existed, so
every "earliest evidence state" below may be too late and every latency may be
too short. Treat the table as a qualified hypothesis. Nothing here establishes
that code did not exist before its commit, and no avoidable-work inference
follows from any latency figure in it.

| finding | earliest evidence state | detected | latency | minimal read set |
|---|---|---|---|---|
| `E2 P2-1` | **10-22 23:32:51**, commit `1918c582` adds `pub use comparison::check_comparison_bytes;` at `lib.rs:19` (contract side existed from 20:01:28) | `e2/review-v1.md` 23:47:55 | **15m 04s** | contract §5 lines 424-427; `proposal-evaluation/src/lib.rs:18-21`; `src/check.rs` (item absent). 3 files, ~10 lines |
| `E2 P2-2` | **10-22 20:01:28**, contract-v1 as written (S6 at v1:62, §3.4 at v1:232-236); no code required | `e2/review-v1.md` 23:47:55 | **3h 46m** | contract lines 64-68 and 273-283. **One document, two passages** |
| `P1 P2-1` | **10-23 08:25:11**, commit `382a7d9d` adds the comment and the `return None` guard at `upstream.rs:71-73` (structure rules `dc5a3530` 08:17:09; contract §4.2 step 5 + S6 from 10-22) | `review-notes.md` **08:44:24**; `review-v1.md` 09:02:42 | **19m 13s** to notes, 37m 31s to artifact | `upstream.rs:63-73` and `:516-522`; contract §4.2 step 5 (v2 ~374-376) and S6 (64-68); `src/structure.rs`. ~4 files |
| `E1 F8` | **10-22 20:01:28**, contract-v1 as written (§2.1 "optional `interval`" at v1:121; S4 "`null`, never an absent key"; AC2's nullable list) | `e1/review-v1.md` 22:52:12 | **2h 50m** | contract §2.1, S4, AC2's list. **One document, three passages** |

**The two long-latency findings each passed through a dedicated contract pass
that did not catch them.** This is the subset's most decision-relevant result and
it is not the one an "earlier review is better" reading predicts.

- `r2_contract_verify` read **contract-v1 in full** at 20:05:09-20:12:12 and
  returned 7 P2 findings and no P1. Its output
  (`r2-verify/verification-v1.md`, sha256 `d5aee39b...`, 166 lines) contains no
  statement of the S6-versus-§3.4 ambiguity and none of the `interval` /
  §2.1-versus-S4 inconsistency - searched for "S6" with "§3.4", "ambig", and the
  `interval` wording.
- `r2_contract_revision` produced contract-v2 at 22:18:29; `revision-log-v2.md`
  records neither issue.
- `r2_tier2_fable` read **contract-v2 in full (458 lines)** at 22:22:48-22:30:58,
  quoted the exact `check::` path at `r2-tier2/review-v1.md:137`, and concluded
  "E1 → E2 → P1 interfaces in §5 are consistent with §2-§4".

So for `E2 P2-2` and `E1 F8` the evidence was document-only and complete **3h 46m
and 2h 50m before detection**, and in that window three separate contract passes
read the document and recorded neither. For `E2 P2-1` the Fable pass could not
have caught it: it ran at 22:30:58 and the code half is not recorded before
23:32:51.

**What that does and does not support.** Readability establishes that the
evidence was *sufficient*; it does **not** establish that a reviewer at that
point would have found it, and here n = 3 prior passes and 0 detections on the
two document-only findings.

**A conclusion drawn here on 2026-10-24 is WITHDRAWN**, on the peer's challenge
at board **2793**. The withdrawn claim: that because review followed the code
within 15 and 19-37 minutes for `E2 P2-1` and `P1 P2-1`, there is little latency
for an earlier-review intervention to remove. Error class: treating an
**upper-bound** timestamp proxy as the exact time of first availability, and then
drawing a conclusion in the direction the proxy's error does not support. A
commit date is when the code was **recorded**, not when it first existed, so true
availability is at or before it and true latency is at or **above** the measured
figure. 15m 04s and 19m 13s are therefore **lower bounds**, and a lower bound
cannot support "the latency is small". Consequence had it stood: the plan's
candidate 1 would have been qualified downward on two of four findings using a
quantity that runs the other way. The check that would have caught it: ask which
direction a proxy's error runs, and whether the conclusion survives its worst
case.

**What survives.** The two document-only findings' latencies - 3h 46m and
2h 50m - are differences between two artifact timestamps and are likewise
proxies, but the result that rests on them does not depend on their size: three
dedicated contract passes read that document before detection and recorded
neither finding. That is a count of passes, not a duration. n = 4 findings and
n = 3 prior passes, this session's judgment, detected set only.

**Observed repair versus hypothesised avoided work.**

| finding | observed repair | avoided work |
|---|---|---|
| `E2 P2-1` | `d6022564` (HEAD-side `fe6a634a`) 23:54:23, +4 in `check.rs` and a new 41-line `tests/api_path.rs`; **~7m after the review** | **None established.** §8a: the consumer already used the root path and no P1 breakage is observed |
| `E2 P2-2` | fixed by `r2_E2_fix` (dispatched with P2-1, P3-1, P3-3, P3-4) | `not established`; no counterfactual model |
| `P1 P2-1` | `404b4992` (HEAD-side `c96447dc`) 08:47:23, `src/upstream.rs` +33/-8 and `tests/upstream.rs` +89; then `r2_P1_recheck` 0.591 Mwt | observed associated recheck cost only - see §7 as corrected under `FI03` |
| `E1 F8` | **none. Left open by `r2_E1_fix`, and the erratum it recommended was never written** - `contract-v2-errata-1`, `-E1-1` and the errata log contain nothing on `interval`, "nullable" or "required key"; errata-E1-1 edits §2.1 only for `implementation_authorized` | **The predicted cost is not observed and the recommended mitigation was not applied.** The non-occurrence therefore cannot be credited to the recommendation |

**Later invalidation.**

- `E2 P2-1`: not invalidated. The re-export landed and `lib.rs:18-21` is unchanged
  at HEAD.
- `E2 P2-2`: no record of invalidation; the contract was not re-frozen after it.
- `P1 P2-1`: the conclusion stands, **its citation has drifted.** At HEAD the guard
  sits at `upstream.rs:75` and the comment has been rewritten to "A wrong shape is
  *not* reported here: ... goes through `resolve_nested`". The guard statement
  itself is unchanged. Anything that binds this finding by `upstream.rs:71-73`
  will now resolve to the wrong lines.
- `E1 F8`: not invalidated, and still live. §2.1's "optional `interval`" survives
  unchanged at contract-v2 line 145 with no erratum against it, so the condition
  F8 described holds at the contract's final recorded state.

## 6. Recurrence: one defect class, three findings, two slices

The diagnostic-location class is the most repeated in the corpus:

- `E1 F7` — a typed-parse failure reports `at: ""` and only the first serde error.
- `E2 P3-1` — `pointer_unresolved` under `--rederive` has `at = ""`.
- `E2 P3-3` — schema errors from serde have `at = ""`.

Three findings, one cause, found by two independent reviewers in two slices. A
diagnostic-location convention settled in the contract after `E1 F7` would have
pre-empted both E2 findings. `P1` deviation 6 ("the contract names no `at`") was
accepted there, which is the same gap reaching a third slice as an accepted
deviation rather than a finding.

Two further cross-slice carries, which cost re-investigation rather than repair:

- **`E1 F2` (duplicate JSON keys resolve last-wins) was investigated three times.**
  E1 probed it (#205, #206, #221); E2 re-probed it ("duplicate keys in a finding and
  in a contract → rejected"); P1's R-A recorded it independently as observation O12
  ("the last value wins, the same as Python `json`"). Whether `proposal_packets.py`
  refuses duplicate keys is `not established` in both E1 and P1 — the sub-question
  survived the whole campaign unclosed.
- **`E1 F3` (first matching selection element decides the profile) was carried, not
  closed.** E2 lists it as outside its domain: "E1 F3 first-match basis resolution
  (still open; not relied on)".

**One false linkage declined, stated so no one else draws it.** `E1 F3` concerns
`check.rs:209`'s `find`, and this role's predecessor separately withdrew a
"first-matching, not last-wins" rule in `builder_ramp.py` at handoff-v2 correction 1.
These are different artifacts with a coincidence of shape and no shared defect.
They are not evidence for each other.

## 7. Question 4: where later changes move the early conclusion

- **`P1 P2-1` is resolved, not merely re-labelled.** Fixed across `e043ce21..63679a15`
  (commit `404b4992`), and `r2_P1_recheck` re-ran R-A's 133-line probe set: F2d, F2e
  and F2f moved MISS → OK, 23/23 of the dedicated P2-1 cases hit their exact
  `(code, at)`, and every other line of the 133 is byte-identical to the before-run.
  This is the one finding in the corpus with an **observed associated recheck
  cost**: one fix commit plus one full re-check agent, `r2_P1_recheck` at
  0.591 Mwt. **That 0.591 is not an established avoidable cost** (challenge item
  `FI03`; an earlier draft called it "a measurable late-detection cost"). The
  recheck re-ran all 133 probe lines and covered the slice's integrated repairs,
  not `P2-1` alone - the next bullet records it catching a `P3-2` side effect -
  and earlier detection would still have required some validation. The avoidable
  share and any net saving are `not established`; establishing either needs a
  counterfactual task and validation model this inventory does not have.
- **`P1 P3-2`'s fix changed behaviour beyond the finding, and the recheck caught it.**
  The remedy made limitation matching stricter than the contract text: an authored
  `strategic_basis_unverified — handoff not parsed` is now `limitation_missing`,
  where "before the fix it was accepted". The recheck reasoned it is not a finding —
  it fails closed, the message states the rule, and no real `portfolio-decision-v1`
  record exists to break, verified by a grep of `*.json` across the main checkout
  excluding `target` and worktrees. Recorded here because it is a case where later
  change altered the early conclusion's surroundings without invalidating it.
- **`E1 F4` generalised rather than being fixed.** `E2 P3-3` states: "E1 F4 located
  out-of-range numbers for estimate sets. The new records do not get the same
  treatment." The early conclusion stands and widened.
- **`E1 F8` predicted a downstream cost that is `not established`.** It warned that
  "E2 and P1 fixture authors reading §2.1 literally will omit the key and get a
  `schema` error". Neither E2's nor P1's review records having hit it. Whether the
  recommendation prevented it or it never bit is `not established`; absent a
  completeness basis over both slices' working records, this is not an absence claim.

## 8. Remediation disposition: what was actually repaired

Established from the four remediation packets and result JSONs, each read in full.
Remediation was dispatched **per finding**, not per review, and the dispatch was a
coordinator choice with explicit user approval recorded in two packets ("User
approved: fix these four, then integrate"; "User approved: add these tests,
re-check, integrate").

| agent | dispatched | reported fixed | left open by its own record |
|---|---|---|---|
| `r2_E1_fix` | F1, F2, F4 (+ mutant M15) | all | **F3, F5, F6, F7, F8** |
| `r2_E2_fix` | P2-1, P2-2, P3-1, P3-3, P3-4 | all 5 | **P3-2, P3-5, P3-6**, plus a `derive`-parameters erratum routed to the coordinator |
| `r2_P1_fix` | P2-1, P3-1, P3-2 | all 3 | — (P3-1 closed as a *declared limitation* plus a pinning test, not a behaviour change) |
| `r2_P1_tests` | P3-3, P3-4, P3-5, and an unnumbered "read-guard deadline" item | all 4 | — |

**14 of 22 findings were remediated; 8 remain open, and all 8 are P3.** All three
P2 findings were fixed. P1 is the only slice remediated completely (6 of 6);
E1 is the least (3 of 8).

**One inversion worth recording.** The `at: ""` diagnostic-location class of §6 was
fixed at its two *downstream* instances and left open at its *upstream* one:
`E2 P3-1` and `E2 P3-3` were both dispatched and fixed by `r2_E2_fix`, while
`E1 F7` — the same cause, found first — is in `r2_E1_fix`'s left-open set. The
campaign repaired the symptoms in the slice that found them twice and not the
instance in the slice that found them first.

**Remediation ran on a different model class than review.** All 6 review agents
recorded `model_requested` "opus, effort high" with observed `claude-opus-5-5`; all
4 remediation agents recorded "sonnet, effort high" with observed
`claude-sonnet-5-5`. So the 4.000 Mwt review cohort and the 2.482 Mwt remediation
cohort are not the same instrument, and their weighted costs should not be pooled
as though they were. Observed **effort** is `not established` for all 10:
`model_requested` says "effort high", but `recorded_effort` is `null` in every one
of the 12 context-check files.

**One process detail that bears on question 1.** `r2_P1_fix` was dispatched against
`p1-portfolio/review-notes.md`, the predecessor reviewer's staged notes, **not**
against `review-v1.md`; its `reference_sha256` lists the notes file and no review
file. Remediation therefore began before the review artifact existed, concurrent
with `r2_P1_review_2` completing that review on a frozen archive. The campaign thus
already demonstrates that a finding can be acted on at the point the *notes* are
stable, which is earlier than the point the review artifact is stable.

## 8a. Question 3 for `E2 P2-1`: the predicted cost is not observed in the landed code

`E2 P2-1` was graded P2 and formed half of E2's `blocked` assessment. Its stated
failure scenario was "P1 is written to its contract and does not compile."

**No breakage by this path is observed in the inspected landed code.** An earlier
draft said "it did not happen, and P1 was never broken by it". That is a claim
about the slice's whole build history, and landed source alone cannot carry it
(challenge item `FI04`). What is observed: the P1 crate's only two mentions of the
symbol use the crate-root path, not the contract's `check::` spelling:

- `rust/crates/portfolio-decision/src/upstream.rs:22` —
  `use proposal_evaluation::{check_comparison_bytes, check_estimate_set_bytes};`
- `rust/crates/portfolio-decision/src/upstream.rs:432` — an unqualified call inside
  `fn comparison`, resolved through that root import, passing `rederive = true`.

`proposal_evaluation::check::` appears nowhere in the P1 crate; the only `check::`
hits are P1's own internal module (`src/lib.rs:37`, `src/cli.rs:12`). The `E0425`
the review cites came from a purpose-built compile probe, not from P1's build, and
P1's gates passed throughout.

So the finding was **real as a contract-versus-code divergence and precautionary as
a severity grade**: contract-v2 §5 named a path the code did not export, but the
consumer had already been written to the path that did exist. `r2_E2_fix` commit
`d6022564` has since added the `check::` re-export and a test pinning the contract
path, so the divergence is closed on the upstream side while P1 still calls through
the root. Basis: the main checkout; worktree `agent-a8b36f8b30274b385` no longer
exists, so this is established from the landed code, not from the worktree the
review examined.

**Build-record basis, added under `FI04`.** The P1 slice's recorded gates are, at
`e043ce21` (`p1-portfolio/result-v1.json`): `fmt --check` 0, `clippy -D warnings`
0, `cargo test` 51/0, `proposal-evaluation` 96/0, `decision-readiness` 128/0;
plus the reviewer's own run (51/0, `review-notes.md:10-12`), the coordinator
check (51/0), `result-v5-fix.json` 54/0, `result-v6-tests.json` 57/0, and the
recheck (`recheck-v1.md:111-117`) 57/0, 96/0, 128/0. **One recorded P1 failure
exists and is not this path**: `recheck-v1.md:118-119` reports a first DR run of
8 passed / 14 failed caused by the reviewer's crates-only copy lacking
`skills/evidence-battery`, resolved to 128/0 after the directory was added.

**What that record does not cover.** One gate run per commit, no raw gate logs
for the R2 P1 slice, intermediate per-unit results only as handoff prose, no
logged test counts for unit 1 (`b89cf3e5`), and the agent worktree's build output
deleted (~44 GB, recorded in `assignments.json` events). Window: 2026-10-23
08:07-09:48. So within that record no failing P1 build attributable to the
`check::` path appears; whether P1 ever failed to compile on this path at an
unrecorded intermediate state is **`not established`**.

## 8b. Context readings across the cohort

12 context-check files cover the 10 agents (37 context-check files exist across the
three directories; 12 match the review/recheck/fix naming). **Exactly 2 of the 12
are above the 150,000 trigger**, and they are precisely the two that generated
handoffs, both belonging to the two `handoff_ready` reviewers:

| agent | readings | above trigger |
|---|---|---|
| `r2_E1_review` | 140,834 | no (+9,166 headroom) |
| `r2_E2_review` | **158,847** (`fresh`) | **yes**, −8,847 |
| `r2_E2_review_2` | 47,199 → 99,753 | no |
| `r2_P1_review` | 130,321 → **156,071** (`fresh`) | **yes**, −6,071 |
| `r2_P1_review_2` | 53,313 → 127,481 → 133,026 | no |
| `r2_P1_recheck` | 101,362 | no |
| `r2_E1_fix` | 114,694 | no |
| `r2_E2_fix` | 136,091 | no |
| `r2_P1_fix` | 93,872 — **prose only, no backing context-check file** | no |
| `r2_P1_tests` | 75,286 | no |

Both figures previously held independently (158,847 and 156,071) reproduce exactly,
including their `fresh` recommendation and negative headroom. `harness_usage` exists
for only 3 of 10 agents (`r2_E2_review` 175,000 tokens / 32 tool uses;
`r2_E2_review_2` 124,225 / 33; `r2_P1_review` 164,869 / 37); for the other 7 it is
`not established`.

**Two telemetry gaps, recorded because the peer's pilot plans to sample this.**
`r2_P1_fix`'s published 93,872 has no context-check JSON behind it — the number
exists only as prose in its result. And
`p1-portfolio/context-check-tests-predispatch.json` records `input_tokens: 0` with
`recorded_model: "<synthetic>"`, because the API call it was meant to measure failed
(`EAI_AGAIN`) and logged no usage. A sampling design that reads these files must
treat a 0 reading as `unavailable`, not as a low reading.

## 9. What this inventory does not establish

- It does not re-grade any review's severity labels.
- It does not establish defect density, or that any class is absent.
- It does not certify detector recall for this role's own ramp instrument, which the
  peer's 2776 verification also excluded and which `builder_ramp.py` structurally
  cannot see: an undetected write produces no row to audit.
- It makes no claim about the review that should have happened, only about the
  review that did.
