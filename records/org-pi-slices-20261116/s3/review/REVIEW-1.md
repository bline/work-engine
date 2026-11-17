changes_required

# S3 independent review 1 of the R2-S candidate

Reviewer: Claude Fable 5.2 (`claude-fable-5-2`), separate session and context, same provider family as
the builder. Parent and acceptance authority: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`.
Subject: checkpoint `24597fef102fd7e98b5c172d6d55aa1cf4fa4d8b`, tree `bf615d59…`, read in the export
`s3/export-24597fef/rust` (`E`). Paths below are relative to `E/crates/` unless they start with `s1/`,
`s2/` or `s3/`.

No P1. Three P2, six P3. The gated path behaves as invariants 3 and 4 require. The verdict rests on
invariant 2: one member of the host description is documented by the subject itself as untrue (F-1), the
description is not derived from the configuration that selects what runs (F-2), and two absence
statements in the source are false as written (F-3). All three repairs are small and inside R2-S.

## Method and limits

- **Static reading only.** I ran no build, test or command. Shell execution was denied in this session,
  so `git diff e6a57e7c 24597fef` was **not run**. I read the export tree for the 14 files RESULT.md
  lists. That the patch touches only those files, and that `work-engine-organization` is unchanged, is
  `not established` by me; it rests on RESULT.md and the patch digest in `s3/ROOT-GATE.json`.
- **Executed facts come from records**, not from me: `s3/root-gate-1/*.stdout` and
  `s3/build/mutants/*/`. I matched every `test result` line in `root-gate-1`: 21 lines in
  `test-four.stdout` sum to 251 passed, 0 failed; default features 33; authority store 37; native 23 in
  each of `native-1..3`. These equal ROOT-GATE.json. `native-1.json` records cwd = the export.
- Read in full: `review-delivery/src/{gate,binding,lib}.rs`, `review-preparation/src/realization.rs`,
  `tests/gate.rs`, `tests/common/mod.rs`, DERIVATION.md, PACKET, RESULT, MATRIX, addendum, owner
  decision, S1 acceptance, `s1/build/MATRIX.md` rows. Read in part: `native.rs` (1–48, 60–259, 528–577,
  960–1381), `prepare.rs` (96–645), `tests/native_vertical.rs` (47–135, 211–242, 494–669),
  `tests/binding.rs` (300–374), `tests/preparation.rs` (382–421). Not read: `attempt.rs`, `wire.rs`,
  `tests/pure.rs`, the adapter beyond 16 grep hits, `s2/review/REVIEW.md`, `s2/design/PROPOSAL.md`.
- Board read and context check were not performed (no shell; no identity binding delivered):
  `usage_unavailable`. My spend is `not established`.

## 1. Invariants 1–6 at the subject

| # | Invariant | Finding | Basis |
|---|---|---|---|
| 1 | Closed `BlockerKind`, one kind per emitted blocker | Holds | Static: 9 variants, `ALL`, `of_id` (`review-preparation/src/prepare.rs:215-277`); every construction site sets the kind (`:529-626`); no id prefix is a prefix of another, and the kind is set at construction, never parsed. Executed: `every_emitted_blocker_has_exactly_the_kind…` passes (root gate). |
| 2 | Host-derived realization, truthful, not caller-supplied | **Partly.** Caller cannot supply or override the description: holds. "Derived from native configuration" and "truthful": not met (F-1, F-2) | `review-delivery/src/binding.rs:204-214`; `native.rs:224-247` |
| 3 | Refuse before spawn and before consumption; spawn 0 / consumer 0 | Holds on the gated path; the "no other route" wording is false (F-3) | Static: `evaluate(BeforeSpawn)` at `native.rs:1340` precedes `run_native` at `:1347`; fixture setup and the spawn counter are inside `run_native` (`:1099`, `:1126`); `evaluate(BeforeConsumption)` at `:1362` precedes the only `binding.consume` call at `:1369`. Executed: the frozen-world and re-check tests pass in 4 root-gate runs each (n = 4 runs, 1 world per case). |
| 4 | Discharge total, host-owned, never caller-supplied; realization checks never assumable | Holds | `gate.rs:41-53` is an exhaustive match with no wildcard, so a tenth kind is a compile error; `KnownIncompatibility` refuses by class whatever the kind (`:126-129`); `deliver_and_consume*` takes no discharge argument (`native.rs:1326-1339`). Subject to F-1 for what reaches the gate. |
| 5 | Second world, labelled; frozen world is the negative through the real host path | Holds | §4 below |
| 6 | Discriminating evidence | Adequate for the five named guards, with limits | §5 below |

Specific questions in the task:

- *Discharge host-derived and total, never caller-supplied*: yes. The seam can add blockers only.
- *Realization check blockers never assumable*: yes at the gate. Whether a mutation check blocker exists
  at all is decided upstream by the claim value the host chooses (F-1).
- *Caller cannot supply or override the description*: yes. A caller can still change what the
  description describes (F-2).
- *Refusal leaves spawn 0 / consumer 0*: yes, static and by record.

## 2. Findings

### F-1 (P2) The mutation claim: sound as a scoped host claim, not as written

This is the root's provisional ruling on builder deviation 1.

**Structure (static, high confidence).** `ReadOnlyClaimed` with no tools evaluates to `ClaimedCompatible`
(`review-preparation/src/realization.rs:139-143`), which emits no blocker (`prepare.rs:615`). The gap
between that claim and evidence is the unconditional `realization-admission:<condition>` blocker, whose
requirement names mutation ("admitted with evidence of the required capabilities, mutation, provider
family and confinement", `prepare.rs:582-588`). That kind is in the root's assumption set
(`gate.rs:50`) and appears in `AssumedDischarges::assumed()`. So "`mutation: none` is assumed" is
carried by a labelled assumed discharge of an assumable kind. This matches addendum item 2 and invariant
4. On this point the root's provisional position is right, and the builder's framing of an unavoidable
tension is too strong.

**Why the subject still does not meet the position.**

1. The subject says the claim is false. `native.rs:218-220`: "A labelled assumption, not a true
   statement". DERIVATION.md:28-32 lists it under "What is assumed, not true". `native_vertical.rs:528`
   repeats it. Invariant 2 requires a truthful description. If the host holds the member untrue, the
   truthful value in the existing vocabulary is `NotEstablished`, which emits
   `realization:<condition>:mutation` (Unestablished) and refuses. That blocker is the suite's own
   exemplar of a never-assumable check (`tests/gate.rs:103-126`; `native_vertical.rs:621-625`). As
   written, the only thing keeping it off the delivered path is a claim value its author disowns. That
   is an assumption placed upstream of the never-assumable rule.
2. The assumption is labelled in prose only. The delivered evaluation says `claimed_compatible` with
   basis "the descriptor claims read-only with no tools; the claim is not admission evidence". That is
   a truthful label for a claim. It does not say the claimant disbelieves it.
3. Before spawn nothing backs the claim (F-2).

**What the host can truthfully claim and does back (static).** No tool is granted, so the model has no
mutation route. The adapter creates the session with `noTools: 'all', tools: []` and in-memory session,
settings and auth (`review-delivery/adapter/review-delivery-adapter.mjs:79,90-94`; grep hits only, the
adapter was not read in full). `BoundAttempt::bind` refuses any allocated tool (`lib.rs:268-270`).
`verify` refuses any active or advertised tool (`lib.rs:331-348`) and runs before the pre-consumption
evaluation (`native.rs:1348-1362`). Not backed: writes by the child process itself, which is unconfined;
the description says so in `execution_confinement`. In the owner's vocabulary mutation is evaluated with
the tool list (`realization.rs:120-143`) and confinement is a separate dimension, so the pair
"read-only claimed, no tools" plus "unconfined" is a coherent, honest description.

**Recommended design inside R2-S (option 1).** Keep `ReadOnlyClaimed`; keep realization checks never
assumable. Change three things:

- Reword `native.rs:218-220`, DERIVATION.md:31-32 and `native_vertical.rs:528` to the scoped claim: what
  is claimed (no granted tool, hence no model mutation route), what backs it (construction; refusal at
  allocation and request), what is not established (process-level writes; no confinement), and that the
  evidence gap is the `realization-admission` blocker passed only by the labelled assumed discharge.
  Remove "not a true statement".
- Bind the description to what runs (F-2).
- Carry the assumed discharges into the success output (F-4).

**Option 2, not recommended.** The host states `NotEstablished` and the ruling is amended so that exactly
`realization:<condition>:mutation` with class Unestablished is assumable. The assumption then appears by
id in `assumed()`. Cost: it creates an assumable realization check, breaks "one kind per id family" or
changes `discharge`'s signature, and inverts the M3/M4 exemplar. It needs a root ruling change.

Confidence: high on the code facts; medium on the judgment between options.

### F-2 (P2) The description is a constant; caller-controlled fields select what runs

`realization_description(cfg)` binds all eight `NativeConfig` fields to `_` and returns
`runner_realization()` (`native.rs:224-247`). Its doc says "No `NativeConfig` field selects a provider,
a confinement, tools or continuity" (`:222-223`). By reading, that sentence is wrong:

- `node`, `adapter` and `sdk_root` are public fields (`:173-178`) and `run_native` spawns
  `cfg.node cfg.adapter --sdk cfg.sdk_root` (`:1104-1110`). They select the whole program.
- `Control::OrdinaryStream` leaves the ordinary SDK provider stream selected (`:73-74`; adapter `:160`).

Consequence (static; not executed): on the assumption world a caller who sets `cfg.adapter` to another
script passes the pre-spawn gate, and that program receives the contract in the start frame
(`:1053-1062`) under a description of a different program. Post-spawn checks read that program's own
report. Product code verifies no pin (`:176-177`). The test pin covers the SDK entry module and manifest
only (`tests/common/mod.rs:290-296`); I found no check of the adapter's bytes in `tests/common/mod.rs`
or in the parts of `native_vertical.rs` I read, and I did not search the rest of the crate, so "no
adapter pin exists" is established within that subset only.

Both guards that consult the description are tautologies: `RealizationDiffers` (`:1288`) compares two
results of the same constant function.

This is partly inherited (S1 unclaimed: "no exhaustive SDK audit", "SDK pin covers…"). It is newly
load-bearing because invariant 2 and the root's condition "a claim the host itself backs" now depend on
it.

Repair (small). Keep the exhaustive destructuring but use `adapter`: when
`cfg.adapter != NativeConfig::default_adapter()`, return a description with `mutation: NotEstablished`
and unstated provider, confinement and continuity. The existing comparison at `:1288` then refuses with
`RealizationDiffers` before spawn, spawn 0. Add one test and one mutant (ignore `cfg`). That also gives
`RealizationDiffers` a real trigger (§5). Path equality is name-level correspondence, not a pin; say so.
Do not key the description on `control`: the negative controls must still reach the child. Classify
`node`, `sdk_root` and `control` in the doc as presumed inputs and list the presumption under unclaimed.
Minimum alternative: correct the sentence at `:222-223` and add that unclaimed entry.

Confidence: high.

### F-3 (P3, wording must change) Two absence statements about spawn routes are false

`native.rs:14`: "The only route to a child is `deliver_and_consume`". `native.rs:1323-1324`: "no other
route to a spawn or a consumption in this crate". But `run_attempt` (`:542`) and `run_attempt_observed`
(`:553`) are public and spawn any caller-built `Command` (`:564-566`); `AttemptDriver` is a public trait
(`:305`); `NativeConfig::default_adapter()` is public (`:192`); the contract bytes are public through
`binding.frozen().contract()` (`binding.rs:359`, `lib.rs:168`). A caller can spawn the adapter and send
it the frozen world's contract without the gate. The integration tests already call `run_attempt` with
`/bin/sh` (`tests/native_vertical.rs:1017,1477,1699,1728`).

What does hold: the SDK-protocol runner and the binding's consumer are reachable only through the gate.
Basis: a grep for `run_native(` and `.consume(` over every `*.rs` under `work-engine-review-delivery`
(src and tests) finds one caller each, `native.rs:1347` and `:1369`. Separately, the consumer function
`work_engine_review_preparation::fixture::consume_fixture_result` is public under `fixture`
(`binding.rs:32-35`) and callable with no gate; unchanged from S1.

This is within the packet's stated remedy ("make `run_native` non-public"), so no behaviour change is
required. Narrow both sentences to "the only route that runs the SDK delivery protocol from a binding",
or make `run_attempt*` crate-private behind a test-support export.

### F-4 (P3) Label does not reach the success output

`Delivered` (`native.rs:1223-1228`) carries no `AssumedDischarges`. The pre-spawn value is used only for
the boundary check (`:1094`) and the pre-consumption `Ok` value is discarded (`:1362`). A reader of a
successful `Delivered` cannot see that it passed only by labelled assumptions. Addendum item 6 says the
discharge map is an output. Carry both evaluations in `Delivered`. Also rename the variable `cleared`
(`:1092`, `:1340`): `lib.rs:25` says nothing here clears a blocker.

### F-5 (P3) `AssumedDischarges` is not bound to a binding and is publicly mintable

`gate::assess(Boundary::BeforeSpawn, &[])` returns a pre-spawn value for an empty list (`gate.rs:108`);
mutant M1 used exactly that. Harmless while `run_native` is private. R2-A' must not treat this type as
proof that a given binding was evaluated.

### F-6 (P3) Owner-decision record contradicts itself on O11

`s2/OWNER-DECISION-S2.json:31` records `O11: "yes: operator 'build R2-S' (2026-11-16)"`. Lines 34-35
say "Implementation … remain unauthorized" and "chose design only … O11 not". `s3/build/PACKET.md:7`
cites O11 as the build authority. Which statement is current is `not established` from the record.
Correct it before the acceptance record cites it.

### F-7 (P3) RESULT.md deviation 4 understates the contract-byte change

Besides `kind` per blocker, the delivered bytes now carry a non-null `realization_evaluation`
(`binding.rs:209-213`): the host's claims are delivered to the session. Invariant 2 intends this; the
record should say it.

### F-8 (P3) Guards with no test or mutant

`GateRefusal::RealizationDiffers`, `PreparationChanged`, `Preparation(_)` (`native.rs:1288-1295`) and
`run_native`'s boundary check (`:1094-1098`). MATRIX names the first two only. Pre-spawn `Undischarged`
is never produced through the host path: before spawn only `KnownIncompatibility` is exercised;
`Undischarged` is exercised by the pure gate test and by the seam before consumption.

### F-9 (P3) Mutant evidence standing

Six mutants, n = 1 per row, applied by hand to the uncommitted worktree before checkpoint `24597fef`
(MATRIX header). Each mapped test fails in its record (I matched the `FAILED` lines in all six
`*.stdout`). M3 and M6 ran the `gate` test binary only (5 tests), so whether the native
`recheck-undischarged` case also fails under M3 is `not established`. Same class as S1's carried F-2.

## 3. Deviations 2–6

| # | Deviation | Assessment |
|---|---|---|
| 2 | Free-text descriptor values | Acceptable. `against_allowed` compares strings (`realization.rs:88-96`); the values are self-describing and extend no schema. Limit: the frozen world's refusal needs only that the strings differ from `anthropic` / `guarded_sandbox`, and on a world without constraints no check reads them (`:189-203`). |
| 3 | Add-only `BetweenBoundaries` seam | Acceptable. Added blockers are appended and assessed by the same function (`native.rs:1296-1298`); every branch of `assess` that a blocker can reach is a refusal, so an addition cannot turn a refusal into a pass. It is public API under `fixture`; remove it when a real observation exists. |
| 4 | `Blocker.kind` in contract bytes | Acceptable. No other workspace crate depends on either crate: a grep of every `Cargo.toml` under `E` for both package names finds the two workspace entries and delivery's own dependency. Readers outside the Rust workspace were not searched. See F-7. |
| 5 | Join tests on the frozen world | Acceptable and useful. `tests/binding.rs:10` aliases `frozen_world as world`; the join is independent of the gate, the constraints-cleared case needs non-empty constraints (`:273-275`), and it shows the binding does not refuse on blockers. |
| 6 | New `DeliverError` variant | Acceptable under the pre-release policy; no workspace consumer (same grep). Consider `#[non_exhaustive]`, as on the sibling refusal enums. |

## 4. Second world and S1 coverage

- Derivation (static + record): the stored requirements equal the frozen requirements with
  `realizationConstraints` removed from every condition, with the removal asserted to remove something
  (`tests/native_vertical.rs:499-510`); compiled by `derive(_, FixedSliceReviewV1)` with no unresolved
  item and exactly one candidate (`tests/common/mod.rs:42-48`). The test passes in the root gate.
  "Unchanged compiler" rests on RESULT.md (see limits).
- Label: DERIVATION.md states the standing. Its mutation bullet needs the F-1 rewording.
- Negative: the frozen world goes through `deliver_and_consume` with the real configuration and is
  refused `BeforeSpawn` with four `KnownIncompatibility` ids, `run` none, spawn 0, consumer 0, empty
  scratch root, episode state unchanged (`tests/native_vertical.rs:535-577`). Passed in 4 recorded runs.
- Coverage lost: a positive native run no longer delivers non-empty constraint contexts
  (`:517-522`). None of the 13 rows in `s1/build/MATRIX.md` depends on them: the native rows are M03,
  M06, M08–M12, and the preparation rows stay on the frozen world. S1's carried F-9 (constraint values
  not asserted against the view) now has no native positive carrying constraints at all.
- S1 mutants were not re-run, as the builder states; that each is still killed at `24597fef` is
  `not established`. By reading, the M06 and M09 guards are still present (`native.rs:1053-1062`,
  `:1265-1269`).

## 5. Matrix and mutant adequacy

The five guards the packet names each have a mapped test and a recorded failing mutant, six mutants in
all because the builder split the third guard in two: pre-spawn (M1), pre-consume (M2), undischarged
kind (M3) and unknown or mislabelled kind (M6), non-assumable (M4), host-derived (M5). M1 is
informative beyond its row: with the pre-spawn check neutralised the frozen world was still refused,
before consumption. Limits are F-8 and F-9. `RealizationDiffers` and `PreparationChanged` are equivalent
mutants today because both sides are pure functions of immutable inputs; that is acceptable for
`PreparationChanged`, and the F-2 repair makes `RealizationDiffers` discriminable.

## 6. Label truthfulness

- Assumed discharges are named and labelled as assumptions, never admissions (`gate.rs:1-21,26,33`).
  Holds, except that the label is dropped on success (F-4).
- Production admission: the gate, binding and runner exist only under `fixture` (`lib.rs:42-47`); the
  default-feature doctest (`lib.rs:31-34`) passes as compile-fail in the root gate. Holds.
- The unclaimed list in RESULT.md is accurate as far as it goes. Add: the description presumes the
  crate's adapter and the selected SDK (F-2); `run_attempt` is an ungated generic spawn route (F-3);
  mutants are pre-checkpoint, n = 1 (F-9).

## 7. Requested repair (smallest set)

1. F-1: reword the mutation member to the scoped, host-backed claim in the three places named; record
   the root ruling.
2. F-2: make the description depend on `cfg.adapter`, with one test and one mutant; or, at minimum,
   correct `native.rs:222-223` and extend the unclaimed list.
3. F-3: narrow the two absence sentences.

F-4 to F-9 can be carried as recorded limits. A recheck can be confined to these hunks and the new test.
If the root carries F-1 to F-3 without change instead, the acceptance record needs to name the three
source statements as known to be inaccurate; I would not call that private readiness for invariant 2.
