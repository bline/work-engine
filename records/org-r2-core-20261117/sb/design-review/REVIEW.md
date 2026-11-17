local_design_correction

# S-B escalation design review (Fable 5.2 `claude-fable-5-2`, read-only)

Parent and decider: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. Advisory under
`ARCHITECTURE-GUARD.md` (trigger 3, recurrence of an invariant violation). Subject: S-B checkpoint
`dd68df6c`, read at the clean export `sb/export/rust/crates/`.

**Evidence standing.** Static reading only; no build, test or mutant was run. Read: the guard, this packet,
`AGENTS.md`, `docs/contracts/core.md`, S-B `REVIEW-1.md` (all), S3 `REVIEW-1.md` §2 F-2 (lines 112-150
only), S-B `PACKET.md`, `RESULT.md`, `SA-ACCEPTANCE.json`, `ROOT-RULINGS-1.md`; review-delivery
`src/gate.rs` (all), `src/native.rs` lines 184-299, 578-637, 1296-1581, `src/binding.rs` lines 126-225 and
330-417; S-A `src/gate.rs` lines 212-586 and 936-1339; `observation.rs` lines 123-179 by search;
`tests/common/mod.rs` lines 488-535. Not read: `run_native`, `MATRIX.md`, the mutant records, the test
files apart from the lines named, `FixtureDeclaration`, `EpisodeInputs`. Line numbers are export lines.
Not done: the board read and context-pressure delivery that `AGENTS.md` asks for at session entry (this
packet limits reads and forbids the state directories; no claim, launch or paid operation was made here).

## 1. Verdict

`local_design_correction`. The plan and the crate boundaries stand. The host's input surface for the
admission does not: `FixtureAuthority.request` hands the caller the whole `OperationRequest`, and the host
then compares an enumerated subset of it with the binding. The root's candidate (the host constructs the
request) is the right correction and I recommend it, with one factual amendment (§3) and four companion
changes (§6). A third field-level patch (add the vantage comparison) is not credible as a repair of the
class, for the reason in §2.

## 2. Diagnosis: one invariant, two input surfaces of the same function (n = 2)

Invariant: *what the gate evaluates is determined by what the host will run and consume.*

| Occurrence | Caller input | What it selected | Repair taken |
|---|---|---|---|
| S3 F-2 (`24597fef`) | `NativeConfig.{node,adapter,sdk_root,control}` | the program that runs, under a constant description | description keyed on `adapter`; other three classified "presumed inputs" |
| S-B P2-1 (`dd68df6c`) | `FixtureAuthority.request.{vantage, organization_id}` | which vantage R1/R2/R3 are decided for; which vantage bindings O8 and R3 consult | none yet |

Both are inputs of `deliver_and_consume_observed`. Three facts make the comparison form structurally
unable to close the class:

1. **A comparison list is partial by enumeration; a constructor call is total by signature.**
   `OperationRequest`'s fields are private to S-A (`gate.rs:227-233`), so review-delivery cannot
   destructure it exhaustively the way the S3 repair made `NativeConfig` exhaustive
   (`native.rs:254-263`). A field S-A adds later is silently caller-chosen under the comparison form.
   `OperationRequest::new` takes all five fields positionally (`gate.rs:238-244`); a new field is a
   compile error at the host's one construction site. This holds while `new` is the only constructor.
2. **The comparison was specified upstream, not improvised.** S-A F-7 → `ROOT-RULINGS-1.md` → packet
   invariant 2 each name the same four fields. The builder implemented that list and disclosed the
   vantage gap (RESULT decision 3). The correction therefore belongs in the ruling and packet wording,
   not only in the code.
3. **The test world agrees with the binding by convention.** `tests/common/mod.rs:493-509` builds the
   request from the constants `ORG` and `VANTAGE`; nothing relates `VANTAGE` to
   `binding.preparation().model_context.vantage.id`. Passing tests therefore say nothing about the
   relation (n = 1 world read).

The architecture guard asks for diagnosis, not a presumption that the architecture is wrong. The boundary
placement is sound: S-A decides, the host reserves, observes, regates and commits, the preparation owner
derives blockers, `discharge` is keyed on kind and `CheckId`. Nothing I read argues for moving any of it.

## 3. The root's candidate: correct, with one amendment

**Amendment (fact).** The binding does not own the requirements. `OperationRequest::new` needs the whole
`OrganizationalRequirements` by value, and S-A O3 re-derives the candidate from it (`gate.rs:459`).
`ExperimentBinding` holds an `OrganizationCandidate` (`binding.rs:134-147`), whose fields are the
requirements *digest*, profile, problem scope and derived parts (`derive.rs:204-219`), not the
requirements. So "the host constructs the whole request from facts it owns" is not achievable as worded.

