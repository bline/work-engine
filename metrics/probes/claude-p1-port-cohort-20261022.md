# P1 context-pressure port: observational cohort, labelled not R2

Registered 2026-10-22T16:25Z, **while the builder was still running**. R2 stays
frozen; no R2 builder has been dispatched.

## Why this is not R2

The port-boundary scout (`~/.local/state/work-engine/r2-pressure-controller-20261022/scout/result-v1.json`)
returned **`does_not_meet`**. The restated criterion in prereg amendment 2 asks
each scored code slice for a read set of at least 2,000 existing lines, at least
1,000 added lines in one language, and at least three code-slice builders in
total. The P1 packet names **813 product lines plus 251 skill lines = 1,064**, and
plans **one** builder. Only the added-lines leg would likely pass
(1,400–2,100 Rust estimated).

The scout also named the only slicing that would reach 2,000 read lines — Rust
lifecycle integration — and rejected it itself: no Rust owner contract for a
pressure consumer exists, so it is design rather than a port, and it would touch
another session's uncommitted `lifecycle-*` edits and the shared `Cargo.toml`.
That is the bend the registered ruling forbids, and the scout declined it without
being asked to.

So the ruling applies as written: run the port instrumented identically, record
it as an observational cohort labelled **not R2**, leave R2 frozen.

## Subject and builder

`token-usage-pressure-projection.mjs` (112) + `context-pressure-controller.mjs`
(257) → `rust/crates/context-pressure`, evidence producer only, with byte-identical
sha256 revisions proven by JS-generated golden vectors. Builder
`ad67ff1b4318eee35`, Sonnet 5.5 high, own worktree, base `6f4d08a1`, trigger
150,000.

## Measured so far (as of 16:04:42Z, not final)

| quantity | value |
| --- | --- |
| first manifest write (`Cargo.toml`) | **72,427** at call 28, 24 read calls before |
| first Rust source write (`src/error.rs`) | **81,055** at call 33, 26 read calls before |
| API calls | 66 |
| max context | 110,815 |
| weighted tokens | 1.249 Mwt |

Against the **corrected** baselines (prereg amendment 3): context at first Rust
write is **20,994 below** B1's median of 102,049, while pre-write read calls are
**10.5 above** B6's median of 15.5.

Under the disambiguation rule registered before any of this ran, that pattern is
**earlier committing, not leaner reading** — and with n=1 and a read set a third
the size of B1's, it is not evidence about the instruction change at all. It is
the confound amendment 2 named when it restated the criterion, showing up exactly
where it was predicted to.

## What this cohort can and cannot answer

Can: whether a builder given an enumerated read set reads only it (`read_set_actual`
against the 813 named); whether the 150,000 trigger fires on a single-slice port
and the turnaround if it does; the first in-repo JS→Rust expansion ratio, which R1
could not give because R1 was greenfield; Mwt/krs for a port as a new comparator.

Cannot: H1 as a test of the instruction change; H2 at n=1; H3 with one slice; H4
against B4, since a port and a greenfield crate are not comparable denominators;
H5 without a multi-slice campaign.

## Coordinator side

| agent | model | weighted | max ctx | duration | outcome |
| --- | --- | --- | --- | --- | --- |
| scout `a1fb23e0beb53db59` | Opus 5.5 | 0.636 Mwt | 93,089 | 198 s | `does_not_meet` |
| alignment `acff6f62c6078c24b` | Opus 5.5 | 1.070 Mwt | 121,864 | 260 s | `aligned_with_changes` |

**1.707 Mwt went on gating before a line of code existed**, both read-only, and
both changed the plan: the first stopped a campaign from being mislabelled, the
second established that the accepted lifecycle design still wants this component
and said what has to change about how it connects to Rust.

The successor root reached **154,879** by 15:59Z — across the 150,000 trigger
**73 minutes** after the transfer that reset it to 51,954. That is consistent with
the measured 1.4-coordination-call break-even rather than against it: a root
accumulates context fast because almost all of it is re-sent carrying cost.

