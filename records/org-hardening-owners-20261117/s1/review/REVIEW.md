# S1 review: post-R2 gate hardening (fixture)

Reviewer: Claude, requested Opus 5.5 xhigh, observed `claude-opus-5-5`. Read-only: no builds, no
mutating git. Subject: the uncommitted diff of `wt-org-hardening-20261117` vs `0fe9dc74` (12 files,
+731/-71). Contract: `s1/PACKET.md`. Context usage: `usage_unavailable` (no binding was given).
`references/reviewer.md` does not exist, so I loaded `references/worker.md`.

Custody note: contrary to the packet, RESULT.md and MATRIX.md exist (13:32/13:33, `map_ready`); used
as pointers only, every claim checked against code or custody logs.

## Verdict: `changes_required`

Invariants 1–3 are met in code; mutants are killed for the right reasons. Two P2 defects (docs, one
label) need fixing before private-readiness acceptance. No design escalation.

## Findings

**P2-1: one of the gate absence sentences is false.**
- Where: `review-delivery/src/native.rs:26-29` says `consume_fixture_result` "is not a route around
  the gate in any build". `lib.rs:25-27` says the consumer needs "*the* consumption admission with
  *the host's* fresh regate observation".
- Failure: when S-A `fixture` is on (`review-delivery/fixture` turns it on), any caller can build its
  own `FixtureSource`, call `admit::<Fixture>` for any operation at a Consumption boundary, then
  `observe_for`, then `consume_fixture_result`. The episode advances with no delivery gate, spawn,
  native correspondence or blocker evaluation.
- Evidence: the slice's own test does exactly this
  (`s1_a_current_gate_proof_for_this_consumption_lets_the_episode_advance`, op
  `fixture-consumer-op-1`, `tests/common/mod.rs:334-369`). RESULT's unclaimed list concedes it.
  This is the S3 F-3 class that invariant 2 was meant to clear.
- Fix: state what is true. Without S-A `fixture`, no episode can advance. With it, any caller can
  mint a fixture admission and consume outside this crate's gate. The consumer checks only that S-A
  found *a* consumption admission current for its candidate, subject and vantage; it cannot tell
  whose admission it is or how fresh the observation is.

**P2-2: observation freshness is an unlabelled runtime caller assertion.**
- Where: `fixture.rs:206-211` says the consumer cannot tell a fresh observation from an earlier one,
  but `GateCheck` (`:227-241`) carries only the time label.
- Failure: a direct caller passes an earlier observation for the same reservation. The result is
  `Current`, and the success output does not show that freshness was asserted rather than checked.
- Why it matters: the acceptance frame allows no unlabelled caller assertion, and S3 F-4 holds that a
  label which does not reach the success output is a defect. RESULT itself says "documentation only".
- Fix: add a label such as `"caller_asserted_regate_observation_freshness"` (or widen the time label),
  expose it on `GateCheck`, assert it in the positive test, and update the inventory row.

