changes_required

# S-B independent review 1 (Opus 5.5 `claude-opus-5-5`, requested effort xhigh; read-only)

Subject: checkpoint `dd68df6c7a7f526d934bc16354fe632c985620ff` (tree `59eb0a36…`), diff `bfaa455c..dd68df6c`
(9 files, +1330/−189, read in full for `src/gate.rs`, `src/native.rs` changed regions, `src/lib.rs`, `Cargo.toml`,
`tests/gate.rs`, the S-B tests in `tests/native_vertical.rs`, `tests/common/mod.rs` `authority` module,
`DERIVATION.md`). S-A sources read at the export: `observation.rs`, `fixture.rs`, `lib.rs`, `gate.rs` (admit,
evaluate, `o8`, `r1_r2`, `r3`, `admission_state`). Line numbers below are export lines
(`sb/export/rust/crates/...`). Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`.

**Evidence classes.** I ran no build or test. *Executed facts* cited here are the builder's and root's custody
records: root gate at `dd68df6c` passed (`sb/ROOT-GATE.json`: fixture tests 324 passed, default 42, authority-store
37, native 28/28 ×3); mutant records in `sb/mutants/runs-1`. Everything else is static reasoning from the source.

Verdict basis: one P2 (P2-1). The remaining items are P3. Invariants 2–6 hold as implemented; invariant 1
holds for the four F-7 fields but not for the request vantage that three of the four admitted kinds depend on.

---

## P2-1 — The admission's vantage is caller-chosen and never compared with the binding (decision 3)

- `FixtureAuthority.request` is a public caller input (`src/native.rs:1330-1338`). `compare_with_binding`
  (`src/native.rs:1410-1439`) compares exactly candidate digest, requirements digest, subject and realization
  digest. `request.vantage()` and `request.organization_id()` are not compared with anything.
- In S-A, R1, R2 and R3 are decided **for `request.vantage`**: `gate.rs:552` (`r1_r2(c, &request.vantage, …)`),
  `gate.rs:1082` (conditions filtered by `c.vantage == vantage`), `gate.rs:1093` (effective requirements of that
  vantage), `gate.rs:1154-1192` (R3 resolves *that vantage's* binding and compares actor and realization digest
  with it).
- S-B's `discharge` (`src/gate.rs:75-89`) discharges ObserverIdentity (R2+R3), RealizationAdmission (R1+R2+R4),
  Separation (O8+R3) and ComponentBinding (O4+R3) with those checks. The blockers being discharged belong to
  the binding's preparation, whose vantage is `binding.preparation().model_context.vantage.id`
  (`work-engine-review-preparation/src/prepare.rs:88-115`), which the host already holds.
- Failure scenario (static, not executed): an owner-issued fixture world in which another candidate vantage V′
  (for example the builder vantage) is bound to the declared actor and the host realization digest, and whose
  effective requirements the accepted realization meets. A caller passes `request.vantage = V′`. All nine
  checks pass for V′, F-7 passes (all four compared fields match), and `Delivered.admitted_before_*` reports the
  reviewer preparation's ObserverIdentity and RealizationAdmission blockers as discharged by R2/R3 — checks that
  established the binding of a *different* vantage's actor. This is the separation outcome the gate exists to
  refuse (a non-reviewer actor runs the reviewer contract and its result is consumed). Whether the current test
  world admits such a V′ is `not established`: its `vantage-builder` binding names realization `real-builder`
  (`tests/common/mod.rs:424-430`), so R3 would refuse there.
- Why this is a defect and not a scoping choice: F-7's purpose (ROOT-RULINGS-1: the admission must describe the
  binding) and invariant 1 ("never supplied by a caller") both separate a trusted authority source from a
  caller-supplied request. The four-field list does not cover the one request field that selects which
  Gate R checks run. The host has the value; the comparison is one line.
- Fix: in `compare_with_binding`, refuse `AdmissionMismatch { field: "vantage" }` when
  `request.vantage() != binding.preparation().model_context.vantage.id`; add a native refusal-before-spawn test
  (spawn 0, consumer 0) and one mutant. The organization id cannot be compared (the binding and the
  organization crate carry none; `work-engine-organization/src` has 0 occurrences of `organization_id`), so it
  belongs in the unclaimed list (P3-7).
- Confidence: high that the comparison is absent and that R1/R2/R3 key on `request.vantage`; medium on how
  easily a passing V′ world arises in practice.

---

## P3 findings

**P3-1 — Delivery admission is never regated; a pin replaced between the two pre-spawn observations is invisible
(decision 2).** `src/native.rs:1510-1522`: the delivery admission is taken, then the consumption admission from a
second `observe()`, then spawn. S-A `EffectAdmission` permits an effect "only when the consuming host's own regate
… also passes" (`gate.rs:322-327`); the spawn effect has no regate. Because `evaluate` takes no boundary, the
consumption admission is effectively a later re-check of the same request, so the only unrepresented case is a
pinned issuance superseded between the two observations by a replacement that still passes: the consumption
admission pins the replacement and its regate is Current, which is the silent-replacement outcome decision 1
was designed to prevent. Exposure is narrow (needs a source that changes between two consecutive calls).
Fix: require `delivery.admission().revision() == held_consumption.admission().revision()`, or regate the
delivery admission immediately before `run_native`. Confidence: high (static).

**P3-2 — One caller-supplied time for admission and regate; expiry between the boundaries is not representable.**
`src/native.rs:1507` and `:1549` both pass `authority.now_millis`. Grants carry `expires_at_millis`
(`records.rs:137`, `AuthorityExpired`). A grant that expires during the ~70 s native run is still `Current` at the
regate. Invariant 3 names revocation, supersession and missing grant only, so this is not a breach. It is
missing from RESULT's unclaimed list, and the doc at `src/native.rs:1463-1465` ("regate it against a fresh
observation") is true of the observation only. Fix: a second time input for the regate, or an unclaimed line.

**P3-3 — `test_support` closes the R2-S P3 in name only (decision 4); with the still-open half of that S3 P3, an
ungated binding→child→consumer route exists in any fixture build.** `native` exists only under `fixture`
(`src/lib.rs:48-49`), so `native::test_support::run_generic_attempt[_observed]` (`src/native.rs:609-631`) is the
same public generic spawn route under its only build. With public `AttemptDriver`/`DriverStep`
(`src/native.rs:355`), public `ExperimentBinding::frozen().contract()`, and
`work_engine_review_preparation::fixture::consume_fixture_result` still public under that crate's `fixture`
(`review-preparation/src/fixture.rs:403`; S3-ACCEPTANCE `p3_carried[1]` second half), a fixture caller can deliver
a binding's contract bytes to the SDK adapter and consume the result with no gate. The packet allowed this
option, and inside review-delivery the consumer is reachable only through the gate (`consume` is
`pub(crate)`, `src/binding.rs:396`). But RESULT neither closes nor carries the `consume_fixture_result` half.
Fix: move `test_support` behind a separate non-default feature (not implied by `fixture`) and list the carried
P3 explicitly. Confidence: high.

**P3-4 — F-7 field coverage is two of four, and MATRIX row 2b overstates it.**
`sb_an_admission_for_another_subject_or_candidate_refuses_before_spawn` (`tests/native_vertical.rs:859-880`)
separates `subject` and `candidate_digest` only. No test separates `requirements_digest`. The
`realization_digest` branch (`src/native.rs:1430-1437`) cannot fail on the host path: `prepared_blockers` already
requires the binding's description to equal `realization_description(cfg)` (`:1396`), and the admission's
digest is built from that same description (`:1496-1497`). M5 removes the whole function, so it does not show
per-field adequacy. Executed fact: under M5 the other-subject delivery was not refused
(`sb/mutants/runs-1/M5-no-digest-comparison.test0.stdout`: panic "must be refused" at
`native_vertical.rs:423`), which confirms the comparison carries load as a unit. Fix: say so in MATRIX, or add a
requirements-only mismatch case if one is constructible.

**P3-5 — The supersession half of the finality test is vacuous.** `tests/gate.rs:282-295`: a superseded pin stays
superseded in `FixtureSource`, so S-A alone returns the same `Stale` on the second regate. Only the `Refused` half
(`:262-281`) separates finality from recomputation. M9 was killed at `gate.rs:281`, which is consistent with
that. Not a coverage gap, because the Refused half covers F-3. The comment "Supersession is final too" claims
more than the assertion shows.

**P3-6 — One S3 positive assertion was dropped without being stated.** The S3 test
`the_assumption_world_passes_only_by_labelled_assumed_discharges` asserted that the assumption world's
`realization_evaluation` has no `realization:`-prefixed check rows. Its replacement
(`tests/gate.rs:79-111`) asserts only that no blocker has kind `RealizationCheck`. A search for
`starts_with("realization:")` under `tests/` finds 0 hits. MATRIX says the S3 gate tests were rewritten with the
"same refusal assertions". This one was a positive assertion on the evaluation and is now absent. Fix: restore it
or record its removal.

**P3-7 — Unclaimed-list gaps (truthfulness).** RESULT "Explicitly unclaimed" is accurate as far as it goes: fixture
standing, assumed mapping kinds, O10 (epoch 1 at both boundaries, redelivery possible), R3 over declared values,
realization digest over the description and not the bytes, no hosted path. It omits:
(a) the request's organization id is caller-chosen and unanchored, and selects which vantage bindings O8 and R3
consult (`gate.rs:978`, `:1169`);
(b) until P2-1 is fixed, the vantage as well;
(c) the single time input (P3-2);
(d) the carried `consume_fixture_result` P3 (P3-3);
(e) the default-adapter realization digest is the same for every node/SDK root/control, which inherits S3
`p3_carried[0]` (name-level correspondence). The "description, not bytes" line covers this implicitly.
`FixtureAuthority`'s doc (`src/native.rs:1322-1329`) already states most of (a)/(c) for the source and request.

---

## Review questions

**1. Invariants at the subject.**
- Keyed on `CheckId`: yes, by construction (`src/gate.rs:274-281`; `Outcomes::passed(id)`). Reason subjects are
  not read.
- Host-obtained admission: yes (`HeldAdmission::admit` inside `deliver_and_consume_observed`, `:1501-1511`). Its
  request input is the caller's, guarded only as far as P2-1 allows.
- Mapping kinds assumed and labelled: yes (`:84-86`, `ASSUMED_DISCHARGE_LABEL`).
- RealizationCheck never dischargeable: yes (`:87`; `screen` refuses before admission, `:164-167`).
- Admission and F-7 before spawn, spawn 0 / consumer 0 on refusal: yes (`:1510-1522`). Tested with fixture root
  untouched (`native_vertical.rs:804-854`).
- Regate with a fresh observation: yes. `regate` takes a source, never an observation (`src/gate.rs:365-382`),
  and the host passes the same source (`native.rs:1549`). M6 (replayed observation) was killed by both mapped
  tests (custody record).
- F-3 finality: yes (`:370-380`). In the vertical each admission is regated once, so finality is exercised only
  by the direct test.
- Unknown fails closed: yes (`native.rs:1549-1551`; M8 killed).
- `Domain::Fixture` asserted: yes (`gate.rs:270`). The branch is unreachable because only `admit::<Fixture>` is
  called; this is disclosed in substance.
- Labels on success: yes (`Delivered`, `native.rs:1308-1323`; positive assertions `native_vertical.rs:253-326`).
- `run_attempt*` crate-private: yes, with the P3-3 caveat.
- `AssumedDischarges` private fields, `assess` `pub(crate)`, `bound_to` checked by `run_native`
  (`native.rs:1176-1182`): yes. `bound_to` is the frozen contract digest, so two bindings with the same
  preparation and transport but different episode inputs share it. That matches the packet's
  "binding/preparation".

**2. Decisions 1–7.**
- (1) Acceptable and well reasoned: a post-run admission would pin a replacement. Tested by
  `sb-between-superseded` → `Stale(IssuanceSuperseded)`.
- (2) Acceptable as a near-zero gap, with P3-1.
- (3) **Defect for the vantage (P2-1)**. Organization → P3-7.
- (4) Within the packet's option, nominal (P3-3).
- (5) Acceptable. `HeldAdmission` wraps public S-A `admit` and yields no discharge token. Its `regate` trusts the
  caller to pass the admission's own source. The host does.
- (6) Acceptable. `NotAdmitted` is unreachable through S-A and disclosed.
- (7) Acceptable. Disclosed, N-1 class.

**3. Bypass and stale routes.** Inside review-delivery, no route reaches the consumer except
`deliver_and_consume[_observed]`: `consume` is `pub(crate)`, `run_native` is private, and its only call site is
`native.rs:1522`. Across crates, P3-3 applies. Stale or revoked admission reaching consumption: revocation,
supersession, retraction (`Switch`) and Unknown all refuse (`native_vertical.rs:945-991`; M7 killed in runs-2).
Expiry is not representable (P3-2). Boundary-epoch supersession cannot arise (O10, disclosed).

**4. Labels and unclaimed list.** The labels are truthful (`FIXTURE_ADMISSION_LABEL`, `ASSUMED_DISCHARGE_LABEL`,
crate and module docs, `DERIVATION.md`'s fixture-standing statement for the acceptance's `provides`). The gaps are
listed in P3-7.

**5. Matrix and mutants.** Eleven mutants, n = 1 each. The records I opened match the claims: the M5 stdout, the
M10 doctest record (`native.rs` line 25 compiled under the mutant), and the M6/M5 edit specs. The discharge
table is pinned by an exact-table test, and the positive test uses `discharge` as its own oracle. That is
acceptable because the table test pins it. Gaps: P3-4, P3-5, P3-6. S1/S3 coverage is otherwise kept: the root
gate passed all four crates plus S-A, default features, authority-store and native ×3. Two S3 gate tests were
replaced by design-equivalent ones.

## Disposition

To reach `accept_private_readiness`, the minimum change is P2-1 (vantage comparison + test + mutant). The P3s can
be carried, though P3-3's carried-P3 line and P3-7's unclaimed additions are cheap truthfulness fixes for
RESULT.md.
