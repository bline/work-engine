# S-A result: pure organizational admission core

Status: **map_ready** (worker completion; acceptance, readiness and integration are the root's).

**[R1] Remediation 1 (2026-11-17)** was applied to the working tree after independent review 1
(`sa/review/REVIEW-1.md`, `changes_required`) under `sa/ROOT-RULINGS-1.md`; its record is
`sa/remediation-1/RESULT.md`. Passages below that REVIEW-1 found wrong or incomplete are corrected
in place and marked **[R1]**; the run, test-count and mutant figures in the unmarked sections
describe the checkpoint `1b8bf301` (before R1) and are superseded by the R1 record where they differ.
Packet: the S-A builder brief received in this session (`sa/PACKET.md` exists; I did not compare it
with the brief, so their identity is unverified).
Builder: Claude Code, requested model Opus 5.5 (`claude-opus-5-5`); cost/usage not observed by me
(`not established`; the parent observes it).

## Entry

- Worktree `/home/bline/cbm-work/wt-org-r2-core-20261117`, branch `org-r2-core-20261117`,
  HEAD `561c6ffc301f9faac7609ad07e8a9704dd379e0b`, `git status --porcelain` empty (0 lines) at entry.
- No commit, stage, branch or other git mutation was made.

## Files

| File | Reason |
|---|---|
| `rust/Cargo.toml` (+1) | workspace member |
| `rust/Cargo.lock` (+10) | the new crate's lock entry only (deps: serde_json[dev], sha2, thiserror, work-engine-organization). It was written by my first unlocked `cargo build --offline` during development, **not** by a recorded custody run; every recorded run afterwards used `--locked` and passed. |
| `crates/work-engine-organization-authority/Cargo.toml` | new crate; `fixture` feature (non-default); no I/O dependency |
| `src/lib.rs` (69) | crate contract, re-exports, 3 compile-fail doctests (not `Clone`) |
| `src/ids.rs` (206) | `IssuanceId`, `BodyDigest`, `ObservationRevision` (sequence + state digest), `Boundary{operation, kind, epoch}`, `RealizationDigest`, `ExecutableDigest`, hashing encoder |
| `src/records.rs` (391) | rev6 §18 records with 6.1 F4/F5; fixture constructors only under `fixture` |
| `src/observation.rs` (179) | `AuthorityObservation` trait with the four source obligations; `AuthorityState`; `reserve`, `observe_for`, `SealedObservation` |
| `src/fixture.rs` (107) | `FixtureSource`: issue / revoke / supersede / `inject_unknown` |
| `src/gate.rs` (1328) | `CheckId`, reasons, `Outcomes`; `Policy` (`Production`, `Fixture`); O3, O4, O5, O8, O9, R1–R4; `admit`; `EffectAdmission`; `admission_state` |
| `tests/admission.rs` | 29 fixture tests named by oracle class at the checkpoint (1124 lines); **[R1]** 43 after remediation 1 |
| `tests/probe_tmp.rs` (77) | shape preconditions of the base candidate. **[R1]** The checkpoint `1b8bf301` contains this file as `tests/base_candidate_shape.rs` (75 lines, 1 test); the rename was made after this text was written and by whom is `not established`. |

`git diff --stat`: `rust/Cargo.lock | 10 ++++++++++`, `rust/Cargo.toml | 1 +` (2 files, 11 insertions);
plus the 9 new files above (untracked). `work-engine-organization` is unchanged.

## Runs (custody, `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`)

`sa/runs/final/` (plan `sa/runs/plan.json`), summary sha256
`780df77f4eb22b0c773fa78a7f9877977801a99440ecac3d7550014c00fed103`, **6/6 exit 0**, leftovers 0:

| Step | Result |
|---|---|
| `cargo fmt --check -p work-engine-organization-authority` | ok |
| clippy `--all-targets --all-features -D warnings` (`--locked`) | ok |
| clippy `--all-targets -D warnings` default features (extra) | ok |
| `cargo test --offline --locked -p …-authority --all-features` | lib 1, admission 29, probe 1, doctests 3: all pass |
| same, default features | lib 1, admission 0 (file is `cfg(feature = "fixture")`), probe 1, doctests 3: all pass |
| `cargo test --offline --locked -p work-engine-organization` | pass (crate unchanged) |

Mutants: 18, each applied alone, mapped test run through custody, source restored with sha256
verification: **18/18 killed by a test assertion** (`sa/mutants/summary.json`,
`sa/mutants/records-20261117T111045Z/`). See `sa/MATRIX.md`.

## Matrix

`/home/bline/cbm-work/org-r2-core-20261117/sa/MATRIX.md`: every packet outcome mapped to tests and mutants.

## Design choices where the specs left room

1. **One `admit`, two internal gates.** O3–O9 and R1–R4 are separate functions; `admit::<P>` runs
   both over the same sealed observation so the outcome vector is complete (9 ids) even when
   Gate O refuses. There is no public `AdmittedOrganization`. Note: owner decision O1 placed
   Gate R and effect admission with the consuming host; the later R2REC packet puts R1–R4 and
   `EffectAdmission` in this crate. I followed the packet; the host still owns epoch allocation,
   single use and the effect commit. **[R1]** Root ruling (choice 1): the operator-approved R2REC
   subject places Gate R R1-R4 and the five states in S-A and supersedes O1's placement sentence.
   The `EffectAdmission` doc no longer says "the only value that permits an effect"; it says it is
   the only value that *can* permit an effect, and only when the consuming host's own regate
   against a fresh observation also passes.
2. **Seal = boundary value.** `reserve(Boundary)` returns a non-`Clone` `Reservation`;
   `observe_for` copies its boundary into the observation; `admit` refuses a different boundary
   with `ObservationBoundaryMismatch` and **no check outcomes** (it is not an observation for that
   decision). The crate allocates no epochs and records no consumption (O10 gap stays).
3. **O3 recomputes `derive`.** The request carries the requirements and the admitted candidate's
   digest; the gate derives and uses its own candidate for every other check. This uses
   `derive` in addition to `view`/`enforceability` (rev6 §4.3 O3 requires it); the organization
   crate is unchanged.
4. **Checks without a candidate** (O4, O5, O8, R1, R2, and O9 if no internal error) refuse with
   `candidate_unavailable` rather than being omitted (fail closed, G2-testable).
5. **Problem scope by value** (`ProblemScopeRef{run_id, plan_lineage, slice_number}`), because no
   problem-scope digest codec exists; 6.1 F4 says `problemScope{digest}`.
6. **Grant coverage pins every covering grant**; revoking any pinned grant makes the admission
   revoked even if another grant still covers (conservative).
7. **Statuses.** `Committed`, `Revoked`, `Superseded{by}`, `Missing`. Superseded pin ⇒ `Stale`;
   revoked pin ⇒ `Revoked`; missing pin at a later revision, lower sequence, or same sequence with
   another digest ⇒ `Unknown` (source-obligation violation, fail closed).
8. **`admission_state` always regates** (time is an input, so expiry can change the outcome at
   one revision); a regate refusal is `Refused` with every outcome (proposal §4.4 table), not stale.
9. **Fixture O5 attestation** = the fixture policy accepts only `separation` `none` rows (decided by
   O8 over fixture records); every other `none` row refuses in both domains. No separate
   `FixtureSeparationAttestation` type; the `Domain::Fixture` variant and all fixture branches are
   `cfg(feature = "fixture")`, so default builds contain only the production refusals.
10. **Installed-digest match is in R3** (rev6 §18.2 puts it at Gate R), not O4 as proposal Q8 says.
11. **`RealizationAcceptance` gains `provides`** (classes and the five normative properties the
    accepting owner states). R1/R2 read values only from the committed record (proposal §4.3:
    "the accepted realization record states the value; a claim alone never discharges"). This
    extends the rev6 §18.5 schema. R1 checks class membership; mechanism-row existence is O5's.
    R2 does not check `independence` (separation's concern) or `capabilities` (empty by validation).
12. **Grant schema**: grant-level `boundaries` and `supersedes` dropped; boundaries come from the
    decision scope (6.1 F4), supersession from the source status.
13. **Production admission is structurally unreachable**: production R3 always refuses
    `binding_contract_unqualified` (O8: needs host observation of the spawned process). Hence
    `EffectAdmission` is non-generic (review F-9) and records its `Domain`.
14. **No serde**: nothing is serializable (rev6: not `Serialize`). Digests use a crate-domain,
    length-prefixed SHA-256 encoding, which is not a storage codec.
15. **`AuthorityState` and `IssuanceId` have crate-private constructors**, so only an in-crate
    source can implement the trait until the storage decision defines one.

Suggested discharge map for S-B (S-B owns the function; not decided here): Separation → O8+R3;
ComponentBinding → O4+R3; AuthorityRequirement → O4; RealizationAdmission → R1+R2+R4;
ObserverIdentity → R2 (observerClass) + R3. Mapping kinds and RealizationCheck: no check here.

## Unfinished / not done

- O9's negatives are unreachable from valid requirements (derive/view fail only on their own
  defects; `ViewError`/`Unresolved` are `non_exhaustive`), so O9 is tested at function level only.
- "Effect-commit outcome uncertain ⇒ unknown" is a host fact; not representable in this crate.
- **[R1] Correction (REVIEW-1 F-1).** The checkpoint text here said "No recorded mutant for the
  `RevisionConflict` / `IssuanceRetracted` branches or the other O4 sub-reasons (tests exercise
  them)". For the O4 sub-reasons that was wrong: at the checkpoint no test exercised insufficient
  problem scope, actions or boundaries, a superseded grant or a missing grant (and none exercised
  `ComponentBindingAmbiguous`, the R3 realization-digest mismatch, `VantageUnknown`, `VantageUnbound`,
  `VantageBindingAmbiguous` or `RealizationAcceptanceAmbiguous`). The error class was a claim about
  test coverage made without running the check that would show it wrong (replace each condition
  with `false` and see whether any test changes). Remediation 1 adds the missing tests and records
  mutants for the containment conditions; see `sa/remediation-1/RESULT.md` and `sa/MATRIX.md`.
  `RevisionConflict` / `IssuanceRetracted` remain tested without a recorded mutant of their own
  (a mutant of the first was judged killed by static reading in REVIEW-1 §6; not run).
- Housekeeping I could not do (deletion denied): `tests/probe_tmp.rs` name; superseded first-run
  mutant directories `sa/mutants/<name>/` (the valid records are in `records-20261117T111045Z/`).
- Coordination: the board read (`codex_chatboard.py read`) was denied as part of a compound
  command and not retried alone; no context listener was set up (packet: the parent observes
  context/budget); no identity binding message was received, so none is acknowledged.

## Explicitly unclaimed

- Standing is **fixture** only: records, source and policy are synthetic; positive admissions
  exist only under the `fixture` feature.
- No storage, publication, issuance, installation, production admission, lifecycle integration
  or review-delivery change.
- The four observation-source obligations are **contract text** (trait docs), not implemented by
  any real source; the fixture source meets them by construction only.
- R3 compares **declared** actor values (O8), not an observation of who ran.
- Excluded per packet: `GrantProjection`, `realization_digest` definition, O1/O2/O6/O7.
- Not reviewed independently. **[R1]** Reviewed once (REVIEW-1, `changes_required`); the remediation
  itself has not been re-reviewed.
- **[R1] O3 is request self-consistency only (REVIEW-1 F-7).** `candidate_digest` and `requirements`
  both come from the caller. With O1/O2 excluded no committed admission anchor exists in this slice,
  so O3 cannot detect substitution against anything the owner committed, and every other check runs
  over the caller's requirements, tied to records only by problem scope, principal ids,
  organization id and vantage name. The consumer (S-B) compares the admission's candidate,
  requirements, subject and realization digests with its binding.
- **[R1] Stale conditions not represented (REVIEW-1 F-12).** Proposal §4.4 lists five stale
  conditions; this crate represents two (boundary superseded; pinned issuance superseded). "Episode
  revision moved", "realization marked stale" and "admission head superseded" are host or
  episode-owner facts not represented here.
- **[R1] Items recorded as S-B obligations by the root, not carried by this crate:** a fresh
  observation for the regate (F-2), the first non-current state other than `Unknown` is final for
  that admission (F-3), digest comparison with the binding (F-7), assert and label
  `Domain::Fixture` (F-8), key discharges on `CheckId` not on reason subjects (F-10). Carried as P3
  and not taken: F-6 (first class of a dimension wins; latent, unreachable in v1), F-9
  (`AdmissionState::Current` is constructible), the `o8_bindings` simplification.