Handling that keeps the correction local: the requirements stay an input, as a **digest-bound preimage**.
The host refuses unless `requirements.requirements_digest() == binding.candidate().requirements_digest()`,
and it sets `candidate_digest = binding.candidate().content_digest()` itself, so O3 then checks that the
supplied requirements derive the *binding's* candidate. That is different in kind from comparing chosen
fields: the caller can supply only the one value the host's digest already names. It also upgrades O3 on
this path from request self-consistency (S-A unclaimed) to consistency with the binding's candidate; it is
still not a committed admission anchor. Whether the host-side digest check is redundant with O3 is
`not established`: `derive.rs:1439-1440` writes `requirementsDigest` into a candidate encoding, but I did
not read whether that encoding is the one `content_digest` hashes. Keep the check; it costs one line and
gives a named refusal before any observation is taken. The alternatives (the binding retains the
requirements; an S-A constructor from a candidate) are larger and touch S1/S3 fixtures or S-A semantics.

**Does construction remove the class?** For the request, yes:

- `vantage` ← `binding.preparation().model_context.vantage.id` (derived by the preparation owner from the
  candidate and obligation, `prepare.rs:400-409`). P2-1's scenario becomes unrepresentable: R1, R2 and R3
  are decided for the vantage whose contract the child receives.
- `subject` ← `binding.request().subject`; `candidate_digest` ← `binding.candidate().content_digest()`;
  realization digest ← `realization_digest(&realization_description(cfg))` (already so, `native.rs:1496`).

It does not remove the class for the inputs in §4 that have no host-owned fact behind them. Those must be
named, typed as assertions and labelled; they cannot be derived.

`compare_with_binding` must then be **deleted, not kept**. After construction the admission's request is a
clone of the host's (`gate.rs:433`), so every branch is a tautology that no mutant can kill. That is the
shape S3 F-2 already recorded for `RealizationDiffers`, and REVIEW-1 P3-4 records it for the realization
branch here. `AdmissionMismatch` survives only for the requirements preimage.

## 4. Gate input inventory after the correction

Every input that reaches `admit`, `evaluate` or `admission_state` on the host path (10 rows):

| Input | Today | Host-owned fact? | Handling |
|---|---|---|---|
| request vantage | caller | yes (preparation) | host-constructed |
| request subject | caller, compared | yes (binding) | host-constructed |
| request candidate digest | caller, compared | yes (binding) | host-constructed |
| request requirements | caller, digest compared | digest only | digest-bound preimage; refuse on mismatch before observing |
| request organization id | caller, uncompared | **no** | stays caller-asserted; own field, labelled unanchored (below) |
| realization digest | host | yes | unchanged; inherits S3 `p3_carried[0]`: the same digest for every `node`, `sdk_root`, `control` |
| declared `actor_identity`, `installed` | caller | no (no observation of the child; owner decision O8) | stay declared; already unclaimed in S-A and S-B; label must reach `Delivered` |
| boundary `operation` | caller | yes | host derives it from the binding (e.g. the frozen contract digest); epoch stays 1 (O10) |
| `now_millis` | caller, one value | yes (when the host reaches a boundary) | host samples a time *source* at each evaluation (§5, P3-2) |
| authority `source` | caller | no, by role | the one input meant to be supplied; labelled synthetic; hold it in `HeldAdmission` so `regate` cannot be given another |

On `operation`: `AuthorityObservation::observe` takes no boundary (`observation.rs:123-126`) and
`evaluate` takes none, so the operation name selects no check today. Deriving it is a one-line change that
makes `AdmissionSummary.boundary` name the binding and makes the O10 gap concrete (the same binding
re-delivered reserves the same boundary identity).

On the organization id: it selects which vantage bindings O8 and R3 consult (`gate.rs:978`, `:1169`), so
Separation and ObserverIdentity are discharged *relative to a caller-asserted organization*. No anchor
exists in what I read: `work-engine-organization/src` has 0 text hits for `organization_id` (whole
directory searched), and the binding struct, `PreparationRequest` and `CandidateIdentity` carry none;
`FixtureDeclaration` and `EpisodeInputs` were not read. A real anchor is a record relating an organization
to a problem scope or candidate, which is an S-A record-schema question with no owner yet (the same kind of
gap as the O7 mapping rules). That is a plan-level open item for the root to record with an owner. It is
not a reason to reopen S-B, whose standing is fixture and whose vehicle for this is the unclaimed list.

## 5. P3-1, P3-2, P3-3

