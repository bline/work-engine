changes_required

# S-A independent review 1: pure organizational admission core

Reviewer: separate Claude Code session (model reported to me as Fable 5.2, `claude-fable-5-2`; effort as
configured by the launcher, not observed by me). Same provider family as the builder. Read-only except this
file. Parent, collector and acceptance authority: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`.

Verdict in one paragraph: **no P1 and no code defect that admits wrongly was found.** The crate is pure, the
nine checks are evaluated without short circuit, production admission is unreachable, and the five states
behave as the approved table says for the cases tested. The verdict is `changes_required` for one P2:
the decision-scope containment checks that Amendment 6.1 F4 defines (problem scope, actions, boundaries) and
several other fail-closed branches have **no test at all**, while `RESULT.md` says tests exercise them. The
remediation is tests and claim corrections, not redesign. Eleven P3 items and the rulings on the builder's
design choices follow.

## 1. Subject and what was checked

| Item | Value | How established |
|---|---|---|
| Checkpoint / tree / patch | `1b8bf301…fcde` / `5e6390fc…0b8b` / `08d0ca29…d7a2` vs `561c6ffc` | from the packet; tree and patch digests **not recomputed by me** (`unverified`) |
| Diff shape | 11 files, 3,515 insertions: new crate (9 files), `rust/Cargo.toml` +1, `rust/Cargo.lock` +10; `work-engine-organization` not in the diff | executed by me: `git diff --stat 561c6ffc 1b8bf301` |
| Source read | export `…/sa/export/rust/crates/work-engine-organization-authority`: all 6 `src` files, both test files, `Cargo.toml`, in full | read |
| Mutant evidence binds to this source | `sha256(gate.rs)` in the export = `873b16a8…2e79` = `summary.json` `source_sha256` | executed by me (`sha256sum`) |
| Root gate | 7/7 steps outcome 0; `test-all` = 1 + 29 + 1 + 3 = 34 passed | read from `sa/root-gate/summary.json` and `test-all.stdout`; not re-run |
| Mutants | 18 of 18 records show `test result: FAILED` from a test panic; no compile error among them | read from `records-20261117T111045Z/*/*.stdout` (18 files with the line) |

Contract read: builder packet, both owner decisions, root addendum, proposal (whole file), rev6 §4.3 and §18,
6.1 §3–§6, and lines 12–31 of the R2REC-approved `org-r2-reconciliation-20261117/ROOT-SUMMARY.md` (to settle
design choice 1). Organization-crate types were checked by targeted grep (`derive.rs`, `schema.rs`,
`enforce.rs`), not read in full.

Process facts: the AGENTS.md board read was **denied by the session permission mode**, so board state is
`not established`. No context listener (packet: none). No identity binding message was received. My own cost
and usage are `not established` (the parent observes them). No build, test or git mutation was run.

Everything below marked *static* is reasoning from the source; *executed* means I ran or read a record.

## 2. Findings

| # | Sev | Finding | Confidence |
|---|---|---|---|
| F-1 | **P2** | 6.1 F4 containment and other fail-closed branches are untested; `RESULT.md` says they are exercised | high |
| F-2 | P3 | One observation decides any number of admissions and can be replayed at the point check | high |
| F-3 | P3 | `Refused` (and boundary `Stale`) is documented terminal but can return to `Current` | high |
| F-4 | P3 | `RealizationAcceptance.profile` is carried but never compared with the candidate's profile | medium |
| F-5 | P3 | O4 resolves a named principal without the exactly-one rule that O8 applies | medium |
| F-6 | P3 | R1/R2 class lookup takes the first class of a dimension (latent; unreachable in v1) | high |
| F-7 | P3 | O3 compares two caller inputs; no authoritative anchor exists in this slice and that is not stated | high |
| F-8 | P3 | Domain is a runtime field; no test asserts it; future non-fixture consumers must check it | high |
| F-9 | P3 | `AdmissionState` is a plain public enum; `Current` is constructible by any caller | high |
| F-10 | P3 | Reason subjects are not a stable key (double prefix, detail embedded, per-domain shapes) | high |
| F-11 | P3 | O9 wiring in `evaluate` and "no production constructor" rest on static reading, not a test | high |
| F-12 | P3 | Stale conditions and one oracle rule from the approved text are not covered and not listed as unclaimed | medium |

### F-1 (P2). Decision-scope containment and other refusal branches have no test

Executed: a search of the crate's complete test set for twelve reason codes returned **zero** references.
The test set is `tests/admission.rs`, `tests/base_candidate_shape.rs` (2 files, both searched), the one unit
test in `gate.rs:1301-1328` (read; `o9_reasons` only) and 3 doctests in `lib.rs` (read; not-`Clone` only). The
other four `src` files contain no tests (read in full). The twelve, of 30 `ReasonCode` variants:

`AuthorityInsufficientProblemScope`, `AuthorityInsufficientActions`, `AuthorityInsufficientBoundaries`,
`AuthorityMissing`, `AuthorityRevoked`, `AuthoritySuperseded`, `ComponentBindingAmbiguous`,
`RealizationDigestMismatch`, `VantageUnknown`, `VantageUnbound`, `VantageBindingAmbiguous`,
`RealizationAcceptanceAmbiguous`.

Why it matters:

- 6.1 §4 (F4) defines exactly three containment checks for a decision scope. They are implemented at
  `gate.rs:713-718` and `740-745` (problem scope), `746-748` (actions) and `749-753` (boundaries). Every test
  world uses one matching problem scope (`admission.rs:147-149, 172`), a decision scope whose actions include
  every permission (`:207-218`), and a requirement with `"scope":{}` (`:63`), so `requirement.boundaries` is
  `None` in all 29 tests. Static: replacing any of those three conditions with `false` changes no test result.
  The boundaries branch is never entered, positively or negatively.
- §18.3 names the realization-digest comparison as R3 trust evidence (`gate.rs:1154-1156`). No test varies it.
- These checks are what S-B will rely on to discharge `authority-requirement:*` (O4) and `separation:*` (O8+R3).

Claim correction required: `RESULT.md:112-113` says "No recorded mutant for the `RevisionConflict` /
`IssuanceRetracted` branches or the other O4 sub-reasons (tests exercise them)". For insufficient
problem scope, actions, boundaries and superseded grant that is wrong: no test exercises them.
`MATRIX.md:44-47` lists them as "not mutated individually" without saying they are also untested.

Required: negative tests (with exact failure sets) for the three F4 fields, including one requirement that
names boundaries with a covering and a non-covering scope; `AuthorityMissing`; R3 realization-digest mismatch;
at least one recorded mutant in the containment family. Recommended in the same pass: unbound or ambiguous
vantage binding, ambiguous component binding, ambiguous realization acceptance, unknown vantage. Then correct
the two documents.

### F-2 (P3). Observation reuse within one boundary

`admit` takes `&SealedObservation` (`gate.rs:394-400`), and `admission_state` takes
`&Result<SealedObservation, Unobserved>` (`:1236-1240`). Static consequences:

1. One observation can decide any number of requests at its boundary (different vantage or subject).
2. The observation used for `admit` can be passed again to `admission_state`. Boundary and revision match, the
   pins are committed in that same state, the regate passes, and the result is `Current`. A revocation
   committed after admission is not seen. The doc says "point check against a new observation" (`:1233`);
   nothing makes it new.

The packet's stated negative (same observation, later boundary) is implemented and tested, so this does not
fail a packet outcome, and 6.1 F3 says type-level limits are ergonomics with enforcement at the host. It is a
host obligation S-B must carry: ROOT-SUMMARY l.24 already requires "regate before consumption against a
fresh fixture observation", and a Q2(b)-style test through the real host would catch a replay. Cheap
structural option for the root: have `admit` consume the observation by value, so a point check needs
another `observe_for`.

Also static: `reserve` is a free function over a boundary value (`observation.rs:143-145`), so "reservation"
means "names a boundary"; allocation and single use are entirely the host's (stated in the docs; O10 gap).

### F-3 (P3). Terminal states are not terminal in the API

The state table says `Refused` is "terminal for this request" (`gate.rs:1213`; proposal §4.4).
`admission_state(&self, …)` keeps no memory. Static example from the builder's own test
(`admission.rs:1107-1124`): a duplicate decision scope makes the state `Refused`; revoking the duplicate makes
the same admission `Current` again, because the duplicate was never pinned. `Stale` from a boundary mismatch
likewise depends only on which observation the caller passes. `Revoked` and `Stale(IssuanceSuperseded)` are
sticky only through source obligations 2 and 3. A pure crate cannot hold this state, so the consequence
"drop the admission on the first non-current result, except `Unknown`" must be written into the S-B packet
and tested there.

### F-4 (P3, ruling). `profile` of a realization acceptance is unchecked

`r4` matches on the digest only (`gate.rs:983-990`) and copies `record.profile` into the admission
(`:998`, `:425`). Nothing compares it with `request.requirements.profile()` (whose string form is
`fixed-slice-review-v1`, `schema.rs:26-29`). Rev6 §18.5 defines the field and states only the digest check, so
this is room the spec left, not a contradiction. An acceptance attested for another profile would admit.
With one profile in v1 the consequence today is nil. Root ruling requested: either R4 requires equality (one
comparison and one test), or the field is declared display-only.

### F-5 (P3). Named-principal resolution differs between O4 and O8

O8 resolves the acceptor through `principal_named`, which requires exactly one committed principal with that
id (`gate.rs:611-627`, `931-941`). O4's `PrincipalMatch::Named` accepts a grant held by **any** principal
issuance carrying that id (`:683-689`). With two committed principals sharing an id and different anchors, O4
passes on a grant to either while O8 would call the same name unresolved. 6.1 F4 applies exactly-one to
decision scopes; rev6 does not state it for principals. Suggest O4 use the same resolution.

### F-6 (P3, latent). First class of a dimension wins

`constraint_reasons` uses `provides.classes.iter().find(|c| c.dimension() == dimension)` (`gate.rs:1015-1016`).
If an acceptance stated two classes of one dimension, the result would depend on set order and could pass
with a disallowed class also stated. Unreachable today: v1 has one class per dimension (`enforce.rs`
`mechanism_of` is exhaustive over `Anthropic` and `GuardedSandbox`). For the same reason R1's
`RealizationIncompatible` branch (`:1020-1022`) cannot be reached or tested in v1. Fail-closed form when the
vocabulary grows: every stated class of the dimension must be allowed.

### F-7 (P3). O3 has no authoritative anchor in this slice

Rev6 O3 compares the recomputed candidate with the digest in the committed organization **admission record**.
O1 and O2 are excluded (addendum item 9), so here `candidate_digest` and `requirements` both come from the
caller (`gate.rs:219-243`, `444-464`). O3 therefore shows the request is self-consistent. It cannot detect
substitution against anything the owner committed, and every other check runs over caller-supplied
requirements, tied to records only by problem scope, principal ids, organization id and vantage name. This
follows from the approved exclusion, but `RESULT.md` "Explicitly unclaimed" does not say it, and the doc
comment "the admitted candidate" (`:32`, `:228`) reads as more. S-B must compare the admission's candidate and
requirements digests with its binding (proposal §4.5 item 1).

### F-8 (P3). Domain separation is a runtime field behind a Cargo feature

`EffectAdmission` is non-generic and records `Domain` (`gate.rs:316-329`, choice 13). The proposal's Q10
barrier was type-level (`EffectAdmission<P>`, compile-fail). Addendum guidance F-9 allowed dropping the
uninhabited production instantiation, so this is within guidance. Consequences to record: (a) no test asserts
`admission.domain()` (executed: zero matches for `domain()` or `Domain::` in `tests/`); (b) Cargo unifies
features, so any crate that enables `fixture` enables the fixture policy for the whole build, and a future
non-fixture consumer must match on `Domain` or the type must become generic before such a consumer exists.

### F-9 (P3). `AdmissionState::Current` is constructible

`AdmissionState` is a public enum (`gate.rs:1216-1223`) and `ObservationRevision` is `Copy`, so
`AdmissionState::Current { revision: admission.revision() }` can be written by any caller and
`permits_effect()` returns true for it. `Outcomes` and `EffectAdmission` are not forgeable (private fields).
A host must take the state only from its own call. Ergonomics, per 6.1 F3.

### F-10 (P3). Reason subjects are not a stable key

Executed (test expectations): production O4 emits `component:component:campaign` (`admission.rs:434`);
`PrincipalNotCurrent` puts the detail inside the subject, `authority:auth-1: revoked` (`:810`); O8 uses
`separation:<cond>` in production (`:454`) and `<cond>/<vantage>` or `<cond>/principal:<id>` in fixture
(`:756`, `:818`). S-B should key discharges on `CheckId` only and treat subjects as diagnostics.

### F-11 (P3). Two properties rest on static reading

- O9: only the helper `o9_reasons` is tested and mutated. The wiring that feeds it (`gate.rs:467-477`: the
  `InternalError` filter and `view(c).err()`) has no test; dropping either would pass the suite (static). The
  builder states the negatives are unreachable from valid requirements; I did not check whether the
  organization crate offers a test hook (`not established`).
- "No production constructor": the three doctests show not-`Clone`. Non-constructibility is by private or
  `pub(crate)` fields and by `AuthorityState::new` being `pub(crate)` and called only from the cfg-gated
  fixture source (static, all `src` read). No compile-fail test asserts it. `MATRIX.md` row 2 cites the
  doctests for more than they show.

### F-12 (P3). Approved text not covered and not listed

- Proposal §4.4 lists five stale conditions. The crate represents two (boundary; superseded pin). "Episode
  revision moved", "realization marked stale" and "admission head superseded" are host or episode-owner
  facts and are not in `RESULT.md` "Unfinished" or "Explicitly unclaimed" (only the uncertain effect commit is).
- 6.1 §6 masking control: "every negative test asserts specific reason codes". One test asserts check ids only
  (`admission.rs:1120`).

## 3. Packet outcomes 1–7 and purity

| Outcome | Assessment |
|---|---|
| 1 Records, F4/F5, identities, no production constructor | Met by static reading. `DecisionScope` has no `decisions` field (`records.rs:119-125`). `IssuanceId` and `BodyDigest` are distinct types; the revoke/reissue test is real. F4's **checks** are untested (F-1). |
| 2 Observation trait, four obligations, fixture source | Met. Obligations are contract text (`observation.rs:100-126`), as claimed. `Unknown` injection works by returning `Unobserved`. |
| 3 Reservation seal, later-boundary refusal | Met as worded (`gate.rs:401-406`; test and mutant). Limits in F-2. |
| 4 Gate O subset and Gate R, outcome per check id | Met: nine ids always present (`:548-556`), no short circuit (mutant killed). Production refusal exact for the one v1 candidate shape tested (n = 1). |
| 5 Admission and five states, outcomes by id | Met for the tested cases; consequences are documentation plus `permits_effect` (F-3, F-9). |
| 6 Oracle classes | Used and named. One masking-control gap (F-12). |
| 7 Exclusions | Respected (executed: diff touches only the new crate, workspace list and lock). |
| Purity | Met (static, all six `src` files read): imports are `std::fmt`, `std::collections`, `std::cell::RefCell` (fixture), `sha2`, `thiserror`, `work_engine_organization`. Time is the `now_millis` argument. Purity of `work-engine-organization` itself was not re-checked. |

Production unreachable-positive (static, high confidence): without `fixture` no `AuthorityState` can be built,
so no `SealedObservation` exists and `admit` cannot be called. With `fixture`, `admit::<Production>` always
fails R3 (`gate.rs:1120-1126`), independent of the candidate; `Policy` is sealed (`:172-179`). For v1 candidates
the refusal is over-determined (O4 components, O5, O8, R3).

Check-by-check against rev6 §4.3/§18 and proposal §4: O3, O4 (grant, decision scope, component binding), O5,
O8, O9, R1–R4 match the cited text by static reading, subject to F-4 to F-7. Fail-closed handling of a missing
candidate (`:483-494`), no observation, regressed or conflicting revision and a retracted pin
(`:1241-1279`) is correct.

## 4. Builder design choices: what needs a ruling

| # | Choice | Changes approved meaning? | Ruling |
|---|---|---|---|
| 1 | Gate R and `EffectAdmission` in this crate | Differs from the wording of O1. The owner-approved R2REC subject (ROOT-SUMMARY l.18-19) places "Gate R R1–R4, five states" in the S-A pure crate, so placement is covered. Not covered explicitly: this crate minting the value its doc calls "the only value that permits an effect" (`gate.rs:313`). | **Root**: record that R2REC supersedes O1's placement sentence, and that the host still decides the effect. No owner ruling needed unless the root reads O1 otherwise. |
| 2 | Seal = boundary value | No. See F-2 for limits. | Root note |
| 3 | O3 recomputes `derive` | No (rev6 requires it). See F-7. | None |
| 4 | `candidate_unavailable` for checks without a candidate | No; fail closed. | None |
| 5 | Problem scope by value, not digest | Departs from 6.1 F4's `problemScope{digest}` in form; equality by value is not weaker. | Root note; revisit when a codec exists |
| 6 | Every covering grant pinned | Conservative. Revoking one of two covering grants gives `Revoked` although a regate would pass; 6.1 F3 step 4 alone would commit. Matches proposal §4.4's revoked row. | **Root**: accept as the fail-closed reading |
| 7 | Superseded → stale; missing pin → unknown | Superseded status is packet-required. Consistent. | None |
| 8 | Regate on every state query | No. Makes time-only expiry visible at one revision, as rev6's `now` input implies. | None |
| 9 | No `FixtureSeparationAttestation` type | Simpler than rev6. The fixture arm exists only under the feature; the production arm refuses every `none` row. | Root note |
| 10 | Installed-digest match in R3 | Follows rev6 §18.2; differs from proposal §4.3/Q8 (O4). Consequence: R3 mixes actor and component evidence, so a component digest mismatch also blocks any discharge that needs R3. | **Root**: confirm ComponentBinding → O4+R3 for S-B |
| 11 | `RealizationAcceptance.provides` | Extends rev6 §18.5. Supported by the approved proposal §4.3 ("the accepted realization record states the value"). It changes what the owner attests when accepting a realization. | **Root** now (fixture standing); **owner** when a production record schema is approved (§18.6) |
| 12 | Grant `boundaries`/`supersedes` dropped | Consistent with 6.1 F4. Rev6 §3.3 was outside my read list (`not established`). | Root note |
| 13 | Production R3 always refuses; non-generic admission | Within F-9 guidance. See F-8. | Root note for S-B |
| 14 | No serde | No. | None |
| 15 | Crate-private constructors for state and issuance ids | No external crate can implement the trait usefully, so a future source must live here or the constructors must open. | Root note for the storage decision |

## 5. Over-built or missing for S-B

Over-built (small; nothing here warrants removal on its own):

- `Production` policy: its only caller is the P-oracle test, because `admit` is uncallable without `fixture`.
  The packet required it; noted against earlier guidance F-9.
- `o8_bindings` (`gate.rs:481`, `1132-1142`): O8 and R3 both require exactly one committed binding per
  `(organization, vantage)`, so the map can never select a different binding than the fallback.
- `StaleReason::BoundarySuperseded` versus `ObservationBoundaryMismatch` and `Boundary::supersedes`: labels only.

Sufficient for S-B: `CheckId`, `Outcomes::{get, passed, refused, iter}`, `AdmissionRefusal::Checks`, and
admission accessors for subject, vantage, candidate and requirements digests, realization digest, boundary,
revision and pins. No dependency on review-preparation (executed: `Cargo.toml`, lock entry). Because an
admission exists only when all nine checks pass, a discharge map derived from an admission is all-or-nothing;
per-check outcomes matter for refusals and for the kinds with no check here.

For the S-B packet (obligations this crate cannot carry):

1. Take a new observation for the regate; never pass the admission-time one (F-2).
2. Treat the first non-current state other than `Unknown` as final for that admission (F-3).
3. Compare candidate, requirements, subject and realization digests with the binding (F-7). The obligation is
   reachable through the candidate's conditions (`vantage` ↔ `obligation`), not from the admission directly.
4. Assert and label `Domain::Fixture` (F-8); key on `CheckId`, not subjects (F-10).
5. The accepted record's `provides` values have no public accessor and the host's realization claim is not a
   gate input, so proposal Q3's "claim differs from the accepted record" has no check here. ROOT-SUMMARY l.24
   keeps realization checks undischarged, which covers it for now.

## 6. Matrix and mutants

- Executed: 18 of 18 mutants killed by a test assertion, restored exactly, against the same `gate.rs` bytes as
  the export. The packet's minimum list (seal, later boundary, one per check family, each non-current state,
  unknown, production refusal) is met.
- Not vacuous: `assert_exact` requires every unlisted check to pass, so G1 and G2 tests discriminate; the
  doctests pin error `E0599`.
- Gaps: F-1 (no test, hence no mutant, for the containment family, R3 digest, ambiguity refusals); F-11 (O9
  wiring); one ids-only assertion (F-12). All positive evidence is on one base candidate shape (n = 1: two
  conditions, one obligation, one authority requirement without boundaries, two component bindings).
- Static, not run: a mutant disabling `RevisionConflict` would still be killed (the pins go missing and the
  test expects `RevisionConflict`); production O4/O8 arms have no mutant but the exact-set P test would fail.

## 7. Required before private readiness

1. Tests and at least one mutant per F-1; correct `RESULT.md:112-113` and `MATRIX.md:44-47`.
2. Root rulings on choices 1, 6, 10, 11 and on F-4; record F-2, F-3, F-7, F-8 as S-B obligations.
3. Add F-7 and F-12's unrepresented stale conditions to "Explicitly unclaimed".

Optional, root's call: `admit` consumes the observation (F-2); R4 profile equality (F-4); exactly-one principal
in O4 (F-5); a compile-fail test for non-constructibility (F-11).

## 8. Not checked

Tree and patch digest recomputation; `sa/runs/final` (builder runs; I read the root gate instead); the 18
superseded first-run mutant directories; rev6 §3.3; `s2/review/REVIEW.md` beyond F-9/F-10; full source of
`work-engine-organization`; the board. No forbidden path was read.
