# R2 reconciliation with the finished dev vertical

Standing: a design reconciliation written by one worker (Opus 5.5, xhigh requested), for the Pi root
`01a1e56e-be12-7424-977b-c7c7983887d0`. It is a proposal only. It grants no implementation, issuance,
storage, production-admission or live-spend authority. Where it goes beyond the approved R2 design
(`s2/OWNER-DECISION-S2.json`, proposal as revised by `s2/ROOT-S2-ADDENDUM.md`), it says
**design change: needs owner approval**.

Source: main checkout `/home/bline/code/work-engine` at HEAD `9b289171`, read-only. Paths below are relative to
`rust/crates/` unless they start with `docs/`, `records/`, `planning/` or `s2/`/`s3/` (the latter under
`/home/bline/cbm-work/org-pi-slices-20261116/`). No builds, tests, live jobs or git operations were run, so every
behavioural statement below comes from reading source and is not observed at runtime.

---

## 0. Verdict on the root's hypothesis

The hypothesis has two parts. Each is partly right.

**H1: "R2-B's host boundary inside the fixture delivery crate would duplicate an existing owner."**
Right about what the boundary means in production. Not right about the fixture spawn as such.

- On the hosted Pi path, every mechanism R2-B planned for the delivery effect already has an owner. Reservation
  is the Host's durable attempt record. Currentness at effect commit is the lifecycle grant, checked inside a
  `BEGIN IMMEDIATE` transaction held across the exact write. Single use is "consumed before entry". Attempt
  identity is `(request, attempt)`. Settlement belongs to the Host journal and the ledger rule. Evidence is in §1.