**P3-1 (delivery admission never regated).** Not a caller-chosen input; a sequencing gap with the same
root: the two pre-spawn admissions are not bound to one authority state. S-A's contract is that an
admission permits an effect only when the host's own regate against a fresh observation passes
(`gate.rs:322-327`). The spawn is an effect and has no regate. Correction: regate the delivery admission
immediately before `run_native` and require `Current`. This states one rule for both effects (no effect
without a current regate of its own admission) and catches a pin replaced between the two admission
observations as `Stale(IssuanceSuperseded)`, spawn 0. Keep decision 1 (consumption admission taken before
spawn). Revision equality between the two admissions is a weaker substitute: it leaves the spawn effect
without a regate, so if chosen it must be stated as such.

**P3-2 (single time input).** Under the class definition this is a member: the time at which the host
reaches a boundary is a host fact, and a caller value decides O4 expiry at both admissions and at the
regate. S-A's point check re-runs the gates "if the revision moved or time passed" (`gate.rs:1273-1274`);
passing the admission-time value disables the second trigger. Correction: the caller supplies a time
*source* (labelled synthetic, like the authority source) and the host samples it at each evaluation
(two admissions, two regates). Expiry during the run then shows as `Refused` at the consumption regate.
If the root prefers to defer it, it must be an unclaimed line and the sentence at `native.rs:1463-1465`
must say the regate's observation is fresh and its time is not.

**P3-3 (`test_support` and `consume_fixture_result`).** A different class: a route around the gate, not an
input to it. Two parts.

- `consume_fixture_result` is public in review-preparation under that crate's `fixture`
  (`fixture.rs:403`), so any fixture build can consume a result with no delivery gate, with or without
  `test_support`. Review-delivery cannot close that from inside. Closing it means the consumer demands a
  gate-issued proof, which moves a boundary between the preparation and delivery owners and belongs to
  the slice that introduces the committed consumption effect. **Carry it explicitly** (S3
  `p3_carried[1]`, second half) with that owner; do not fold it into this correction.
