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
