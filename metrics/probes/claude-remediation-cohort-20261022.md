# Remediation cohort, 2026-10-22: closing outside-review findings

**Registered before either builder's first write.** Labelled **not R2**. This is
an observational cohort, instrumented identically to R2 under amendment 2's
cohort rule, and it is not scored against B1–B6 as an R2 result.

## What this cohort is

Two builder slices repair the five findings an outside reviewer returned against
R1's S3 and the P1 port. Both are frozen at base `624ec60e`, both are Sonnet 5.5
high in isolated worktrees, both run under `skills/manual-orchestration`
unchanged.

- **fix-a** — `rust/crates/context-pressure/**`. EXT-001 (P1): replace the
  `format!("{:e}")`-derived number formatting with `ryu-js =1.0.3`, because Rust
  and ECMAScript break ties between equally short round-trip candidates
  differently. EXT-005 (P3): give the golden suite a guard independent of the
  fixture's own counts. Plus correcting the `lib.rs:12` parity claim to state its
  evidence domain.
- **fix-b** — `rust/crates/decision-readiness/**` and
  `skills/manual-orchestration/scripts/integration_readiness.py`. EXT-002 and
  EXT-004 (P2, provenance), EXT-003 (P2, instrument).

## Why it is worth instrumenting

The campaign has no measurement of **remediation** work, only of greenfield
slices. Three properties make this cohort informative:

1. **The defect targets came from outside the orchestration.** R1 amendment 2
   established that in-orchestration review at these tiers finds P3-class issues
   reliably and provenance- and contract-class issues unreliably. This cohort
   asks the next question: given a finding it did not find, can the orchestration
   close it, and at what cost per finding.
2. **Both packets require reproduce-before / reject-after evidence per finding.**
   That is a stronger evidence requirement than any R1 or P1 packet carried. If
   it holds, it is a candidate treatment change for R2's successor, not for R2.
3. **fix-a's defect is a formatting-tie class, not an input.** Closing it
   requires a differential test outside the corpus, which is the exact
   methodological gap the review exposed.

## Measured, per builder

Context at first code write (B1-comparable, adjudicated by reading the matched
command, never by detector); pre-write read calls (B6-comparable); weighted
tokens `uncached + cached/10 + 5*output`; added Rust lines and Mwt/krs; context
boundaries declared and whether a successor continued (outcome A); gate results;
and, as a cohort-specific quantity, **findings closed per builder with
reproduce-before evidence present**.

## Pre-registered limits

- n = 2 builders, one slice each. Every figure carries n.
- Remediation against a named defect is **not** comparable to greenfield build
  cost. No Mwt/krs figure from this cohort may be pooled with R1's or P1's.
- Added Rust lines will be small; `krs` denominators under ~0.3 make the ratio
  unstable, and that will be stated rather than smoothed.
- Quality here is not reviewer-relative in the usual way: the findings were set
  by an outside reader before the work began. Whether the repair is **correct**
  still needs a reader who is not the author, and that is `not established`
  until one runs.
- This cohort shares a branch with R2's subject. Measurement commits are held to
  my own `metrics/**` paths.

## Status at registration

Both packets dispatched; no result file exists for either. Every quantity above
is `not established`. P1-M-001 was applied to both packets before dispatch and
passed, 15 of 15 read-set paths present at base. R2-M-002 was raised against
fix-b's write scope and reported at chatboard sequence 2494; its resolution is
pending with the coordinator.

---

# Score, 2026-10-23

Both builders complete. **5 of 5 findings closed**, each with reproduce-before /
reject-after evidence in the run record. n = 2 builders, 5 findings.

## What I verified myself rather than reading from the result files

- **fix-a's whole suite, re-run independently.** `git archive` of head
  `00a77fe3` into a clean scratch tree with a fresh `CARGO_TARGET_DIR`: 35 tests
  pass, 0 fail (lib 4, controller 8, golden 11, policy_profile 5, projector 7),
  matching the builder's claimed counts exactly. The builder reported a
  stale-binary incident in its shared target dir; a clean extract and fresh
  target dir cannot have that failure mode, so this run is independent of it.
- **EXT-001 closed at the value that failed.** The fixture carries
  `["430c6bf526340002", "1000000000000000.2"]` and
  `["4305edd45e85c45a", "771558860699787.2"]`. I decoded both bit patterns —
  they are the two values from the review — and ran `node` myself: it prints the
  same two strings. The Rust test over all 25,027 cases passes in my re-run.
- **Both numstats**, recomputed with `git diff --numstat` from base.
- **R2-M-002 compliance.** `git diff --name-only 624ec60e 2161afd7` over
  `model.rs`, `validate.rs`, `digest.rs`, `evaluate.rs` is empty. The coordinator
  narrowed fix-b's scope after the report and the narrowing held: none of the
  four files in R2's read sets was touched.
