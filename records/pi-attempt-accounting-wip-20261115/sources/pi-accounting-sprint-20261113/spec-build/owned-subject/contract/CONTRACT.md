# Pi accounting contract revision 6 (proposed, unreviewed)

Contract `pi-accounting-contract/1.5-s1`. Revision 5 immutable predecessor remains separately preserved. C–K normative definitions are owned solely by [spec/ck.json](../spec/ck.json), rendered in [CK-GENERATED.md](CK-GENERATED.md). Generated text is a view, not a second semantic authority. This file retains boundary owners E/L/T/R; duplicated C–K statements below are superseded by canonical IDs. No reducer, freeze, live verifier or S2a readiness is claimed.

Stage DAG: E → C → {S,V} → F → X → K. S reads no V result; shared pricing leaves inspect evidence/rates. F owns reservation replacement and amount_source. K findings cannot feed amounts. L/T/R consumer registration is in spec/dependencies.json.

### 2.2 Spending mechanisms are kept apart (R-04, P1.30; OQ-3, P1.32)

| # | Mechanism | Source today | Is it a provider-billing bound? |
|---|---|---|---|
| 1 | **request-count stop** | `--max-requests`: the proxy refuses requests beyond a count | **no**: it limits how many requests are forwarded, not what each costs |
| 2 | **cost-proxy stop** | subscription `--max-usd`: an API-equivalent cost-proxy threshold checked before admission (`pi_container.py` subscription budget) | **no**: one admitted response can overshoot, and subscription billing is unknown |
| 3 | **proxy reservation constraint** | the proxy refuses a new reservation when settled + outstanding + worst case > the session limit (`:984-988`) | **no**: it gates reservations, and a reservation is not a bound (B-2, B-4, B-5, B-6) |
| 4 | **internal accounting commitment** | §12.3 `committed = max(0, cap − owned high)` | an allocation for admission arithmetic, not a bound |
| 5 | **unenforced declared cap** | the admission `max_usd` | **no**: `settle()` charges the served tier without a cap (`:994-1019`), and a retired session can resume under it (§2.3) |
| 6 | **enforced provider-billing bound** | an `ExecutionGate` mechanism `billing_bound` (§2.3) | **yes, only when qualified**: enforcement across forwarding, retries, failures, incomplete streams and settlement must be evidenced. No current producer evidence qualifies |

Unknown additional provider exposure stays `High::AtLeast` and commitment kind `AtLeast`, and blocks (D5). Renaming a
mechanism changes no expectation.

### 2.3 Execution gates and capability profiles (OQ-1, owner disposition P1.32)

**Producer fact (release `92a03789`).** Retirement does not stop a Pi session from spending. `role_host status`
reports a retired Pi session as `stopped` (`role_host.py:1504`), `PiHost.deliver` refuses only a `running` or
`absent` session (`:1543-1550`), and the new turn starts under the original session cap and prior spend (`:1256`,
`:1305`), without reading the objective admission journal. So, for the current producer:
- retirement and idleness are activity labels, not enforcement;
- outstanding exposure and the ability to resume without admission are **separate facts**: the first is the owned
  lines (§12.1), the second the commitment (§12.3);
- settled past requests do not make a session safe to count as finished;
- a declared cap is not a provider-billing bound (§2.2).

**`ExecutionGate` evidence (for the future Rust vertical; no current producer writes it).**

```text
ExecutionGate = { gate_id, authority: { id, kind }, session: SessionId, scope: Session | Request{request_id},
                  effective_from: Stamp, effective_until: Stamp | Open, writer: Authority | SameUidFile | Declaration,
                  mechanisms: { new_forwarding: Mechanism, retries: Mechanism, reopening: Mechanism, billing_bound: Mechanism } }
Mechanism     = Enforced{ receipt: { ref, sha256 } } | Absent
BillingBound  = Enforced{ receipt, bound: { amount_nano, currency, scope, covers: [Activity], binding_sha256 } } | Absent   // mechanism billing_bound
Activity      = forwarding | retries | partial_streams | failures | settlement
```

A gate **qualifies** only if every rule holds (otherwise it is ignored for commitments and its failures are recorded
as `gate.reasons`):