- `test_support` is public under the only build in which `native` exists, so the statements at
  `native.rs:593` ("the only spawn route outside tests is the gated delivery path") and `:1468` ("the only
  route that runs the SDK delivery protocol from a binding") are true by convention only: a fixture caller
  can pass `binding.frozen()` contract bytes to the generic runner. Absence statements about spawn routes
  were already corrected once (S3 F-3). Either move `test_support` behind a separate non-default feature
  not implied by `fixture` (then the first statement holds for the `fixture` build by `cfg`), or narrow
  both sentences to "the only route in this crate that takes a binding". A feature is a statement of build
  intent, not access control; say that.

## 6. Minimal correction scope (review-delivery only; no S-A, organization or preparation change)

1. `FixtureAuthority`: remove `request` and `operation`. Fields become: `source`; `organization_id`
   (doc: caller-asserted, unanchored); `requirements` (digest-bound preimage); `actor_identity` and
   `installed` (declared); a time source in place of `now_millis`.
2. One private host function builds the `OperationRequest` from the binding, the requirements preimage and
   the asserted organization id, and refuses `AdmissionMismatch { field: "requirements" }` on a digest
   mismatch. Called once; both admissions use its result. Delete `compare_with_binding`.
3. The host derives the boundary operation from the binding.
4. Regate the delivery admission immediately before `run_native` (P3-1).
5. Time source sampled per evaluation (P3-2), or the deferral label.
6. `HeldAdmission` keeps its source (and time source), so `regate` takes no argument that could differ
   from the admission's. Small; it removes REVIEW-1's "trusts the caller to pass the admission's own
   source" (decision 5).
7. Labels on success: `AdmissionSummary` (or `Delivered`) carries the asserted inputs (organization id,
   declared actor identity) under a label such as `caller_asserted_unanchored`, plus the request identity
   the host constructed (vantage, subject, candidate digest). S3 F-4 established that a label which does
   not reach the success output is a defect.
8. `test_support` feature split or sentence narrowing (P3-3); doc sentence at `native.rs:1463-1468`.

Items 1-4 and 7 are the correction. 5, 6 and 8 are the same move applied to the remaining inputs and are
small; each may instead be carried if it is labelled as an open member of the class.

## 7. Tests and mutants (one mutant each, n = 1, as in the S-B matrix)

| # | Test | Mutant that it must kill |
|---|---|---|
| T1 | A world in which the preparation's vantage binding names another actor while a second candidate vantage V′ is bound to the declared actor and the host digest. Delivery refuses before spawn (spawn 0, consumer 0); assert the exact refused check set and the R3 reason code and subject (the preparation's vantage), so a refusal for a different reason cannot pass the test. | construct the request with any other vantage (constant, or the candidate's first other vantage) |
| T2 | Positive: `Delivered` reports the constructed request identity equal to the binding's vantage, subject and candidate digest, and the asserted inputs with their label. | construct with a constant subject; construct with a zero candidate digest (O3 refuses the positive test) |
| T3 | Requirements from another world (the frozen world's) refuse before spawn with `AdmissionMismatch { field: "requirements" }`, source observed 0 times. Closes REVIEW-1 P3-4's missing requirements case. | skip the digest check (the refusal then comes from O3 after an observation: the refusal identity and the observation count both differ) |
| T4 | A scripted source whose second observation supersedes a pinned issuance with a passing replacement: refuse before spawn, `NotCurrent(Stale(IssuanceSuperseded))`, spawn 0. | remove the delivery regate |
| T5 | A grant that expires between the delivery admission and the consumption regate under a scripted time source: `Refused` at consumption, consumer 0, run retained. | regate with the admission-time sample |
| T6 | Compile-fail doctest: `FixtureAuthority` has no `request` field. Same standing as S-A N-1 (stable rustdoc asserts non-compilation only). | restore the field |
| T7 | Summary boundary operation is derived from the binding: two bindings yield different operations, one binding yields the same. | constant operation |

Retire `sb_an_admission_for_another_subject_or_candidate_refuses_before_spawn` and mutant M5: the inputs
they vary no longer exist. Record that in MATRIX as a replaced claim, not as coverage kept. Whether the
existing test world can host T1's V′ is `not established` (REVIEW-1: its `vantage-builder` binding names
`real-builder`); T1 as written does not need V′ to pass every check, because it asserts the exact refusal.

## 8. Carry or label (not repaired by this correction)

- Organization id: caller-asserted and unanchored; Separation (O8) and ObserverIdentity (R3) are
  discharged relative to it. Plan-level open item: an organization anchor record, owner to be named.
- Declared actor identity and installed digests: R3 passes for whatever the caller declares that matches
  the record (owner decision O8; already unclaimed).
- `node`, `sdk_root`, `control`: presumed inputs from the S3 repair; the realization digest does not vary
  with them (REVIEW-1 P3-7 e). This is the unrepaired remainder of the S3 occurrence of the same class and
  should be listed as such, not only under "description, not bytes".
- `consume_fixture_result` public route (§5).
- The binding itself is caller-built (`ExperimentBinding::new` is public). That is not this class: the
  binding is the object evaluated, and its obligation selects the delivered contract and the evaluated
  vantage together.
- O10 (no single-use ledger), fixture standing, assumed mapping kinds: unchanged.
- REVIEW-1 P3-5 (comment claims more than the supersession assertion shows) and P3-6 (dropped S3
  positive assertion): restore or record in the same repair packet. P3-4 dissolves with
  `compare_with_binding`.

## 9. What the root needs to record

- **Ruling.** Restate S-A obligation F-7 and packet invariant 2 from "compare candidate, requirements,
  subject and realization digests with the binding" to "the host constructs the admission request from
  the binding; the requirements are a digest-bound preimage; no caller-supplied request exists". This is a
  wording change to a root ruling and a root packet inside the approved S-B subject. I see no scope or
  held-authority change that would make it the operator's decision; that reading is the root's to confirm.
- **Guard.** Per the guard's reopening rule: freeze this correction as the revised S-B design, record the
  history revision, trigger 3 state and this decision in `assignment.json`, and have the consequential
  change reviewed before or with the repair's independent review. The S3 F-2 and S-B P2-1 events stay in
  history.
- **Repair packet.** Give the builder the inventory in §4 as the acceptance frame (every gate input is
  host-derived, digest-bound, or a named labelled assertion) instead of a field list. A reviewer can then
  check the frame against the function signature rather than hunt for the next unlisted field.

## 10. Why not the other verdicts

- `retain_with_bounded_repair` (add the vantage comparison): it closes P2-1's stated scenario and leaves
  the surface that produced it. It would be the third field-level patch on the same function, it cannot be
  made exhaustive across the crate boundary (§2 point 1), and the specification that enumerated the
  fields would stay in force.
- `reopen_plan`: nothing read shows a boundary in the wrong crate or an S-A semantic that must change. The
  correction is confined to one crate's host input type and one function. The one plan-level finding (no
  organization anchor) is an open item to record, compatible with S-B's fixture standing.

Confidence: high on the code facts cited with lines; high that construction is total where comparison is
not; medium on the time-source item being worth doing now rather than labelling; the redundancy of the
requirements digest check with O3 is `not established`.