- **fix-b's Rust gates cover head.** They were run at `df6f7334`, not at head.
  The only files changed between `df6f7334` and `2161afd7` are the two Python
  ones, so no Rust changed after the gates ran.

**Not independently reproduced:** fix-a's mutation runs, fix-b's before-state
executions (artifacts read, not re-executed), and the 267-of-25,027 before-count.

## Cost

| | weighted | added Rust | Mwt/krs |
|---|---|---|---|
| fix-a | 1.614 Mwt | 138 | 11.7 |
| fix-b | 1.718 Mwt | 214 | 8.0 |
| both builders | 3.332 Mwt | 352 | 9.5 |
| R2 planner (no deliverable yet) | 1.685 Mwt | — | — |

**The Mwt/krs column is the wrong unit and is printed only to retire it.** As
pre-registered, remediation cost per added line is not comparable with R1's
code-slice 1.26 and 1.78. The unit that carries meaning here is **0.67 Mwt per
finding closed**, n = 5.

The planner spent **1.685 Mwt and produced no deliverable** — the contract, the
three builder packets and `freeze-check.json` are all still unwritten. It wrote
design notes and a handoff instead. That figure belongs beside R1's 36.2%
orchestration share when R2 is scored.

## B1-comparable, with a caveat that matters

fix-a's first successful write to a repository `.rs` file is at tool call 22,
context **56,680**. An earlier `.rs` write at call 20 went to the scratchpad, at
55,729; the 951-token difference does not change any reading.

**Three write attempts — calls 16, 17 and 21 — were refused by the worktree
isolation guard** ("too complex to verify that it stays inside the worktree").
No write occurred on any of them. A detector keyed on the command text would
have scored the first code write at call 16 and 3,000 tokens early. This is the
same failure class amendment 3 corrected, caught here by reading each tool
*result* as well as each command.

Both figures sit below R1's corrected B1 range of 71,539–116,779 (n=6), as P1's
81,055 did. **This is not evidence for H1.** Remediation against a named defect
with a 4,049-byte packet is a different task from a greenfield slice; scope is
fully confounded with the treatment, n=2 across two different subjects, and
neither is an R2 builder. It is recorded so that R2's figures are not read as
the first time anything moved.

fix-b's peak context was 131,771 and fix-a's 102,910; neither reached the
150,000 handoff trigger, so this cohort contributes **no outcome-A instance**.

## The R2 planner declared a boundary; outcome A is incomplete

The planner (Opus 5.5, `a6d8a16f300c0a71d`) reached **175,123** — above the
150,000 trigger and 27,410 below the observed band floor — declared
`handoff_ready`, and wrote a handoff naming its state, its read inputs with
sha256s, and four next actions. **No successor has been launched.** Outcome A
requires a successor to continue; whether one does is `not established`. The
boundary declaration is worker-owned and well-formed, which is the part the
pilot measures as the precondition.

## The disclosed confound materialised, and was caught

The planner's handoff records: *"`metrics/probes/claude-builder-r2-prereg-20261022.md`
changed on disk after I read it. Re-read its amendments before writing the
criterion status. I read Amendments 1–3 only."*

That file changed because this measurement role committed `d6ed2270` while the
planner was reading. The prereg already carries the confound saying this role
should hold commits or use a separate branch. The cost here was one re-read
instruction passed to a successor, not a wrong result — the planner detected the
change itself. The amendment had to exist before the first builder write, so the
commit was the work; the interference is recorded rather than reconstructed
later.

## Residuals, reported and not fixed

- **EXT-002 is closed at the assessment check, not at the structural evaluation
  check.** With no profile supplied, a class-relabelled evaluation still reports
  `valid: true` — `fix-b/repro-after.txt` shows exactly this. The packet scoped
  the structural case to "if the profile is available", so this is as specified,
  but the narrower closure should be stated wherever EXT-002 is called closed.
- **fix-b weakened the new citation check for non-absolute paths** to keep an
  existing test passing. The skip is sound: `check_file_ref` already rejects a
  non-normalised-absolute path with `invalid_absolute_path`, which is why the
  existing test saw both errors. Verified by reading the rule; worth a reviewer's
  eye because it is a guard with a documented hole.
- **fix-a's own disclosure:** repository docs outside its write scope were not
  inspected for parity claims, so repository-wide freedom from the overclaimed
  parity language is `not established`.
- Scratch directories were left at `/home/bline/cbm-work/fix-a-mut` and
  `/home/bline/cbm-work/fix-a-target`.

## What this cohort does and does not show

It shows the orchestration can close findings it did not find: five of five,
including the P1, with reproduce-before evidence and a control that still
passes, at 0.67 Mwt each. It does not show the repairs are correct — an outside
reader has not read them. A tier-1 reviewer was running against fix-a at the
time of scoring; fix-b had none. Correctness is `not established`.