| Rule | Statement |
|---|---|
| G-1 Authority | `authority.kind = vertical_admission_authority`, its `id` equals the objective's declared admission authority, and `writer = Authority`. A same-UID-writable file, a declaration, or a plain `gated: true` field is not evidence (`GateWriterNotAuthority`, `GateAuthorityMismatch`) |
| G-2 Identity | `session` equals the session whose commitment is being decided (`GateSessionMismatch`) |
| G-3 Scope | scope `Session`, which covers every request of the session. A `Request` scope covers one request and cannot release or bound a session commitment (`GateScopeInsufficient`) |
| G-4 Period | `effective_from` is no later than the session's admission (`launched` row), and `effective_until` is open or no earlier than the reference time (`GatePeriodInsufficient`) |
| G-5 Mechanisms | **to release a retired session's commitment:** `new_forwarding`, `retries` and `reopening` all `Enforced` (refusing delivery after retirement alone does not prove that an active session cannot create further exposure). **To make an open or pending commitment finite:** `billing_bound` and `retries` `Enforced` (OQ-3), with a bound satisfying G-8. Otherwise `GateMechanismMissing` |
| G-6 Receipts | every `Enforced` mechanism used carries a receipt with a reference and a sha256 (`GateReceiptMissing`) |
| G-8 Billing bound (A-03, owner disposition P1.33) | to make an open or pending commitment finite, `billing_bound` must carry a **numerical enforced bound**: `amount_nano` a non-negative integer no greater than the admission cap (a bound above, or not comparable with, the authorized cap does not qualify); `currency = USD`; `scope = session` for this session; `covers` all of `forwarding`, `retries`, `partial_streams`, `failures` and `settlement`; and `binding_sha256` equal to the sha256 of the canonical JSON (sorted keys, no spaces) of `{amount_nano, authority_id, covers, currency, effective_from, effective_until, gate_id, session}`. A declared cap, a mechanism flag without a bound, or a same-UID-writable assertion does not qualify (`GateBoundMissing`, `GateBoundExceedsCap`, `GateBoundCurrency`, `GateBoundCoverage`, `GateBoundBinding`). **Authentication limit:** the digest binds the fields to one another and to the receipt. Verifying the authority's signature over them needs key material that S1 does not have, so it is the future vertical's evidence-adapter obligation (stage E) |
| G-7 Consistency | no evidence contradicts the gate: a session whose lifecycle is `reopened` after `effective_from` while `reopening` is claimed `Enforced` makes the gate **contradicted**, blocking `GateContradicted` for the admitting objectives |

A qualified gate never changes an attempt's money (stages C to X). It decides only whether a commitment is released
or finite (§12.3), so outstanding exposure still blocks through the owned lines.

**Capability profiles (corpus).** Every corpus case declares one:
- `current_producer`: what release `92a03789` can establish. No gate evidence is attached, and objectives use the
  legacy journal and closure records.
- `future_qualified_gate`: synthetic normative cases for the unfinished Rust vertical, with explicit gate and closure
  evidence from a `vertical_admission_authority`. They are `producer_unreachable` today, carry no A1.3.4 historical
  comparison, and are counted separately.

## 10. Session lifecycle (stage L)

```text
SessionLifecycle = { turns: [Turn{n, start: Stamp, source}], retirements: [Stamp|Undated], state, reopened, clock }
state    = Open | Retired{at} | Contradicted{reason}          // the terminal state, from the last lifecycle row
reopened = None | Some{retired_at, reopened_turn}             // a deliver followed a retire at some point (R-10)
clock = Trusted | Untrusted{ AheadOfReference | ProxyRowsOutOfOrder | LifecycleContradicted }
```

| Rule | Statement |
|---|---|
| L-1 (G1) | Dated lifecycle rows never go backwards in append order; otherwise `Contradicted` |
| L-2 (G2) | A turn's proxy rows are not dated before its start row; otherwise `Contradicted` |
| L-3 (G3) | A turn's proxy rows are not dated after any dated retirement that follows it; otherwise `Contradicted` |
| L-4 | `Retired{at}` iff the last lifecycle row is a dated retire and L-1 to L-3 hold, whether or not the session was reopened earlier. An undated retirement bounds nothing |
| L-5 | `reopened = Some` iff a `deliver` follows a retire anywhere in the rows (D2: no atomic re-admission mechanism exists, so blocking `ReopenedWithoutAdmission` for the admitting objectives). L-4 and L-5 are independent facts: retire, deliver, retire gives `Retired{at}` with `reopened = Some` (R-10) |
| L-6 | `clock = Untrusted` if any proxy row is dated after the reference time, dated proxy rows go backwards in append order, or `Contradicted` |
| L-7 (OQ-1) | `Retired{at}` is a fact about the lifecycle rows, not an execution gate. It releases a commitment only together with a qualified `ExecutionGate` (§2.3, §12.3) |

