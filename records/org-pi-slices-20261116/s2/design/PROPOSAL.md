# S2 / R2: Lawful organizational and realization admission — design proposal

Status: **proposal for owner approval. Not authority, not implementation.** Written 2026-11-16 by the S2
architect/planner worker for Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`, read-only, against
private checkpoint `1cc771146c72ff7d23b371b730aa5e73ce79db61` (worktree
`/home/bline/cbm-work/wt-p1-vertical-20261113`; the packet states that the working tree equals it. I did not
re-verify that with git). Governing plan: `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md` R2
("Exit: a separately approved admission contract and its discriminating qualification").

Process notes: the AGENTS.md board read was attempted and **denied by the session's permission mode**, so the
board state is `not established` for this proposal. Context and budget observation is owned by the parent,
as the packet states. No listener was started and no children were launched.

---

## 0. Summary and recommendation

R1 (S1) connected one compiled contract to a provider-disabled Pi session and to the implementation-review/v1
consumer. Its accepted semantic limit is that the vertical **never consults `blockers` or
`realization_evaluation`**: a realization mismatch is carried as text and never refuses delivery or
consumption (`S1-ACCEPTANCE.json` `semantic_limit`). R2 has to supply the authority under which those blockers
can be discharged, and it has to make undischarged blockers refuse.

Recommended design, in one paragraph:

> Keep three things separate. Candidate derivation stays pure in `work-engine-organization`. Authority
> publication becomes an observation contract served by the organizational authority owner. Its storage is
> not decided here. Per-effect admission happens at the consuming host. Adopt Stage 0 rev6 §18's
> owner-attested **principal / vantage-actor / component binding** records, with Amendment 6.1's F4/F5
> corrections, as the separation and binding mechanism. Define one admission contract with the states
> **current, stale, revoked, unknown and refused**. It binds principal, actor, subject, operation and
> boundary, the authority revision and the concrete realization. Under it, every preparation blocker becomes
> an obligation that an admission check must explicitly discharge before delivery, and again before
> consumption. Qualify positively **only in the fixture domain**. The production gate keeps refusing
> (`binding_contract_unqualified`, v1 O5) until the owner separately approves a mechanism-table revision, a
> publication mechanism and record issuance.

Two implementation slices follow (§6). R2-A is the pure admission core. R2-B wires it into the R1 consumer
seam. Rough total: **$48–69 API-equivalent**, plus root coordination.

---

## 1. Gate O / Gate R: what I found and did not find

| Item | Location | Standing |
|---|---|---|
| Stage 0 contract rev6: Gate O (O1–O9), Gate R (R1–R4), `DispatchAuthorization`, `AuthorizationBinding` | `/home/bline/cbm-work/org-rev6-20261112/STAGE0-CONTRACT-r6.md` §4.3 (lines 577–728) | Its own status line (l.3): "design proposal for operator review. Not canonical, not committed, not implemented" |
| Grant states and failure vocabulary (`authority_missing/revoked/insufficient/scope_unresolved/unverified`) | same, §3.3 (l.372–424) | same |
| Identity, binding, separation and decision-scope contracts (`Principal`, `ComponentBinding`, `VantageActorBinding`, `DecisionScope`, `RealizationAcceptance`); production enablement needs 3 owner acts | same, §18 (l.1538–1635) | same |
| Module plan: Gate O in new `work-engine-organization-authority`; Gate R, consumption record and epochs in `slice-campaign` | same, §8.2 (l.1030–1039), §9 (l.1043–1066) | same |
| Independent review of rev6 | `assignment-log.md` l.39–42 | **verdict `do_not_authorize`**: F1–F6 P2, F7 P3 |
| Amendment 6.1: F3 replaces the lifetime-based token argument with a host-owned **reserve → observe-for-reservation → regate at effect commit → consume once** boundary in `slice-campaign`; F4 drops `DecisionScope.decisions`; F5 separates `IssuanceId` from `BodyDigest` | `/home/bline/cbm-work/org-rev6-20261112/AMENDMENT-6.1.md` §3–§5 (l.144–235) | Status (l.3): "design proposal. Not accepted". Review packet R6.1 **not launched** (log l.48) |
| Storage line: corrections r2, C5/C6 feasibility | log l.67–80 | **BLOCKER C5-B1**: passive file readers can observe `Activate` before its fsync. Recommended candidate: a protected authority service. Allocation exhausted; stopped for the owner |
| Recovery plan r2 (later than PLAN.md): "Lawful organizational and realization admission — contingent" | `/home/bline/cbm-work/org-recovery-campaign-20261113/PLAN-R2.md` l.59–61 | proposed; same holds as PLAN.md R2 |
| Rev7 handoff: "Gate O and Gate R are designs only, in Stage 0 r5 §4.3" | `handoff-org-compilation-root-rev7.md` l.25–26 | It cites **r5**, while rev6 and 6.1 exist. I treat rev6 + 6.1 as the latest design text, with standing as above |
| Code mentions | `work-engine-organization/src/enforce.rs:226,248`; `work-engine-authority-store/src/lib.rs:4`, `store.rs:233,265`, `fixture.rs:7`; `tests/rules.rs:1529` | Comments only. No Gate O or Gate R type exists |
| Related pending designs (main repo) | `design/ideas/pending/execution-envelope-live-effect-admission.md` (status l.5–33: exploratory; prior review `BLOCKING`); `design/ideas/pending/authority-commit-gate-and-effect-admission.md` (status l.7–16: exploratory, two hard prerequisites unowned) | Exploratory. Read: status blocks and the EEA placement chain (l.287–318) only |

**Absence claim and its basis.** A case-insensitive search for `gate[ _-]?o\b|gate[ _-]?r\b`
over the main repo and the worktree, excluding `target/`, matched only the 5 Rust files listed above.
**No canonical architecture page names Gate O or Gate R.** The basis is ripgrep over both trees. It covers
every non-ignored text file, but it skips gitignored and hidden paths, so absence is established for that
subset only. Under `/home/bline/cbm-work` the search was partial. The broad pass hit its 250-file limit and
timed out. The narrowed `org-*/*.md` pass listed 38 files with 284 occurrences, but ripgrep also reported it
as stopped at the time limit. I read the rev6/6.1/r2/rev7/PLAN-R2 files named above and did not read the
other org-* hits.

Consequence: the Gate O/Gate R decision logic (O1–O9, R1–R4) is reviewed design material. It has two
unresolved defect classes. One is storage and publication (C5-B1). The other was token-lifetime enforcement,
which 6.1 F3 replaces with a host boundary. This proposal reuses the decision logic and the §18 record
contracts. It does **not** depend on the storage design.

---

## 2. Owners and missing consumer seams (question 1)

The canonical layering is accepted design meaning. Org compilation §7–§8
(`design/architecture/organizational-compilation.md:385-456`) places organizational admission before
runtime realization: "The execution organization must become durable, revisioned state before runtime
realization treats it as authoritative". Runtime realization §4 and §6
(`design/architecture/runtime-realization.md:132-210`) places realization admission in that dimension, as an
immutable artifact. Authority and ownership §6 and §12 (`authority-and-ownership.md:285-315, 554-592`)
separate judgment from admission and publication, and say invalidation never mints authority.

| Concern | Owner (existing or decided) | Current code fact | Exact missing consumer seam |
|---|---|---|---|
| Candidate and logical vantages (pure) | `work-engine-organization` (accepted 1A subtree `f6e68f6a`, unchanged) | `enforceability()` emits separation rows as `EnforceStatus::None` (OC2) at `enforce.rs:269-283`; only `Profile::FixedSliceReviewV1` exists (`enforce.rs:257`) | None. It stays pure. Gate O **reads** its report and view; it never edits them |
| Organizational standing, grants, §18 binding records, admissions | **Organizational authority owner.** Recommended home: a new pure crate `work-engine-organization-authority`, the name planned by Stage 0 §8.2 and §9. It is the *single* owner (see below) | `work-engine-authority-store::Store::admit` returns `Result<Infallible, AdmissionRefusal>` and can only refuse (`store.rs:242-270`). Its fixture admissions and grants are P1-F publication-fixture rows (`fixture.rs:1-37`) | (a) No observation contract: nothing serves "the authority state at revision N" to any gate. (b) No Gate O exists. (c) No §18 record types exist. (d) No admission-state computation (current, stale, revoked, unknown) exists |
| Principal-to-vantage separation | Same owner (records). Checked by Gate O (O8) and Gate R (actual actor) | Preparation emits `separation:<cond>` blockers: "no qualified principal-to-vantage identity mapping exists (OC2)" (`prepare.rs:463-474`) | No record type, no resolver, and no consumer that discharges `separation:*` |
| Component bindings | Same owner (records). Trust evidence for the installed executable digest reuses the existing deployment pin owner, `slice-campaign::installed_authority` (`HostDeploymentPin` l.41, `InstalledAcceptancePin` l.254-318) | Preparation emits `component-binding:<c>` blockers (`prepare.rs:506-516`) | No `ComponentBinding` record and no binding-to-installed-digest check |
| Per-effect concrete runtime admission (Gate R + boundary single use) | **Execution owner at the consuming boundary.** For the R1/R3 Pi vertical that is the delivery host, `work-engine-review-delivery`. For the campaign path, Stage 0 and 6.1 place it in `slice-campaign`, which already owns operation identity and CAS revisions (`admission.rs:4-18`) | `deliver_and_consume` accepts only an `ExperimentBinding` and runs `run_native` first (`native.rs:1206-1210`). It consumes via `binding.consume` (`native.rs:1224`). `ExperimentBinding` compares `blockers` and `realization_evaluation` for **equality only** (`binding.rs:112-117`) | (a) No admission argument on `deliver_and_consume`. (b) No pre-spawn admission check. (c) No re-check before consumption. (d) No single-use boundary record. (e) `KnownIncompatibility` blockers do not refuse |
| Realization evidence | Runtime realization (immutable realization artifact + owner `RealizationAcceptance`) | `RealizationClaim` is explicitly "a claim ... not admission evidence" and is never reported as met (`realization.rs:1-9, 26-47`) | Nothing turns a claim into an accepted realization. R4 (accepted realization digest) has no source |
| Native task admission (Pi) | `session-task-admission` (registry owns authority and uncertainty, runtime owns observations: `pi_pilot.rs:1-2`) | `QualifiedContext` has no production constructor (`admission.rs:1-31`); the missing qualified mode/controller is a hard `Held` (`native_authority.rs:1-2`) | Out of R2 scope. It is a held prerequisite for any live Pi route (§5) |
| Result shape and episode transition | `review-episode-core` (`validate_result` checks revision/authority, `reducer.rs:105-139`; `transition` checks identity, `reducer.rs:140-148`) | Unchanged consumer | None. Admission must not duplicate these checks |
| Publication / acceptance consequence | Explicit acceptor plus consuming owner; P1-F `publish_ordinary` always refuses (`store.rs:272-276`) | — | Unchanged in R2 |

**How this avoids a parallel grant owner.** No production grant owner exists today. `admit` cannot
succeed, and P1-F's grants are fixture-domain rows that ordinary paths never read (`lib.rs:1-7`). The
proposal designates one owner for organizational standing. The P1-F authority-store rows stay P1-F history
and are **not** an input to Gate O. `Store::admit` keeps its refusing signature. Optionally, it can delegate
its O5 computation to the new owner, so the O5 logic has one implementation.

**How this avoids a parallel scheduling owner.** The single-use and epoch record is not invented anew. In
the fixture vertical it is a fixture ledger inside the delivery host. In production its home is an open owner
decision (D7): either `slice-campaign`'s existing operation store (Stage 0, 6.1 F3) or the lifecycle attempt
owner. R2 builds no production epoch store.

**Why not put authority in the pure crate.** Stage 0 §9 (l.1056–1058) and the accepted 1A boundary require the
pure crate to carry no authority types. Changing it would also touch the accepted subtree, which the
standing rev7 hold forbids (`handoff-rev7` l.22–23).

---

## 3. Separation and component-binding mechanism (question 2)

The requirement, from authority-and-ownership invariants 2, 3, 11 and 12 (`authority-and-ownership.md:596-624`):
information, capability and execution proximity are not authority, and the same actor does not imply the
same role. Preparation already refuses to treat matching strings as a mapping (`prepare.rs:472, 483`).

| Option | Mechanism | For | Against |
|---|---|---|---|
| **A. Owner-attested records** (Stage 0 §18 + 6.1 F4/F5) | `Principal{principalId, kind, actorAnchor{owner_account\|service_account, id}}`; `VantageActorBinding{organizationId, vantage, principal: IssuanceId, actor{realizationDigest, actorIdentity}}`; `ComponentBinding{component, principal: IssuanceId, executable{installedDigest}, problemScope}`. Separation means the acceptor's and each dependent vantage's principals have **different actor anchors**. Gate R then checks that the realized actor is the bound actor | Already designed and reviewed twice. Explicit and revocable. Names never bind. Matches the canonical invariants. The installed-digest evidence can reuse the existing `slice-campaign` deployment pins | Needs a protected publication mechanism before production, which is blocked (C5-B1, authority-service hold). An actor anchor only separates distinct OS or owner accounts, so two vantages under one service account can never be separate. That is conservative, and in my judgment correct |
| B. Runtime-derived separation | Derive separation from observed uid, credential, or session id at run time | No authority store needed | Mints authority from observation, which violates invariants 2, 3 and 13. Distinct session ids are not distinct actors. S1's unclaimed list says the SDK child is not confined |
| C. Key-anchored capabilities | Per-principal signing keys (cf. `session-task-admission` key policy / registry fence); separation means distinct key custodians | Cryptographic, cross-host, revocable through a registry | Moves the actor-anchor problem into key custody and adds key infrastructure now. It can later be added as a new `actorAnchor` kind under A without changing the contract |
| D. Defer separation | Keep OC2 `None` and admit only obligations without separation | Nothing to build | Not available: v1 candidates always carry separation rows and are never executable (`handoff-rev7` l.46–47) |

**Recommendation: A.** The records are read through the observation contract (§4.1), not a particular log.
Production Gate O refuses O4 component bindings and O8 with `binding_contract_unqualified`, and O5 refuses
under v1. This stays in force until the owner approves a mechanism-table revision (D5), a qualified
publication mechanism, and issued records (Stage 0 §18.6). In the fixture domain only, a
`FixtureSeparationAttestation` and fixture §18 records let O5, O8 and R3 decide positively. This is Stage 0
§4.3 l.716–728. The production O5/O8 code cannot name the attestation type.

Separation is a production-path claim. It quantifies over who produced and who accepts. Its consumer is
acceptance, not only the constraint, so it is not the anti-pattern in `system-design.md` §14. I did not load
`evidence-and-claims.md` §3.9 to confirm this classification: `unverified`.

**Pipeline position.** Derivation stays pure. Gate O is organizational authority over **one observation**.
Gate R is the execution owner over Gate O's result, the realization evidence and the actual actor, at the
boundary. All three stay separate types with separate owners. `AdmittedOrganization` does not permit an
effect. Only the host's `EffectAdmission` does.

---

## 4. Admission contract (question 3), proposed text

### 4.1 Observation

`AuthorityObservation` is one consistent read of organizational authority at a revision
`ObservationRevision{position, head}`. It resolves each referenced issuance to one of
`Committed(record, IssuanceId)`, `Revoked(IssuanceId)`, `Missing` or `Unknown`. While a write is in
progress, or after a read error, a torn record or an unavailable source, there is **no observation**
(`Unknown`). A decision never combines two observations. Production sources are out of R2 scope. R2 ships a
`fixture`-feature in-memory source with an explicit revision counter and the ability to inject `Unknown`.

### 4.2 What an admission binds

`EffectAdmission<P>`, where `P` is `Fixture` (in R2) or `Production` (no constructor in R2), carries an
`AdmissionBinding`:

```text
AdmissionBinding {
  principal        { IssuanceId, BodyDigest, actorAnchor }       -- the actual principal of the acting vantage
  actor            { actorIdentity, realizationDigest }           -- equal to VantageActorBinding.actor (R3)
  acceptor         { principal IssuanceId, actorAnchor }          -- for each separation requirement (O8)
  subject          { commit, tree, patchIdentity }                -- exact review subject (prepare.rs:23-28)
  organization     { organizationId, admission{seq, digest}, candidate contentDigest, requirementsDigest }
  obligation       { id, resultContract{id, version} }
  operation        { operationId, boundaryKind, epoch }           -- one boundary, single use
  authority        { ObservationRevision, grants[IssuanceId], bindings[IssuanceId], principals[IssuanceId],
                     decisionScopes[{id, revision}], realizationAcceptance IssuanceId }
  realization      { realizationDigest, profile }                 -- concrete realization (R4)
  preparation      { digest of the canonical ReviewPreparation; discharges[blocker id -> check id] }
}
```

### 4.3 Decision

Gate O runs O1–O9 per Stage 0 §4.3, with 6.1 F4 and F5. Every check is evaluated (no short circuit) and the
refusal carries every outcome. Gate R runs R1–R4 plus the boundary binding. An admission is issued only if
both pass **and every blocker in the canonical preparation has a discharge**:

| Blocker class (current id prefix, `prepare.rs`) | Discharged by | Production in R2 |
|---|---|---|
| `separation:*` (l.463) | O8 + R3 | refused (`binding_contract_unqualified`) |
| `component-binding:*` (l.506) | O4 binding + installed-digest match | refused (`binding_contract_unqualified`) |
| `authority-requirement:*` (l.517) | O4 grant coverage in decision scope | refused (no issued grants) |
| `realization-admission:*`, `observer-identity:*` (l.477–504) | R1, R2 and R4 against an accepted realization record, with continuity, integrity and evidence custody enumerated (fixes S1 F-8) | refused (no realization acceptance; no build-input closure per S1 `unclaimed`) |
| `realization:<cond>:<prop>`, `KnownIncompatibility` (l.525–541) | **never discharged**: the claim states a disallowed value | refused |
| `realization:<cond>:<prop>`, `Unestablished` | R4: the accepted realization record states the value; a claim alone never discharges | refused |
| `scope-digest-to-subject:*`, `problem-scope-to-episode-identity`, `consumption-evidence-mapping:*` (l.451–485) | No owner states these rules. **Fixture domain only:** discharged by the labelled `FixtureDeclaration` mapping already used by S1 (`fixture.rs:45-90`) | refused (`mapping_unowned`); remains a gap |

### 4.4 States and consequences

| State | Condition (at a point check, against a new observation) | Consequence |
|---|---|---|
| **current** | Same `ObservationRevision`, or a new revision under which Gate O and Gate R re-pass with the same request (6.1 F3 regate); epoch not consumed and latest for its boundary | Exactly one effect at that boundary may proceed; its consumption is recorded once |
| **stale** | Any of: the revision moved and regate was not run; the admission head is superseded; the epoch is not latest; the episode revision moved; the realization is marked stale (runtime-realization §7) | No new effect. An already-authorized step may finish (point-check model), but its outcome is **held** and not consumed. A new boundary needs a new admission |
| **revoked** | A pinned issuance (grant, principal, binding, decision scope, realization acceptance) is revoked in the new observation | As stale, plus no alternative is selected: "failure of an authorized candidate does not authorize a previously unauthorized alternative" (`authority-and-ownership.md:590`) |
| **unknown** | No observation; or the effect-commit outcome is uncertain | Fail closed: no advance, no consumption, no retry by uncertainty. Reconcile the same boundary identity, never mint another. This mirrors `JobAdmissionEffect::OutcomeUnknown` (`job_admission.rs:109-112`) and `Write::Unknown` (`store.rs:205-218`) |
| **refused** | Any Gate O or Gate R check fails, or a blocker is undischarged | No effect. The refusal records every check outcome. It is terminal for that request |

Unknown is distinct from refused and from revoked. None of the three is a negative semantic result about the
work. Each is recorded as such (CLAUDE.md rule 2).

### 4.5 How R1's fixture path changes

1. `deliver_and_consume(cfg, binding)` becomes `deliver_and_consume(cfg, binding, admission:
   FixtureEffectAdmission)`. Before `run_native` (`native.rs:1210`) the host checks that
   `admission.preparation.digest` equals the binding's canonical preparation. It also checks subject,
   candidate, obligation and realization digest equality, and that discharges cover **all** of
   `binding.preparation().blockers`. Any `KnownIncompatibility` refuses. On refusal no native child is
   spawned and `consumer_invocations()` stays 0.
2. Before `binding.consume` (`native.rs:1224`), the admission state is recomputed against a **new** fixture
   observation. If it is not current, consumption is refused (a new `DeliverError::AdmissionNotCurrent{state}`),
   and the run is retained as evidence, as the existing error variants do.
3. The boundary epoch is consumed once in a fixture ledger. A second use is `authorization_reused`, an older
   epoch is `authorization_stale`. This makes S1 F-7's "no-auto-replay is caller obligation only" a host
   property.
4. **Consequence for S1's accepted world:** the frozen fixture's realization mismatch (the
   fake_fixture-versus-anthropic case PLAN.md R1 says "stays unsatisfied") now **refuses before spawn**. The
   S1 positive delivery-and-consumption path therefore stops passing as written. A positive end-to-end control
   needs a second labelled synthetic world, compiled by the unchanged compiler, whose constraints the
   provider-disabled realization truthfully satisfies (D4). The frozen fixture stays unchanged and becomes
   the discriminating negative case.
5. The production (non-fixture) path is unchanged: no `EffectAdmission<Production>` constructor exists,
   `Store::admit` stays `Result<Infallible, _>`, and v1 `executable()` stays false.

---

## 5. Discriminating qualification (question 4)

All of these run in the fixture domain without network, Docker or provider. Each negative case has a paired
positive control that differs only in the varied input, and each asserts that the **intended** check refused
while the others passed.

| ID | Vary | Expect |
|---|---|---|
| Q1 | `VantageActorBinding.principal` differs from the O8-checked principal | R3 refusal; no admission |
| Q2 | Between admission and consumption, commit (a) an unrelated record, or (b) a revoke of the covering grant | (a) regate passes and consumption proceeds; (b) `revoked`, consumption refused (O4), episode unchanged |
| Q3 | Realization digest not in a committed `RealizationAcceptance`; or the claim's provider/mutation differs from the accepted record | R4 refusal / `KnownIncompatibility`; refused before spawn (spawn count 0) |
| Q4 | Admission for subject S used with a binding for subject S′, or another candidate or obligation | Refused before spawn |
| Q5 | Same admission used for a second epoch, or an earlier epoch after a newer reservation | `authorization_reused` / `authorization_stale` |
| Q6 | Realized actor identity differs from the bound actor | R3 refusal |
| Q7 | Acceptor and vantage principals have distinct ids but **the same actor anchor** | O8 refusal (names never bind) |
| Q8 | A principal whose name resembles a component; or a `ComponentBinding` whose installed digest differs | O4 refusal |
| Q9 | Observation source returns `Unknown` (write in progress / read error) at admission, or at consumption | No admission; at consumption the outcome is held, `consumer_invocations()==0`, episode state byte-identical |
| Q10 | Fixture principal, record or admission presented to the production gate or entry | Runtime domain refusal + `compile_fail` test (`FixtureEffectAdmission` is not accepted by any production entry) |
| Q11 | Ordinary-admission preservation | `Store::admit` signature unchanged; `separation_rows_are_none_in_v1_so_no_candidate_is_executable` unchanged; production Gate O yields O5 refusal + `binding_contract_unqualified` for every v1 candidate |
| Q12 | Change one candidate requirement (new blocker) without a matching discharge | Refusal naming exactly that blocker id (joins R1's "change one requirement → consequence") |
| Q13 | Remove one discharge from an otherwise complete admission | Refusal naming that blocker id |
| Q14 | Cross-module | Q3, Q4, Q5, Q9, Q12 and Q13 driven through the real `deliver_and_consume` (fixture feature), not only the pure gate |

Mutants: each must be killed by at least one row above. (a) Compare principal ids instead of anchors (Q7).
(b) Skip the regate at consumption (Q2b). (c) Accept the fixture domain on the production path (Q10).
(d) Ignore `KnownIncompatibility` (Q3). (e) Skip the discharge-completeness check (Q13). (f) Short-circuit the
check evaluation (the outcome-vector assertion). (g) Drop the epoch-consumed check (Q5).

What this qualifies: the **decision contract and the consumer seam**, at fixture standing. It does not
qualify any production binding, any publication mechanism, live execution or real review.

---

## 6. Holds and owner decisions (question 5)

### 6.1 What stays blocked (unchanged by this proposal)

- PAS-4, L1 qualification, production integration, merges, pushes, deployments, **grant issuance**,
  **authority-service deployment**, live provider calls (rev7 l.15–16; PLAN-R2 l.11).
- Execution-start fencing: **the 2026-11-12 S1 start mechanism** (`org-s1-20261112`, the EXITKILL ordering
  failure), A1-x, ptrace supervision, and the OCI-wrapper fallback. The first-instruction guarantee stays
  unqualified (rev7 l.17–19). *Naming collision:* that "S1" is not the 2026-11-16 R1 slice S1 accepted at
  `S1-ACCEPTANCE.json`.
- A fourth Docker run (PLAN.md "No further Docker run is currently authorized").
- Slice 1A semantics, enforceability, OC2, the component-binding requirement, negative tests, frozen
  fixture (rev7 l.22–23).
- R9 and AD1–AD3 stay registered as open P1s (rev7 l.20).
- New gaps this proposal names and does not close: the storage/publication mechanism (C5-B1), production
  realization acceptance (no build-input closure, per S1 `unclaimed`), the Pi native `QualifiedContext`
  (`session-task-admission`), and the three unowned mapping rules in §4.3.

### 6.2 Decisions requested from the owner

| # | Question | Recommendation |
|---|---|---|
| D1 | Adopt the owner layering in §2? Gate O and the §18 records go to a new pure `work-engine-organization-authority` crate as the single organizational-standing owner. Gate R and `EffectAdmission` go to the consuming host. Preparation blockers become discharge obligations | **Yes** |
| D2 | Adopt Option A (owner-attested principal / vantage-actor / component-binding records, actor-anchor separation, with 6.1 F4/F5) as the separation mechanism? | **Yes**. Option C may later add an anchor kind |
| D3 | Should the R1 fixture path now refuse delivery on any `KnownIncompatibility`, and on any blocker without a discharge, and re-check before consumption? This changes S1's accepted behaviour (§4.5 item 4) | **Yes**. It closes S1's semantic limit |
| D4 | Approve a second labelled synthetic world, compiled by the unchanged v1 compiler, whose realization constraints the provider-disabled Pi realization truthfully satisfies, as the positive end-to-end control? (It also raises n from 1 to 2.) If the v1 vocabulary has no such class, accept pure-gate-only positive controls and record end-to-end positive as `not established` | **Yes**, conditional on the vocabulary; the builder checks this first |
| D5 | Mechanism-table revision (a new `Profile` variant adding a binding/separation row) now, or defer? | **Defer.** R2 uses only the fixture attestation path; the accepted 1A subtree stays untouched; decide with D6 |
| D6 | Keep the authority publication mechanism (storage) out of R2, with R2 implemented against the observation contract and a fixture source only? | **Yes.** The Stage 0 1B storage question (C5-B1 → protected-service candidate) needs its own approval, and authority-service deployment is held |
| D7 | Production home of the boundary epoch and single-use record: `slice-campaign` (Stage 0 / 6.1 F3; existing operation identity, Claude campaign path) or the lifecycle attempt owner (Pi forward path)? | **Defer to R3.** Record it as a gap. R2 builds only the fixture ledger. I did not read lifecycle-* sources, so the lifecycle option is `not established` |
| D8 | Approve slices R2-A and R2-B (§7) with builder, separate reviewer and budgets? | Owner's call. Estimates below |

---

## 7. Implementation slicing and estimate (question 6)

Basis: the packet's recent-slice figures of about $10 builder and $12–19 review. These are estimates,
`unverified` against node actuals for these scopes.

### R2-A: pure admission core (fixture-qualified)

- **Scope:** a new crate `work-engine-organization-authority`. Dependencies: `work-engine-organization`,
  serde, sha2, thiserror. No filesystem, SQLite or rustix.
  - §18 record types with 6.1 F4/F5 (`IssuanceId`/`BodyDigest`, `DecisionScope` without `decisions`).
  - The `AuthorityObservation` trait, `ObservationRevision`, and a `fixture` in-memory source with
    `Unknown` injection.
  - Gate O (O1–O9, every check evaluated, `GateRefusal` outcome vector). Production O4-binding/O8 →
    `binding_contract_unqualified`; v1 O5 refuses.
  - Gate R pure predicates (R1–R4, actor, principal).
  - The `AdmissionBinding`, `admission_state()` (current, stale, revoked, unknown) and the blocker-discharge
    map.
  - Optional: `Store::admit` delegates its O5 computation, with its signature unchanged.
- **Owners touched:** the new crate only, plus at most a one-function change in `work-engine-authority-store`.
- **Tests:** Q1, Q2 (pure), Q5–Q8, Q10–Q13 at the pure level; mutants (a), (c), (e), (f), (g); a
  dependency test showing the pure crate has no authority dependency.
- **Cost:** builder $8–11, independent review $12–16, remediation $3–5. **About $23–32.**

### R2-B: R1 consumer seam

- **Scope:**
  - `work-engine-review-preparation`: stable blocker ids exported as a discharge vocabulary. The
    realization-admission blocker enumerates continuity, integrity and evidence custody (S1 F-8).
  - `work-engine-review-delivery`: `deliver_and_consume` takes `FixtureEffectAdmission`; the pre-spawn
    check; the pre-consume re-check; the fixture epoch ledger; `AdmissionNotCurrent`.
  - The D4 second world, if approved.
  - Existing S1 negatives preserved. The frozen-fixture mismatch world is re-expected as refusal before
    spawn.
- **Owners touched:** preparation and delivery crates. Test fixtures only in the authority crate.
- **Tests:** Q3, Q4, Q9, Q14 cross-module, with spawn count 0 and `consumer_invocations()==0` on refusal;
  mutants (b) and (d); the full S1 matrix re-run (provider-disabled Pi SDK, local, no Docker).
- **Cost:** builder $9–12, independent review $12–19, remediation $4–6. **About $25–37.**

Total: **about $48–69**, plus root coordination and closure (about $5–8, `unverified`). R2-B depends on R2-A.
The two could be merged into one slice (about $35–50), but that review surface would include the accepted
S1 vertical and the new authority core together, so I recommend two slices.

The exit claim available after both: *an approved admission contract with fixture-standing discriminating
qualification; production admission still refuses*. A real review demonstration needs D5, D6, D7, issued
records, a qualified publication mechanism, realization acceptance with build-input closure, and the
lifted live-provider and Pi `QualifiedContext` holds. Until then it remains a gap (PLAN.md R2 exit).

---

## 8. Confidence and what I did not read

- **Confidence:**
  - **High** that the current seams are as cited (file:line read).
  - **Medium-high** on the owner mapping and the recommendation to keep storage out.
  - **Medium** on Option A: it rests on rev6 §18 plus an *unaccepted, unreviewed* 6.1.
  - **Medium** on D4 feasibility (the v1 provider vocabulary was not checked).
  - **Low** on the production home of the epoch record (D7).
  - The cost figures are `unverified` estimates.
- **Not read:**
  - lifecycle-* sources.
  - Parts of `session-task-admission`: only the module heads and `admission.rs:1-80`.
  - Parts of `slice-campaign`: only `admission.rs`, `job_admission.rs:1-130` and
    `installed_authority.rs:205-399`.
  - Parts of `work-engine-authority-store`: only `lib.rs`, `store.rs:200-290` and `fixture.rs:1-60`.
  - `work-engine-review-delivery/src/attempt.rs`.
  - `work-engine-organization` `view.rs` and `derive.rs`.
  - Stage 0 rev6 §4.4–§4.6, §10 and §17.
  - Amendment 6.1 §1–§2 and §6–§9, Corrections r2, and the C5/C6 feasibility doc (known only from the
    assignment log).
  - The ExecutionEnvelope live-effect admission body (§1–§18).
  - The authority-commit-gate body.
  - `deterministic-authority-projection-and-adaptive-organizational-topology.md`.
  - `role-and-contract-structure.md` (grep only).
  - `evidence-and-claims.md` §3.9.
  - S1 `REVIEW-1.md`, `RECHECK-1.md` and `MATRIX.md`.
  - The board (read denied).
- No builds, tests, network, Docker or git operations were run. The forbidden paths were not read.