**P3-1: the consumer checks the boundary kind only.** `fixture.rs:252` ignores operation and epoch,
so a Consumption admission for another operation passes. That is narrower than the packet's "another
boundary". It is disclosed only in RESULT Deviations, not in the module or `ConsumptionProof` docs.
Exposure is small (P2-1; the host's admission never leaves `deliver_and_consume`). Fix: document the
narrowing and get owner acceptance, or compare the operation if the consumer can derive it (`not
established`).

**P3-2:** the vantage check (`fixture.rs:268-271`) has no negative test or mutant (disclosed; the
`Ask.vantage` hook exists). Add the case plus a mutant, or call it defense in depth.

**P3-3:** S1-P1..P5 all panic at the shared `fixture_consumer.rs:186` `expect_err`, which does not
name the loop case; pass the name into `refused_with`. **P3-4:** `native_vertical` now requires
`test-support`, so S-B gated-path tests run only where `test_support` exists (behaviour unchanged;
optionally split out a `fixture`-only target).

## Invariant 1: consumption proof

- **Unforgeable outside S-A.** `EffectAdmission` (S-A `gate.rs:328`) and `SealedObservation`
  (`observation.rs:150`) derive only `Debug`, have no public constructors; `Policy` is sealed
  (`gate.rs:180-210`) and `Production` admits nothing, so no admission exists without S-A `fixture`.
- **No feature leakage.** With `resolver = "3"`, the dev-dependency's S-A `fixture` does not reach lib
  builds (the `check-preparation-fixture-lib-alone` step passed). No other crate depends on
  review-delivery or enables `test-support`.
- **Ordering.** `check_proof` (`fixture.rs:740`) runs after the existing checks and before
  `TransitionCommand::new`. It is the only `transition(` call in either crate's `src`. Other episode
  writers in the workspace (e.g. `review-episode`) are out of scope and were not reviewed.
- **Delivery path / standing.** It passes the admission plus the observation and time
  `regate_observed` decided on (observes every call; `None` once settled; `Current` never settles).
  Standing is truthful: time labelled; F-3 finality not carried; proof reusable (O10; the episode owner
  refuses a second use on revision); freshness is the host's obligation, label missing (P2-2).

| Proof input | Class | Handling |
|---|---|---|
| admission (existence) | S-A-issued, fixture domain | sealed `Policy`; any caller can mint one under S-A `fixture` (P2-1) |
| candidate digest | digest-bound | `== candidate.content_digest()`; requirements bound by S-A O3 |
| subject | digest-bound (transitively) | `==` declaration subject, which is sha256-checked against the episode (`:607-618`) |
| vantage | consumer-derived | `== Selection::new(candidate, obligation_id).served_by`; untested (P3-2) |
| boundary kind | checked | `Consumption` |
| boundary operation / epoch | **unchecked** | P3-1; O10 |
| organization id, actor identity | labelled caller assertion (S-B) | not re-checked |
| observation content | S-A-sealed | another boundary gives `Stale` |
| observation freshness | host-derived on the delivery path; caller assertion otherwise | doc-only label (P2-2) |
| `regate_now_millis` | labelled caller assertion | `GATE_TIME_ASSERTED_LABEL`, via `GateCheck::time_label()` |

## Invariants 2 and 3

- **Invariant 2: met.** `test_support` is under `#[cfg(feature = "test-support")]`; `test-support =
  ["fixture"]` is not implied by `fixture`; the cfg-gated `compile_fail,E0432` doctest (native.rs
  line 45) fails under S1-P6. Spawn-route sentences are true apart from P2-1.
- **Invariant 3: met.** `with_regate(revision)` on the delivery summary; `Some` asserted for both
  (`native_vertical.rs:353-369`); the ticking test (`:1219-1256`) shows each regate sequence above its
  admission's and `gate.regate_revision` equal to the consumption regate's. T5 reason: `:1173-1181`.

## Invariant 4: mutants (logs read for S1-P*)

- **S1-P1/P2/P3 kill the right case.** Each removes one `return mismatch(..)`. The loop order is
  candidate, then subject, then delivery boundary. A panic at `:186` therefore means the mutant's own
  case was accepted: an earlier case whose refusal changed would have failed on its own
  `assert_eq!(.., "{name}")` line.
- **S1-P4 kills the right case.** Revoked still refuses and Unknown is accepted. The name "missing
  proof" is a misnomer: the no-argument claim rests on the unmutated `compile_fail,E0061` doctest
  (`fixture.rs:25-36`). Its paths match `binding.rs`, and stable rustdoc does not check the error code.
- **S1-P5/P6/P7:** `unwrap_or` lets Revoked through; only the cfg doctest fails (8/1); panic at
  `native_vertical.rs:1242`. **M1–M4, M8–M11** are faithful to `sb/mutants/mutate.py`.
- **M7** ignores the regate's decision (it cannot skip the regate: the proof needs an observation);
  closest faithful form; killed by refusal identity and consumer count, since the consumer also refuses.
- **M6 (runs-2)** is killed on the host claim by `regate_reads_a_fresh_observation`; the native test
  dies only because the fresh observation still reaches the consumer. The variant where the replay
  also becomes the proof (the one case the consumer cannot detect) was not run: `not established`.

**Scope and regressions.** No S-A file changed; one optional dependency edge, no cycle; T5 in scope;
no storage/issuance/hosted change. Only overclaim: P2-1. No S-B code regression (P3-4 is coverage).

**Not established.** Build/gate/mutant outcomes beyond the custody records (no builds run; hashes and
test counts not recomputed); whether the consumer can derive the contract digest (P3-1); whether S-A
admits another vantage for the same candidate (P3-2); the stronger M6 variant; doctest error codes.