## 11. Temporal attribution (stage T)

### 11.1 Ownership (D1)

```text
Ownership = Owned{ objective, admission: EvidenceRef }
          | Uncertain{ candidates: [Objective], cause }      // conflicting or several admissions; a reopened turn
          | Unadmitted{ candidates: [Objective] }            // no admission record covers the session (O-6)
          | Unowned                                          // no candidate at all
```

| Rule | Statement |
|---|---|
| O-1 | An attempt is owned by the objective whose admission record (journal `launched` row, or a legacy `objective_admission` row) admitted the attempt's session **for the turn that contains the attempt** |
| O-2 | Turns started after a retirement (reopened) are covered by no admission until an atomic re-admission exists (D2), so their attempts are `Uncertain{cause: ReopenedWithoutAdmission}` |
| O-3 | Several admitting objectives for one session, or none established, give `Uncertain`. Session ancestry may define additional **candidates** (which objectives must block) but never assigns ownership |
| O-3a | **Authority evidence is never filtered away (R-08).** Every objective with an admission record naming the attempt's session is a candidate of an `Uncertain` attempt, whatever its tree or window. Ancestry and time can add candidates; they never remove an admitting objective. A contradiction between admission and time (an owned attempt wholly outside its owner's window) stays visible as `AttributionContradiction` (Table T) |
| O-4 | An attempt in a session that no objective tree contains and no admission names is `Unowned`; it affects no objective |
| O-5 | An attempt whose pool is `Codex` (an established subscription lane, no dollar evidence) affects only Codex reporting. Its ownership uncertainty does not block `pi_api` admission. A lane-contradicted attempt is `PiApi`, so O-3 applies |
| O-6 | **Unadmitted historical evidence (Q8).** An attempt in a session that no admission record covers is `Unadmitted{candidates}`. This includes every unbudgeted API session of the current producer, which writes no admission. Its observed financial facts are kept unchanged (stages C to X). No admission authority is synthesised for it |
| O-7 | **Candidates.** The candidates of an `Unadmitted` attempt are the objectives whose tree contains its session **and** whose window could contain its §11.2 interval. The candidates of an `Uncertain` attempt are its admitting objectives (O-3a) plus the same tree-and-time set. "Could contain" uses the obligation interval, never an isolated row stamp (R-12): an `Outstanding` interval runs to its bound (the following dated retirement, or open). A candidate is dropped only when the interval lies wholly `Before` or `After` the window on a trusted clock; an unplaceable interval (P-2: untrusted clock, missing stamps without consistent neighbours, inversion) keeps every tree candidate. Only candidates block, with `HistoricalUnadmitted` or `OwnershipUncertain`; unrelated objectives never do |
| O-8 | **Vertical execution.** In the Work Engine vertical, metered execution requires explicit admission before execution. An attempt whose producer features mark it as vertical-produced (`attempt_id` present) and which no admission covers is blocking `UnadmittedExecution` for its candidates. Historical evidence never establishes that authority retroactively |

### 11.2 Placement

```text
Interval  = Closed obligation: [reservation_at, settlement_at] | Outstanding: [reservation_at, bound]
            | Unbudgeted: [row_at, row_at]
bound     = the dated retirement following the attempt's turn (L) | Open
Placement = Inside | Before | After | Straddles{Start|Closure} | Unplaceable{cause}
```

| Rule | Statement |
|---|---|
| P-1 | Placement is computed against the **owner's** window [start, closed_at or open], using `closed_at` only when the closure is established (§12.4). Otherwise the window is treated as open for placement |
| P-2 | `Unplaceable` if the session clock is untrusted, a required stamp is missing with no consistent dated neighbours (A1.3.x neighbour rule: nearest dated rows before and after, in append order, must not invert), or the interval is inverted |
| P-3 | Stamps after the reference time make the session's clock untrusted (§10), so every attempt in it is `Unplaceable` (repairs S-03, C1, C2) |
| P-4 C1 rule | An attempt owned by O and reserved inside O's window **stays O's** even when it settles or remains outstanding after O's closure. O's totals are `provisional` until every owned attempt is closed or bounded by a dated retirement |