## Instrument note

No live poller was run. R1 defect R1-M-002 came from a watcher that derived
idleness from transcript mtime; a single-builder cohort is fully recoverable post
hoc, so the instrument was omitted rather than repeated.

---

# Score, 2026-10-22T18:30Z

Complete through the builder line and the tier-1 review; **not integrated**.
Six agents, 14:47:59Z–18:06:51Z, six commits, head `2e0a80eb` on
`worktree-agent-ad67ff1b4318eee35`. 2,588 added Rust lines, all additions, plus a
698-line JS generator and a 2.7 MB single-line fixture.

| agent | role | model | weighted | max ctx |
| --- | --- | --- | --- | --- |
| `a1fb23e0beb53db59` | port-boundary scout | Opus 5.5 | 0.636 Mwt | 93,089 |
| `acff6f62c6078c24b` | design alignment | Opus 5.5 | 1.070 Mwt | 121,864 |
| `ad67ff1b4318eee35` | builder, units 1–3 | Sonnet 5.5 | 2.478 Mwt | 148,507 |
| `acae03b0f8c240eec` | successor, unit 4 | Sonnet 5.5 | 1.331 Mwt | 141,015 |
| `a118ffaef45ac2ea6` | tier-1 review | Opus 5.5 | 1.480 Mwt | 167,078 |
| `aef469db429d6513a` | P3 fix pass | Sonnet 5.5 | 0.430 Mwt | 72,964 |

## Outcome A, fifth instance

The predecessor declared at **143,399** — below the 150,000 trigger, by the
packet's "or the next unit would cross it" clause — committed units 1–3, wrote
`handoff-1.md`, and returned `handoff_ready`. The successor started **31 s**
later, inside R1's 27.9–58.4 s range, and completed unit 4 (the golden-vector
parity proof) to passing gates.

That is **5 instances of outcome A in the Claude arm, 3 of them declared below
the trigger**, against **0 in 120 Codex pilot records**. One qualification: the
successor ran in the predecessor's worktree rather than an isolated one, so this
instance did not test worktree re-entry.

## Economics

| | value |
| --- | --- |
| subagents | 7.425 Mwt |
| orchestrator | 3.704 Mwt |
| total | 11.129 Mwt |
| orchestration share | **33.3%** |
| Mwt/krs, subagents only | 2.869 |
| Mwt/krs, with orchestration | 4.300 |
| **slice cohort Mwt/krs** | **2.209** |

The slice-cohort figure is built the same way as R1's B4 rows — builder,
successor, review and fix pass over gross added Rust lines — and comes out
**45.6% above** the mean of R1's two greenfield slices (1.255 and 1.778). n=1 per
cell, and the denominators are not comparable: the port carries a parity
obligation that produces no Rust at all (698 JS generator lines, a 2.7 MB
fixture, 10,168 cases). That non-comparability is why this was registered
observational rather than as R2.

Campaign-level the port looks *better* than R1 (2.869 against 3.611 subagents-only)
for one identifiable reason: R1 spent 11.311 Mwt, 30.7% of subagent cost, forming
its contract. P1's equivalent was the scout plus the alignment check — **1.707 Mwt,
23.0%**, both read-only, and both changed the plan.

Orchestration share 33.3% against B5's 36.2% is **not** a comparison: B5 is
post-integration and P1 is not integrated. R1's pre-integration share was 33.9%.

## Quality

Tier-1 verdict **ready**, accepted on first candidate. **Four findings, all P3,
no P1 and no P2.** The reviewer reproduced all four gates and ran **12 targeted
mutations** against the parity harness, which failed under each — that is the
check that distinguishes a parity proof from a parity claim.

F1 (the golden generator's verdicts depended on the shell time zone near the
ECMAScript `Date` range bounds) and F3 (a wrong comment) were fixed; F2 and F4
were deferred to the wiring step by the reviewer. The fix pass changed no Rust
behaviour, and when it found that the wording needing correction sat in a
coordinator artifact outside its write scope, it reported rather than edited —
and the coordinator wrote the erratum. The correspondence discipline worked
without anyone adjudicating it.