- The review-delivery crate does not use that owner. It spawns its own Node SDK child (`native.rs:1126`) and
  depends on neither `work-engine-manual-host` nor any `lifecycle-*` crate (`work-engine-review-delivery/Cargo.toml:19-28`).
  An R2-B boundary built there would therefore not duplicate code. It would be a second mechanism with the same
  production meaning, and the only thing consuming it would be its own fixture gate. That is the S1 lesson, and it
  is also an anti-pattern in `docs/contracts/system-design.md` §14 ("a production-path claim whose only consumer is
  the constraint that requires it").
- Two parts of R2-B are not duplicated anywhere and remain needed: deriving discharges from gate outcomes
  (addendum item 6), and the regate before consumption. Consumption single-use and staleness already belong to
  `review-episode-core` (review F-7).

**H2: "R2-A's admission core should become what the lifecycle grant consumes."**
Right in direction, but the shape needs correcting. At runtime the lifecycle gate should not consume an admission
object. Instead, organizational authority should be the **issuer** of a lifecycle grant whose identity binds the
admission. The dev vertical already binds its route policy into the grant id this way
(`pi-pressure-context-host/src/coding.rs:240-243`). The existing per-effect currentness transaction then serves as
the 6.1 F3 regate. This needs two additions:

- an organizational currentness check inside that same transaction, or a propagation fence; and
- an issuer that the producing binary does not control.

Both are **design changes** that need owner approval. The approved decision's `effect` lists "grant issuance" and
"production admission" as unauthorized.

---

## 1. Owner map (question 1)

How Pi jobs run on the hosted profile `work-engine.pi-coding-hosted.v1` today:

| Responsibility | Current owner | Evidence |
|---|---|---|
| Caller → job authorization; binds the request to one pinned job before any execution; one Host state per authorization | `work-engine-server` `Jobs` | `work-engine-server/src/job.rs:1-10`, `:55-68`, `:507-509`, `:595-616` |
| Funding/route/model policy (runtime admission) | operator-written `work-engine.pi-coding-runtime-admission.v1` (11 fields, digest-pinned in the job), checked by the server and repeated by the Host | `job.rs:196-197`, `:232-266`; `work-engine-server/src/bin/coding-job-prepare.rs:301-323` |
| Per-provider-attempt admission: order, limit, route/payer within the granted policy, consumed before entry | `CodingContinuity::provider_attempt` | `coding.rs:285-321` |
| Currentness re-check before each upstream dispatch | `lifecycle-store` `with_pi_coding_turn_send` → `parent_binding` (`BEGIN IMMEDIATE`, held through the send closure) | `lifecycle-store/src/pi_coding.rs:199-222`, `:165-198`; called at `coding.rs:333-343` |
| Per-tool-effect admission, serialized with the actuator; the Host's durable intent comes first | `CodingLifecycleGate::execute` → `with_pi_coding_effect` | `coding.rs:97-130`; `lifecycle-store/src/pi_coding.rs:65-139` |
| Currentness predicate | `grants` row: `revision` = turn's revision, `revoked=0`, `issuer` = store's `trusted_issuer`, scope, subject/context/build/proof-run binding, `expires_ms > now`, admin ownership active, native session current | `lifecycle-store/src/pi_coding.rs:93`, `:173-185` |
| Revocation | `revoke_trusted_grant` (revision + 1, durable before new entry) | `lifecycle-store/src/sqlite.rs:646-669`; `pi_coding.rs:69-70` |
| Native turn: one send opportunity per invocation | `claim_native_turn` + `mark_native_send_intent`; comment: "the durable send CAS has already consumed the sole send opportunity" | `coding.rs:487-519`; `lifecycle-store/src/pi_coding.rs:200-201` (body of `claim_native_turn`, `native.rs:2440`, not read) |
| Attempt identity and durable per-attempt journal (`gate_entered`, deadline, routes digest, uncertain/released) | Host, `work-engine-manual-host` `pi_coding_hosted` | `work-engine-manual-host/src/pi_coding_hosted.rs:1-11`, `:837-904` |
| Attempt settlement (Completed / Uncertain; K-a, Q-a) | Host journal + router disposition; the ledger rule | `records/dev-vertical-hosted-qualification-20261117/README.md:67-80`; `pi_coding_hosted.rs:914` (`settle_attempt`) |
| Parent turn settlement | `settle_pi_coding` | `lifecycle-store/src/pi_coding.rs:285` |
| **Grant issuance** | **the producing binary itself** (K-c): it opens the store as issuer `"fixture-root"` and installs four grants with principal `"operator:fixture"` | `pi-pressure-context-host/src/bin/coding-qualification.rs:168-205`; `docs/dev-vertical-open-items.md:33-38` |
| Consumption single-use and staleness (review) | `review-episode-core` reducer | cited by review F-7 (`review-episode-core/src/reducer.rs:105-116, 140-151`); I did not re-read it |

Other owners found on the way that bear on the seam:

- **Signed per-effect leases**: `session-task-admission::pi_pilot` (`PiOwner::enter` → `EffectLease`, signed
  proof/key/current-board re-check; a newly observed revocation is retained). See `session-task-admission/src/pi_pilot.rs:439-487`
  and `:673-740`. It is wired only to the signed fixture profile: `Host::admit` refuses `SIGNED_PROFILE` without
  an owner, and the hosted path passes `None` (`work-engine-manual-host/src/pi_docker.rs:316-346`). The offline
  native review route also uses it and parses implementation-review/v1 (`work-engine-manual-host/src/pi_native.rs:493`,
  `:728-735`). The non-test callers of `pi_native` are not established; I grepped manual-host and server only.
- **`work-engine-authority-store`** (P1-F) has its own `grants` table (`work-engine-authority-store/src/schema.rs:28`).
  No other crate's `Cargo.toml` names it (grep over `rust/crates/**/Cargo.toml`). It is a third grant notion and
  should not become the issuer store (see §6, N8).

**Answers.**

- Per-effect runtime admission, single use / attempt identity, currentness re-check and settlement for hosted Pi
  jobs are held jointly. The Host (`work-engine-manual-host`) owns the journal, identity and settlement.
  `lifecycle-store` owns grant currentness, the send CAS and parent settlement. The bridge `CodingContinuity`
  (`pi-pressure-context-host`) owns attempt order and policy membership. They compose; none of them is a second
  ledger (`coding.rs:1-3`; `lifecycle-store/src/pi_coding.rs:1-3`).
- R2-B as scoped in `R2A-ESTIMATE.md` slice B would duplicate this for the delivery effect: reserve → observe →
  gate → regate at commit, plus per-boundary single use. For the consumption effect it would duplicate
  `review-episode-core`. **The hypothesis holds for that slice.**
- Delivery-boundary single use (O10/D7) has a production home already, provided review jobs run through the
  Host. One prompt delivery is one native-turn send CAS. Each paid provider dispatch (the effect that exposes the
  subject) is one `(request, attempt)` consumed before entry. The Host records `gate_entered` before any upstream
  byte (`pi_coding_hosted.rs:857-872`, `coding.rs:320-321`). On the fixture SDK path it stays the gap O10 recorded.

---

## 2. The seam (question 2)

Design change: needs owner approval. It builds on O1, O4 and O9, but makes the lifecycle grant the carrier.

### 2.1 Where each piece lives

```text
org authority (new pure crate, O1)        issuer role (S-C)                  existing hosted owners
  Gate O + Gate R over one observation  ->  installs a lifecycle grant    ->  CodingContinuity / lifecycle-store /
  sealed to a reservation (6.1 F3)          projected from the admission      Host journal: per-effect currentness,
  -> EffectAdmission + discharge map        (no admission copy)               consumption-before-entry, settlement
```

- **Organizational authority** decides, and keeps the admission record. Its observation source is fixture-only
  until O6's storage decision.
- **The grant carries a reference, not a copy.** A pure `GrantProjection(admission)` yields the grant fields:
  - `issuer`: the organizational issuer's pinned identity;
  - `principal`: the admission's principal `IssuanceId`;
  - `subject`/`context`/`build`/`proof_run`: from the lifecycle subject registration;
  - `scope`: `native_turn`, plus `request_replacement` only if admitted;
  - `expires`: min(observation validity, admission horizon, job deadline);
  - `grant_id`: `base:org:<admission digest>`, composed with the existing `:pi-hosted:<policy digest>`.

  That is the whole seam. There is no second ledger: lifecycle stores a grant row, as it does today, and the Host
  journal is unchanged apart from attribution.
- **Lifecycle subject ↔ review subject** (`commit`, `tree`, `patchIdentity`). This mapping is the unowned
  `scope-digest-to-subject` rule (O7). Until an owner is named, the projection can be qualified only on fixture
  subjects.

### 2.2 Where organizational currentness is re-checked

There are two candidate mechanisms. I recommend using both, with (b) as the guard.

- **(a) Propagation fence.** When an observation shows a pinned issuance revoked or superseded, the issuer calls
  `revoke_trusted_grant`. Lifecycle then refuses the next entry on its own (`sqlite.rs:646-669`).
  Alone this is insufficient. Between the change and its propagation the grant still reads as current, which
  breaks the F-4 obligation that "an uncertain revocation keeps fencing". Short grant expiry, with renewal only on
  a current observation, narrows the window but does not close it.
- **(b) Per-effect regate inside the existing transaction.** `CodingContinuity` holds an
  `OrganizationalCurrentness` handle. It evaluates the handle against a new observation inside the
  `with_pi_coding_turn_send` and `with_pi_coding_effect` closures, before `dispatch` / the actuator runs
  (`coding.rs:333-342`, `:120-129`). Lifecycle grant currentness and organizational currentness are then held in
  one serialized step, which is 6.1 F3's "regate at effect commit", and no new gate exists.

Granularity is **per effect**: every provider attempt, every tool effect, and every IPC dispatch, which is exactly
where lifecycle already re-checks (`coding.rs:1094-1101` for dispatch). It is not per job. The budget/control
check already runs on every frame (`coding.rs:1076-1077`).

### 2.3 Consequences of each state, mapped onto existing outcomes

| Org state at the regate | Before dispatch / actuator | Already dispatched |
|---|---|---|
| current | proceeds; `gate_entered` recorded | — |
| stale (revision moved, regate fails; head superseded) | refused inside gate admission; attempt released, nothing relayed (`pi_coding_hosted.rs:881-903`, README:56-58) | outcome **held**: the attempt settles per K-a/Q-a, the review result is not consumed (consumption regate refuses) |
| revoked | as stale; no alternative is selected. The route list is fixed in the grant (`coding.rs:313-318`), so failover cannot add an unauthorized route; failover to *another permitted pair* must also stop, which is a new condition | as stale |
| unknown (no observation / read error) | fail closed: refused, no retry by uncertainty, same boundary identity on reconcile | Uncertain, as today |
| refused (Gate O/R fails, undischarged kind) | the grant is never installed, so no turn can be claimed | — |

Reconcile after a crash keeps the same `(request, attempt)` and never mints another. The Host already does this:
"existing job: reconcile instead of launch replay" (`pi_docker.rs:354-356`).

### 2.4 Discharges

The host-owned total function `kind → discharge` stays in review-delivery (`work-engine-review-delivery/src/gate.rs:41-53`).
Its range changes from `AssumedDischarge` to outcomes of the admission's checks (proposal §4.3 table):

| `BlockerKind` (`work-engine-review-preparation/src/prepare.rs:215-235`) | Discharged by |
|---|---|
| `Separation` | O8 + R3 |
| `ComponentBinding` | O4 binding + installed-digest match |
| `AuthorityRequirement` | O4 grant coverage in decision scope |
| `RealizationAdmission`, `ObserverIdentity` | R1, R2, R4 against an accepted realization record |
| `RealizationCheck` | never in R2: KnownIncompatibility never; Unestablished would need R4 from a record, kept out |
| `ScopeDigestToSubject`, `ProblemScopeToEpisodeIdentity`, `ConsumptionEvidenceMapping` | labelled fixture assumption only (O7) |

On the hosted path, the R3 actual-actor evidence O8 asks for already exists as Host observation:

- container identity and exec custody (`coding.rs:139-161`);
- `NetworkMode=none`, no `/auth` mount, checked binaries (README:53-65).

`realizationDigest` (review F-6.2, never defined) can therefore be Host-computed. Its inputs are the profile/protocol
digest pair, `PI_SDK_INDEX_SHA256`, the inference image digest, the runtime-admission digest and the Pi config
manifest. That keeps R4 from comparing two values the caller supplied. Design change, N5.

---

## 3. A read-only review job on the hosted path (question 3)

**Needed, none of which exists today:**

1. **Profile.** A new versioned hosted review profile.
   - The current hosted class requires `max_tool_calls == 1` and `WorkspaceWriteBoundary::SrcLibRs`
     (`job.rs:208-212`; `pi_coding.rs:242-246`), and lifecycle accepts exactly three profile/protocol pairs
     (`lifecycle-store/src/pi_coding.rs:42-56`).
   - A review profile needs mutation `none`: no write boundary and no exec tool, with read-only claims tools.
     `ClaimsGate` exists and forwards everything else to the lifecycle gate (`pi_coding_claims.rs:1-27`), but it
     is composed only into a fixture profile. Whether that profile works with provider transport is not established.
   - Owners touched: manual-host (profile + admission), lifecycle-store (new digest pair), server (`verify_live_job`).
2. **Mutation evidence.** On the hosted path, "no mutation" can be observed rather than claimed. The inference
   container has no network, and the Host owns the workspace and the export. This is stronger than the R2-S ruling
   that process-level writes are not established. The Host would still have to record "no export / empty diff" as
   the evidence.
3. **Result contract.** implementation-review/v1, through `review_episode_core::implementation_review_v1::ReviewResult::parse`
   (`review-episode-core/src/implementation_review_v1.rs:31`). This is already done on the offline signed route
   (`pi_native.rs:728-735`). The hosted path has no result consumer: every output is `owner_pending`
   (`coding-job-prepare.rs:433`; open items:39-42).
4. **Consumer.** The `review-episode-core` reducer, which owns consumption single use. The production consumer
   stays unreachable while the three mapping rules have no owner (O7). So a hosted review result could be produced
   and validated, but not consumed with production standing.

**Stays fixture-only:**

- the R2-S Node SDK spawn path, as the discriminating oracle and the negative world;
- the labelled second world;
- the fixture observation source;
- the mapping-kind assumptions;
- `FixtureDeclaration`.

**Needs live/paid authority from the operator:**

- any hosted review run, which spends provider calls under the 700000 µUSD proxy cap (R-c, open items:13);
- use of the subscription route beyond the operator's own use, which is deferred (open items:12);
- a review submitted by an organizational principal who is not the funder, which first needs K-c closed;
- consuming an org admission on the live path at all. That *is* production admission, which is currently
  unauthorized.

---

## 4. Independent grant issuer (K-c) (question 4)

**Can R2-A's organizational authority be the independent issuer?** It can be the *deciding* authority. It cannot
be the issuer by itself. The approved home is a pure crate with no storage or keys (O1, O6), so issuing requires a
role that runs its decision over an observation source and installs the grant.

**Why today's grants add no independence (source):**

- Lifecycle accepts a grant only if `grant.issuer == trusted_issuer` (`sqlite.rs:621-628`). But `trusted_issuer`
  is whatever string the opener passes (`coding-qualification.rs:169`: `"fixture-root"`), and nothing
  authenticates the installer.
- `lifecycle-service` likewise installs grants from its own configuration (`lifecycle-service/src/actor.rs:853-870`).
- The Pi coding currentness predicate does not check `principal` (`lifecycle-store/src/pi_coding.rs:93`, `:173`).
  The direct-semantic path does check it (`lifecycle-store/src/direct_semantic.rs:151`). Whether `claim_native_turn`
  binds the principal it is given (`coding.rs:507`, `"operator:fixture"`) is not established, because I did not
  read `native.rs:2440`.
- I did not search for any path that revokes a hosted grant during a job, so whether one exists is not established.

**What an independent issuer changes in the dev vertical** (design change, N4):

- `coding-qualification` stops installing grants (`coding-qualification.rs:178-205`). It verifies that externally
  issued grants exist and are current. The store is opened with an issuer identity pinned by the operator, not
  chosen by the binary.
- Issuance must be unforgeable by the producing binary. Either:
  - grants are signed and lifecycle verifies the signature at install against an operator-pinned key; or
  - a separate process owns the write.

  `pi_pilot`'s signed proof/current-board machinery is a candidate. Its reuse is not established.
- Runtime admission v2 pins the issuer identity and the grant references. `verify_live_job` KEYS (`job.rs:236-248`)
  and prepare (`coding-job-prepare.rs:308-321`) gain the fields.
- The Pi coding currentness predicate adds `g.principal` (lifecycle-store).
- `CodingContinuity` gains the §2.2(b) regate handle.

**Compatibility.** Under the pre-release policy no runtime consumer requires the v1 shapes. The cost falls on
qualification evidence: the r8 offline matrix (19 scenarios) and the live cutover records are bound to the
self-issued composition and `7c103ad2`. After the change, re-run the offline matrix and do one live smoke. The
`session-query` `OWNER_SHAS` pins (open items:43-46) are unaffected unless `session_events.py` is touched.

**Sequencing.** First qualify the issuer with the funder as the only principal. That stays within today's
operator authority and closes "the grant adds no independent authority". Non-funder submission remains a separate
decision.

---

## 5. Rescoped slices and estimate (question 5)

These replace R2-A' (slice A) and R2-B (slice B) of `R2A-ESTIMATE.md`. Cost basis is the packet's actuals: Opus
builder $7–10, Fable review $7–12, Opus-xhigh normal review cheaper, remediation $1–3, recheck $2–7, root about $5
per slice. All figures are **unverified** estimates.

### S-A. Admission core with grant projection (fixture-qualified, pure)

- **Scope.** R2A-ESTIMATE slice A, with three changes:
  - the observation is sealed to a reservation (`observe_for(&reservation)`, 6.1 F3), with the
    same-observation/later-boundary negative (6.1 B12b) tested *in the core*;
  - add the pure `GrantProjection` (§2.1), which outputs an org-owned struct and does not depend on `lifecycle-core`;
  - add the pure `realization_digest` definition for the Pi hosted realization (§2.4).

  It mints nothing and installs nothing.
- **Owners touched.** The new crate only.
- **Tests.** Gate O subset (O3, O4, O5, O8, O9) and R1–R4; the five states; the 6.1 §6 oracle classes; projection
  totality, and refusal on any non-current state; mutants.
- **Cost.**

  | Builder (Opus, top of range: new crate) | Fable xhigh review | Remediation | Recheck | Root | Total |
  |---|---|---|---|---|---|
  | $9–12 | $7–12 | $1–3 | $2–7 | $5 | **$24–39** |

- **Live inference:** none.

### S-B. Consumption seam and real discharges (fixture)

- **Scope.**
  - Review-delivery's total discharge function returns S-A check outcomes for `Separation`, `ComponentBinding`,
    `AuthorityRequirement`, `RealizationAdmission` and `ObserverIdentity`. The mapping kinds stay assumed;
    `RealizationCheck` stays undischarged.
  - Before `binding.consume`, regate against a new fixture observation. Single use and staleness come from
    `review-episode-core`; there is no epoch ledger.
  - **No reservation/regate host boundary at the SDK spawn.** The R2-S pre-spawn gate stays as it is, and fixture
    delivery single use remains the O10 gap.
  - Close the R2-S P3s that bear on this: `run_attempt`/`run_attempt_observed` are ungated public spawn routes;
    `AssumedDischarges` is publicly mintable and not bound to a binding.
- **Owners touched.** `work-engine-review-delivery`; `review-preparation` unchanged.
- **Tests.** Re-run the S1/S3 matrices; mutants; stale/revoked/unknown at consumption.
- **Cost.**

  | Builder (Opus; S3 was $7.32) | Opus-xhigh review (smaller surface; unverified) | Remediation | Recheck | Root | Total |
  |---|---|---|---|---|---|
  | $7–9 | $4–7 | $1–3 | $2–5 | $5 | **$19–29** |

- **Live inference:** none.

**S-A + S-B together:** $43–68, against $50–74 for R2-A' + R2-B (unverified). Together they meet R2's exit as the
approved design states it, on fixture standing.

### S-C. Independent issuer on the hosted path (K-c) (design change; live)

- **Scope.**
  - An issuer role runs S-A over an observation source and installs projected grants under an operator-pinned
    issuer identity, either signed or with a separate writer.
  - `coding-qualification` verifies grants instead of issuing them.
  - Runtime admission v2.
  - `principal` added to the Pi coding currentness predicate.
  - The §2.2(b) regate in `CodingContinuity`, plus the §2.2(a) revocation fence.
  - Coding profile only, with the funder as the only principal. The review profile is not part of this slice.
- **Owners touched.** `pi-pressure-context-host` (coding.rs, coding-qualification), `lifecycle-store`
  (pi_coding currentness, install verification), `work-engine-server` (job.rs, coding-job-prepare), plus the new
  issuer role.
- **Tests.** Offline matrix (19 scenarios) re-run; new scenarios for org revocation mid-job, unknown observation,
  and an issuer-forgery refusal; one live smoke.
- **Cost.**

  | Builder | Fable review | Remediation | Recheck | Root | Live smoke | Total |
  |---|---|---|---|---|---|---|
  | $9–12 | $7–12 | $1–3 | $2–7 | $5 | ≤ $1 (proxy cap $0.70 per job; actual not established) | **$25–40** |

- **Needs live/paid inference, and owner approval of N2–N4 and O6** (an observation source).

**Not estimated:** a hosted read-only review profile (§3, N7). Its production consumption stays unreachable under
O7, so I recommend holding it until mapping-rule owners are named.

**Risk.** The delivery-gate subsystem already carries one rejection warning (R2A-ESTIMATE, history r41). S-B
touches that subsystem, so a second substantive rejection there triggers the design-review rule (about $8).

---

## 6. Owner decisions (question 6)

### New decisions (recommendation in brackets)

| # | Decision | Rec. |
|---|---|---|
| N1 | Drop R2-B's spawn-boundary host. Production delivery single use lives in the lifecycle native-turn send CAS plus the Host per-attempt journal; the fixture SDK spawn keeps the O10 gap | Yes |
| N2 | The seam is a lifecycle grant issued from a current org admission (`GrantProjection`, §2.1); the grant references the admission and never copies it | Yes (projection in S-A; issuance in S-C) |
| N3 | Org currentness is re-checked per effect inside the existing lifecycle transaction (§2.2 b), with revocation propagation as the fence (§2.2 a) | Yes; this makes O6's storage decision a prerequisite for any live use |
| N4 | K-c closure: an issuer role (not the pure crate) with an operator-pinned identity and signed or separate-writer installation; runtime admission v2; producing binary verifies only. First qualified with the funder as the only principal | Yes, after S-A |
| N5 | `realizationDigest` for Pi hosted is Host-computed (§2.4); production R3 uses existing Host process/container observation (concretizes O8) | Yes |
| N6 | `grants.principal` = org Principal `IssuanceId`; `CommandAdmission.principal` stays the command-caller identity; add `principal` to the Pi coding currentness predicate | Yes |
| N7 | Hosted read-only review profile (§3) as the review route, with the R2-S SDK path becoming oracle-only | Defer until O7 owners exist |
| N8 | `work-engine-authority-store` (P1-F) is not the issuer store; retire it, or keep it as fixture only (F-11 `Store::admit`) | Keep fixture-only; decide retirement separately |

### Changes to the approved O1–O11

- **O1.** Unchanged. "The consuming host" now means two things: Host + lifecycle on the hosted path, and
  review-delivery on the fixture path.
- **O4.** Delivery is still its own admission boundary. For production its *owner* moves from review-delivery to
  Host + lifecycle, through per-attempt dispatch and the send CAS. This is a placement change and needs confirmation.
- **O6.** Wording unchanged, but it now blocks S-C and any live consumption.
- **O8.** Now has a concrete existing source of evidence (Host custody).
- **O9.** 6.1 F3's "regate at effect commit" is realized by the existing lifecycle transaction. The reserve/seal
  step moves into the S-A core.
- **O10.** Changed: the production home is decided now (N1) instead of in R3.
- **O2, O3, O5, O7, O11.** Unchanged. O11 (R2-S) is done.
- **Addendum item 10 and R2A-ESTIMATE.** Slices A and B are replaced by S-A and S-B. S-C is new and needs the
  authority listed above.

---

## 7. Confidence and what I did not read

**Confidence, by part:**

- §1 owner map and §0 H1: high. Each row was read at the cited lines; n = 1 reading per site, with no runtime
  observation.
- §2 seam: medium. It is a design, and it rests on how the cited lifecycle transactions behave. Two points are not
  established: the internals of `claim_native_turn` and the full `dispatch` body (`coding.rs:1094+`, read only to
  line 1101).
- §5 costs: low to medium; unverified estimates from the packet's actuals.

**Not read:**

- `review-episode-core/src/reducer.rs` (relied on review F-7's citation);
- `lifecycle-store/src/native.rs` (`claim_native_turn`);
- the full `pi_coding_hosted.rs` and `pi_route` (ledger reservation, beyond the header and `:760-914`);
- `session-task-admission` beyond `pi_pilot.rs:380-400, 439-490, 673-745`;
- `review-execution-evidence`;
- Amendment 6.1 and Stage 0 rev6;
- `planning/dev-vertical-completion-20261115.md` beyond `:11-90`;
- the qualification README beyond `:1-82`;
- `architectural-risk-history.json`;
- `R2-S` build/review artifacts beyond `S3-ACCEPTANCE.json`;
- proposal §§0–3 and 5–8, and review sections other than F-2, F-6 and F-7.

**Board.** The repository board read was denied by the permission mode, so it was not performed. The parent observes
my context and budget, as the packet says.
