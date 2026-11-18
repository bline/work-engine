# S1 recheck (after remediation 1)

Reviewer: Claude, requested Fable, observed `claude-fable-5-2`. Read-only: no builds, no mutating git, no network.
Subject: uncommitted diff of `wt-org-hardening-20261117` vs `0fe9dc74` (12 files, +788/-73). Context usage and
cost: `usage_unavailable` (no binding given). Paths below are under `rust/crates/` unless absolute.

## Verdict: `changes_required` (one finding, R-1)

P2-1, P2-2, P3-1 and P3-3 are closed. The P3-2 carry rests on a premise that holds only for a single-obligation
candidate: a wrong-vantage admission is reachable by construction, so the vantage comparison is load-bearing.

## R-1 (P2, was P3-2): the vantage comparison is load-bearing and has no test

- Builder's evidence verified: S-A R2 returns `VantageUnknown` unless a `vantage_requirements()` entry names the
  request vantage (`work-engine-organization-authority/src/gate.rs:1090-1096`). `vantage-builder` is not such an entry.
- The conclusion does not follow. `work-engine-organization/src/derive.rs:484-493` pushes one `VantageRequirement`
  per obligation (`vantage:<obligation id>`, `:373-375`), so a two-obligation candidate carries two. Such candidates
  derive: `work-engine-organization/tests/rules.rs:1108-1110, 1446-1465`, and the preparation crate's own
  `tests/preparation.rs:551-575` (`with_second_obligation`, built from the frozen requirements).
- For that candidate S-A can admit `vantage:<second>` at a Consumption boundary with the same candidate digest
  and subject. R1 filters conditions by the request vantage (`gate.rs:1079-1089`), R2 uses that vantage's effective
  requirements (`:1097-1101`), and the actor binding is per vantage (`:1154-1193`). The admission therefore says
  nothing about the first obligation's constraints.
- In the consumer, kind, digest and subject (`work-engine-review-preparation/src/fixture.rs:277-292`) are equal for
  both vantages and the point check (`:297`) does not depend on vantage. `fixture.rs:293-296` is the only line that
  refuses that admission when the declaration names the first obligation.
- Consequence: S1-P8 survives (15 passed, 0 failed), so deleting the comparison is undetected, while five doc sites
  state "candidate, subject and vantage". `tests/fixture_consumer.rs:776-782`, MATRIX and remediation RESULT call
  it defense in depth on evidence from n = 1 candidate (the frozen one).
- Limit of this finding: reachability is by code reading; I ran nothing, so whether S-A's remaining checks admit
  `vantage:<second>` in a test world is `not established`.
- Required: one negative case (two-obligation requirements, a bound and accepted second vantage, admission through
  `Ask.vantage`, declaration for the first obligation, expect `GateProofMismatch { field: "vantage" }`) with S1-P8
  killed, and the comment corrected. If the owner carries the gap instead, the comment, MATRIX and RESULT must say
  "load-bearing for multi-obligation candidates, untested".

## Closed

- **P2-1.** `work-engine-review-delivery/src/native.rs:26-34`, delivery `src/lib.rs:25-30`, preparation
  `src/lib.rs:17-24`, `src/fixture.rs:21-31`. True in the four combinations I checked: no consumer without
  preparation `fixture`; consumer but no possible admission without S-A `fixture`; any caller can mint with both;
  `test-support` named as a spawn route outside the gate. Nit: "whenever S-A's `fixture` is on" omits that the
  consumer also needs preparation `fixture`; that overstates exposure, not protection.
- **P2-2.** Constant `fixture.rs:209-210`, method `:264-266`, on `FixtureConsumption.gate` (`:512`); asserted against
  the literal and the constant at `tests/fixture_consumer.rs:763-770`; MATRIX rows 77 and 84-85. Same form as the
  accepted `time_label()`; `GateCheck` is not `Serialize`. The label is unconditional (also on the delivery path).
- **P3-1.** Documented at `fixture.rs:24-28`, in the `ConsumptionProof` docs and `native.rs:32-33`. No code change.
- **P3-3.** `refused_case` (`tests/fixture_consumer.rs:174-199`); the five kill logs name their case (`:192:24`).

## Check 3: nothing else changed

- Current blobs equal `gate-3/hashes.stdout`, 12 of 12 (I re-ran `git hash-object` on the same list). Delivery
  `gate.rs`, `binding.rs`, both `Cargo.toml` and `Cargo.lock` equal gate-1, 5 of 5.
- `native.rs` against its gate-1 blob `48f8d280` (`s1/mutants/S1-P7-*/orig-0.bak`, hash confirmed): one hunk, module
  doc lines. `fixture.rs` against `03da1081` (`S1-P1-*/orig-0.bak`): docs, one constant, one method; `check_proof`
  untouched. Both `lib.rs` S1 diffs are `//!` lines only.
- `tests/common/mod.rs`: no gate-1 blob exists, so byte equality is `not established`. Supporting: 259 insertions
  equals the original `RESULT.md:88`; `pub fn admitted` sits at `:334`, as the prior review cited; `fixture-other-review`
  has 0 hits in the three crates and `real-builder` has no acceptance. The `vantage-builder` binding at `:255-261`
  is the S-A base world's shape (S-A `tests/admission.rs:317-325`), not a leftover.

## Check 4: rest of the S1 diff

No blocker found in the delivery `gate.rs`, `binding.rs` and `native.rs` code hunks. `regate_observed` returns inputs
only when it observed; `(Current, None)` cannot occur and would refuse. P3 nit: `work-engine-review-preparation/Cargo.toml:17-19`
still says "the host's fresh observation" without the caller-assertion qualifier (`fixture.rs:17-18` has it one paragraph later).

## Carried and not established

- Carried: owner acceptance of the P3-1 narrowing; P3-4; the stronger M6 variant; S1-P6, S1-P7 and M1-M11 stand on
  gate-1 blobs (the `native.rs` change since is docs only, per the diff above); duplicate `remediation-1/mutate.py`.
- Not established: any build or test outcome beyond the custody records; gate-1 against gate-3 test counts.
- Tooling: Bash was limited to single read-only git commands in this session.