**Rule P-5. T modifies no amount.** It returns ownership and placement only.

## 12. Objective rollup and admission-facing output (stage R)

### 12.1 Output

```text
ObjectiveAccounting = {
  objective, window, status: Established | NotEstablished{ reasons: [Reason] }, provisional: bool,
  owned_closed: ExposureSum, owned_outstanding: ExposureSum,     // owned attempts placed Inside (or C1-retained)
  predecessor_obligations: ExposureSum,                          // §12.2
  commitments: [Commitment],                                     // §12.3
  uncertain: [AttemptKey], blocking: [Uncertainty], diagnostics: [Diagnostic] }
```

**`reasons`** uses the §9 blocking codes plus:
- `OwnershipUncertain`: an uncertain `PiApi` attempt names the objective as a candidate;
- `HistoricalUnadmitted`: an unadmitted `PiApi` attempt names the objective as a candidate (O-6, O-7);
- `PlacementUnplaceable`, `PlacementStraddles`: an owned attempt is unplaceable or straddles the window;
- `UnenforcedCommitment`: a commitment of kind `AtLeast` (§12.3);
- `GateContradicted`: a gate for an admitted session is contradicted (§2.3 G-7);
- `ClosureNotEstablished`: the objective's record claims a closure (`closed_at` or a close entry) that is not
  established (§12.4).

**Rollup lines:**
- `owned_closed` sums owned attempts whose obligation is `Closed` or none (unbudgeted and orphan attempts);
- `owned_outstanding` sums owned `Outstanding` attempts;
- owned attempts are summed whatever their placement, including `Before` and `After` (money is never moved by
  placement, P-5), and placement problems add reasons.

`Established` iff **all** of:
- no blocking finding scoped to the objective's owned or candidate attempts and sessions;
- no `AtLeast` in any summed line;
- no owned attempt `Straddles` or `Unplaceable`;
- no `Uncertain` or `Unadmitted` attempt naming the objective as a candidate;
- every commitment well-formed and of kind `Exact` (§12.3);
- no claimed closure that is not established (§12.4).

Otherwise `NotEstablished` with every reason.

### 12.2 Predecessor obligations (D1; revised for R-07, owner disposition P1.30)

- An objective record MAY name its predecessor (`predecessor` field).
- The successor's `predecessor_obligations` = the high exposure of every predecessor-owned attempt whose **billing is
  `Unresolved`** (§6.2: `Outstanding`, or `Closed` with an `AtLeast` high), plus the predecessor's open commitments
  (pending, launched-open, retired-resumable, reopened; §12.3). The line is `AtLeast` if any term is.
- **Settled predecessor spend**, meaning owned attempts whose billing is `Established`, is never included.
- Execution completion does not discharge an obligation. An attempt that closes on an upstream error, an error status,
  a reservation basis or incomplete usage stays in the successor's line until its billing is established.
- Established closure of the predecessor (§12.4) refuses new admission to the predecessor. It does not discharge the
  predecessor's obligations; they keep constraining the successor.
- An attempt appears in exactly one objective's owned lines and at most once in one successor's predecessor line, so
  nothing is counted twice within an objective.
- Shared-root objectives without a `predecessor` link: no constraint is inferred (XO-1 remains a known open issue).

### 12.3 Commitments (computed for the admission authority; enforcement stays outside the core)

```text
Commitment = { admission_id, session: Option<SessionId>, cap: Nano,
               state: pending | launched_open | retired | retired_resumable | released | reopened | invalid,
               committed: Nano, kind: Exact | AtLeast, gate: { qualified: bool, reasons: [GateReason] } }
```