R1's comparator: 19 findings, 0 P1, 2 of 3 slices first-candidate. The pure-Codex
baseline: mean 6.0 findings over 11 slices, 0 of 14 first-candidate — but it
records counts without severities, so no cross-arm severity comparison is
supportable.

## First measured JS → Rust expansion ratio

| | lines | ratio |
| --- | --- | --- |
| JS subject source | 369 | — |
| Rust source | 1,527 | **4.14×** |
| JS subject + its test | 499 | — |
| Rust source + tests | 2,588 | **5.19×** |
| all new code incl. generator | 3,286 | 6.59× |

The scout estimated 2.0–3.0×, total Rust 1,400–2,100, and marked it unverified.
Actual 2,588 — **23% above the top of its range**. R1 was greenfield and could
give no ratio, so this is the first in-repo measurement. A port's Rust output is
about 4× its JS source, and the parity obligation adds material that produces no
Rust lines at all.

## Context discipline

**Zero compactions.** Completeness basis: both marker forms, `compactMetadata`
and `compact_boundary`, over all six subagent transcripts in the orchestrator's
own `subagents/` directory plus the orchestrator transcript — 7 files, 7 counts of
zero. Subagent transcripts are written only in that directory, so the domain is
complete for this campaign.

Eight context checks: six by the two builders, one by the reviewer, one by the fix
pass. Context at first source write: predecessor **81,055** (`src/error.rs`, 26
read calls before, after a `Cargo.toml` write at 72,427); successor **83,177**
(`generate.mjs`, 9 read calls; its first `.rs` write is at 106,127); fix pass
**60,744** (9 read calls). All three sit below the corrected B1 median of 102,049
— and none of it is evidence about the instruction change, with a 813-line named
read set, n=3, and two of the three entering an existing crate rather than
starting one.

Two observations worth carrying forward:

- The **tier-1 reviewer read 157,436 and got `fresh`, and continued to
  completion** (peak 167,078), recording the reading and its reason: a read-only
  review making no claims. First observed case of a worker continuing against a
  `fresh` recommendation.
- The **orchestrator reached 190,668 without compacting** — 40,668 above the
  trigger, 11,865 below the lowest auto-compaction observed on this machine
  (202,533). Another R3 data point.

## Defect found

**P1-M-001 (P2), coordinator.** The packet named 813 product read lines, but
**176 of them did not exist at the base commit**: `docs/rust-development.md` (158)
and `app-server/docs/context-lifecycle-transition-protocol.md` (18). The builder
read about 637. Self-reported by the builder line, and recorded as a coordinator
error in `packet-v2-successor.md`.

This matters beyond tidiness: the **named read set is the quantity R2's restated
criterion is measured against**, and here it was 22% smaller in fact than on
paper. For R2 that is the difference between meeting and missing the threshold.
**Remedy:** before an R2 packet is frozen, verify every read-set path exists at
the required base commit, and evaluate the criterion on the verified set.

## What this leaves

Answered: builders given an enumerated read set read approximately it and
declared their additions; the trigger produces a handoff on a single-slice port;
the expansion ratio; a port cost comparator where none existed.

Not answered: **H1 through H5 remain unscored.** R2 is still frozen and still
needs a subject with a *verified* read set of at least 2,000 existing lines, at
least 1,000 added lines in one language, and at least three code-slice builders.

## Integration (recorded after the score)

The score above says **not integrated**, which was true when it was written. The
coordinator then integrated the slice onto `rescue-detached-20260911` as
`e71c5b7b..65096374`. Verified from the measurement side on the integrated branch:
`cargo test --offline --manifest-path rust/crates/context-pressure/Cargo.toml`
gives **32 passed, 0 failed** across five binaries (lib 2, controller 8, golden 10,
policy_profile 5, projector 7).