| Rule | Statement |
|---|---|
| A-1 (S-08) | Caps are well-formed amounts > 0, and owner limits ≥ 0. Malformed, negative, zero or non-finite values give `Invalid` (committed 0, `Exact`), so the objective is `NotEstablished{CommitmentInvalid}` |
| A-2 | One `pending` per admission. `released` with `launched` or `runner_starting` is `Invalid`. Pending without `launched` or `released` is committed in full (`committed = ceiling`: the cap, or a qualified G-8 bound, A-03). A valid `released` row releases it: no session was launched, so nothing can resume (committed 0, `Exact`) |
| A-3 (OQ-1) | **Launched.** `committed = max(0, ceiling − the session's owned high)`, where `ceiling` is the admission cap, or the qualified G-8 `amount_nano` (never above the cap) when a qualified billing-bound gate exists (A-03). The state follows the lifecycle: `launched_open` (not retired, not reopened), `reopened` (reopened, not retired), or, when the lifecycle state is `Retired`: `retired` if a qualified gate establishes release (G-5), otherwise **`retired_resumable`**, which keeps the commitment (L-7). Only `retired` (committed 0, `Exact`) releases. Owned spend above the cap is `NotEstablished{CommitmentInvalid}` |
| A-4 | A session with `reopened = Some`: whatever the state, the objective is `NotEstablished{ReopenedWithoutAdmission}` until a qualified admission covers the reopening (D2). A reopening never releases a commitment and never establishes safety. If a gate claims `reopening` enforced, the reopening contradicts it (G-7) |
| A-5 (R-04, Q10, OQ-1, OQ-3, A-03) | **Kind.** `pending`, `launched_open`, `retired_resumable` and `reopened` commitments are `AtLeast(committed)` with blocking `UnenforcedCommitment`, unless a qualified gate with an enforced, numerically bound `billing_bound` (G-5, G-8) makes a `pending` or `launched_open` commitment `Exact(committed)`. `retired`, `released` and `invalid` are `Exact(0)`. Every commitment records `gate.qualified` and the failed rules (§2.3). For the current producer: `qualified = false`, reason `NoGateEvidence` |

### 12.4 Admission (outside the core, for corpus expectations; revised for R-09, Q11, OQ-2, OQ-4)

The decision for a request `new` against objective O is taken in this order:

1. **Closure (Q11, OQ-4).** O's closure evidence:
   - **no claim** (`closed_at` null and no close entry): open;
   - **established**, by profile:
     - `current_producer` (**legacy cooperative trust**): a `scope_budget close` entry `{at, disposition, reason,
       set_by}` with `at` equal to `closed_at`; `closed_at` a well-formed RFC 3339 UTC stamp no earlier than
       `started_at` and no later than the reference time; and no contradictory evidence (no admission row for O dated
       after `closed_at`, no second close entry with another time). `set_by` is not evidence of owner identity. A
       same-UID agent can write this record, and S1 cannot prove otherwise. This is operational closure under
       cooperative tooling, not a tamper-resistant authorization;
     - `future_qualified_gate`: a closure record from O's declared `vertical_admission_authority` with a receipt
       reference and sha256, and the same time and coherence conditions. The legacy file format alone does not
       establish authority;
   - **claimed but not established** (anything else, including a `closed_at` with no close entry): the decision is
     `not_established`, and O carries reason `ClosureNotEstablished`.

   An established closure gives **`refused`** before any monetary evaluation. This is an authorization result, not
   evidence that O's accounting is complete: unresolved obligations remain recorded, O's totals stay provisional while
   any owned attempt is unresolved, and its obligations keep constraining successors (§12.2).
2. **Limit.** If O's limit is not a well-formed amount ≥ 0, the decision is `not_established`.
3. **Known over-limit (OQ-2).** `known = owned_closed.high.amount + owned_outstanding.high.amount +
   predecessor_obligations.high.amount + Σ committed (non-invalid commitments) + new`, counting the known amount of
   every `AtLeast` term. Each attempt contributes once (its reservation or its settlement-derived amount, F2), and
   only **owned** attempts count: `Uncertain` and `Unadmitted` attempts are never attributed speculatively. If
   `known > limit`, the decision is **`refused`**, even while billing uncertainty remains. This is refusal under the
   conservative budget policy, not a statement that the provider invoice is known, and O's reasons are unchanged.
4. **Accounting.** Otherwise, if O is `NotEstablished`, the decision is `not_established` (the known portion is within
   the limit, but unknown exposure may exceed it).
5. **Headroom.** Otherwise every term is `Exact`: `admit` iff `known ≤ limit`. (Step 3 has already refused `known >
   limit`.)

`refused` and `not_established` are equally non-permissive (A5); they differ only in meaning.


## Revision6 gate trust boundary

POLICY-R6 supersedes any implication above that raw writer/authority/hash alone establishes verification. Gate assertions are untrusted. Qualification requires GateVerification from a separately trusted adapter, or top-level trusted_fixture_inputs supplied by a test-only harness bound to exact payload; output records test_mock provenance. No producer row can construct it. Real verification authority/channel/enforcement remains unestablished. COMMITMENT-RESIDUAL is separate allocation and cannot resolve AtLeast attempt or aggregate exposure.
