# Pi attempt-accounting contract, S1 (normative)

**Contract version:** `pi-accounting-contract/1.4-s1` (revision 5, P1.33: revision items R5-01 to R5-09,
of which R5-01, R5-02, R5-03 and R5-06 are owner dispositions; see `DECISION-REGISTER.md`). Revision 4 `1.3-s1` (P1.32), revision 3
`1.2-s1` (P1.30), revision 2 `1.1-s1` (P1.29) and revision 1 `1.0-s1` (P1.28) are preserved unchanged.
**Status:** proposed normative contract, not reviewed in this revision and not implemented.

**Producer under contract:**
- release `92a03789`: `pi_container.py` `78083734…`, `role_host.py` `cbca1bc9…`;
- rate authority: `openai_api_rates.json` `c0bdfd66…` and `anthropic_api_rates.json` `8a286433…`.

**Owner decisions:** D1–D10 (`DECISION-REGISTER.md`) are binding inputs.

**Not normative:**
- the deployed reader (`ab2a0f84`, development tooling, unchanged);
- A1.3.4 (`086bdec1`, a historical comparison baseline only).

Key words: **MUST** and **MUST NOT** are requirements. "Blocking" means the finding makes every affected objective's
admission facts `NotEstablished` (§12).

---

## 1. Scope and stages

| Stage | Where | Input | Output | May read |
|---|---|---|---|---|
| **E** evidence adaptation | outside the core (D3) | bytes of every log a turn record names, role_host ledger rows, objective records, admission journals, rate files | `EvidenceSet` (§3) | files only |
| **C** correlation | core | `EvidenceSet` | `AttemptGroup`s plus `NonAttemptEvidence` (§4) | E |
| **S** settlement | core | `AttemptGroup`, `LaunchFacts`, pricing primitives (§7.1) | `SettlementDecision` plus `Obligation` (§5, §6) | C |
| **V** valuation | core | `AttemptGroup`, `LaunchFacts`, pricing primitives | `Valuation` (§7) | C |
| **X** exposure | core | S plus V outputs, lane decision | `Exposure` (§8) | C, S, V |
| **K** classification | core | the complete facts of C to X | `Vec<Uncertainty>`, `Vec<Diagnostic>` (§9) | C, S, V, X |
| **L** lifecycle | core | lifecycle rows of one session | `SessionLifecycle` (§10) | E |
| **T** attribution | core, separate module | X outputs (time facts, attempt keys), L, `AdmissionRecords`, `ObjectiveFacts` | `Ownership` plus `Placement` per attempt (§11) | X, L, K (read-only) |
| **R** rollup | core | T, X, K, L, commitments (§12.3) | `ObjectiveAccounting` (§12) | all above, read-only |
| admission decision | outside the core | closure state, R and the requested allocation (§12.4) | admit, refuse or not established | R |

**Dependency direction (MUST):** `E → C → S → V → X → K`.
- **Pricing primitives** (§7.1) are leaf functions shared by S (the producer-consistency check) and V (bounds). They
  read only evidence and the rate table.
- **No stage reads a later stage's output.** K's findings are never an input to S, V, X or T's placement
  computation.
- **T and R** may copy blocking findings into their outputs but MUST NOT modify any amount.

**Pure and deterministic.** Stages C to R are pure functions. Output is canonical (§13), and equal inputs give
byte-identical outputs. The core does no I/O, reads no clock (the reference time is an input) and never panics on
producer input.

## 2. Money (D10)

- **Canonical amount:** `Nano = i128` nano-USD, with checked arithmetic. Overflow is `Err(Overflow)`, which callers
  MUST treat as blocking `ArithmeticFailure`. Values never wrap or saturate.
- **Rates:** parsed from the rate files' decimal literals, never via binary floats, into nano-USD per token, an exact
  integer for every current rate and multiplier (`MONEY.md`: 163 of 163). A rate or multiplier that is not an exact
  integer in nano-USD per token is `Err(UnsupportedPrecision)`: rejected at load, with the rate table marked
  unusable, so every valuation is `Unpriceable{RateTableUnusable}`.
- **Producer amounts** (`charged_usd`, `reserved_usd`) are decimal literals with at most 6 fractional digits, converted
  exactly. More digits, exponent forms whose value is not an exact nano amount, negative values, booleans, strings,
  NaN or infinities are `Malformed`.
- **Rounding (R-02).** The core never rounds a current amount: valuations are exact integers, and producer amounts
  are taken exactly as logged. The producer's own rounding (`round(x, 6)` USD, `pi_container.py:1018-1019`) moves a
  logged amount up or down from the producer's float value. It can never lower conservative exposure:
  - a logged **charge** enters a high only as a term of `max(charged, usage bound)` (FB-1, §8), so rounding down
    leaves the high at the usage bound or above, and rounding up only raises it;
  - a logged **reservation** is a known amount, never a bound (FB-2), so its rounding creates no finite bound;
  - the supported **low** is a usage valuation, never a producer amount (F11).

  If a future rate table needs rounding, every rational amount MUST be rounded once per bound, low by floor and high
  by ceiling (`MONEY.md`).
- **Producer-consistency tolerance.** `T_round = 501` nano. The producer's logged charge differs from its float value
  by at most 500 nano (6-place rounding), and the float value differs from the exact price `p` by under 1.2 nano for
  `p < 10^15` nano (10^6 USD). Both the logged charge and `p` are integers in nano, so their difference is at most 501
  (`MONEY.md`). For `p ≥ 10^15` nano the bound is not proven, and the settlement is `Rejected[ToleranceRange]`.

```text
High  = Exact(Bound) | AtLeast(Bound)    // AtLeast: `amount` is supported; more is unresolved; never a finite maximum (D5)
Bound = { amount: Nano, basis: AmountBasis, evidence: [EvidenceRef] }
AmountBasis = ProxyCharge | ProxyReservation | UsageValuation{identities, tier_multiplier, split}
            | ProviderBilling          // reserved; no evidence source today; never produced in S1
            | None                     // amount 0 with no support (for `low`)
Exposure = { low: Bound, high: High }  // invariant: low.amount ≤ high.amount
```

**Order on `High`:**
- `Exact(a) ≤ Exact(b)` iff a ≤ b;
- `Exact(a) ≤ AtLeast(b)` iff a ≤ b;
- `AtLeast(a) ≤ AtLeast(b)` iff a ≤ b;
- `AtLeast(a) ≤ Exact(b)` is never true.

**Sum:**
- `Exact + Exact = Exact`;
- any `AtLeast` term makes the sum `AtLeast` of the known sum.

**Bases are never merged:** a sum carries a basis histogram, not one basis.

**Evidentiary standard, stated rather than verified:** usage valuations price provider-reported usage at the published
rates. Verified provider billing is unavailable, so every `Exact` high is conditional on that standard.

### 2.1 Finite-bound rule (owner disposition Q2)

Four amounts are kept apart and never stand in for one another:

| Amount | Basis | Meaning |
|---|---|---|
| reserved | `ProxyReservation` | what the proxy set aside before forwarding |
| conservatively charged | `ProxyCharge` | what the proxy kept against its own session limit |
| provider bill | `ProviderBilling` | established only by bill evidence; **no source exists today, never produced** |
| unknown additional billing | `High::AtLeast` | what remains unresolved |

**Rule FB-1 (usage bound; revised for R-01, R-02, R-03).** A high may be `Exact` only when the attempt has a **usage
bound**: the high of a `UsageValuation` of **complete** provider-reported usage (V-5 `complete = true`), over an
established identity set (one identity, or several identities each priced, §8) and an established tier set (V-3).
The stated high is `Exact(max(usage bound, the known amounts §8 adds))`. A proxy charge above the usage bound raises
the stated high and gives it its basis (X-0, for example `ProxyCharge` after upward rounding), but the high is finite
because the usage bound exists, not because of the charge. **Without a usage bound, no known amount makes a high
finite.**

**Rule FB-2.** Proxy reservations, proxy charges and declared session caps are known amounts, never upper bounds on
provider billing. A high with no usage bound is `AtLeast(largest known amount)`.

**Monetary measures (R5-01, R5-02, owner disposition P1.33).** Six facts are kept apart for every attempt. None stands in
for another:

| Fact | Where it is recorded | Measure |
|---|---|---|
| terminal execution | `Execution` (§6.2) | none (a time fact) |
| structurally valid settlement | `SettlementDecision::Valid` (§5) | none (a validity fact) |
| complete and priceable usage | `Valuation` with `complete` and established sets (§7, FB-1) | **usage valuation**: what the reported usage costs at published rates |
| eligibility to replace the reservation | `Replacement` (below) | none (a decision fact) |
| proxy charge and proxy reservation | `settlement.charged`, `R` | **proxy amounts**: what the proxy kept and set aside |
| independently established provider billing | `AmountBasis::ProviderBilling` | never produced in S1 |

**Exposure is one measure: conservative provisional exposure**, the amount the admission accounting must hold for the
attempt. Its `low` is supported usage valuation, never a proxy amount. Its `high` is either:
- `Exact(…)`, when the reservation is **replaced** by an eligible usage settlement (FB-1); or
- `AtLeast(known)`, where `known` is the largest of the known components the conservative policy must hold: the
  retained reservation `R`, the proxy charge, and the usage valuation's high or `priced_high`. Each component is a
  lower bound of what the policy holds, with its own basis (X-0). The maximum is taken within this one measure, not
  across measures: provider billing is never claimed, and the usage valuation keeps its own record in `Valuation`.

```text
Replacement = Eligible                                   // Valid(Usage) with a usage bound (FB-1): the reservation is replaced
            | Ineligible{ reason: NotUsageSettlement      // no valid usage settlement (absent, rejected, conflicting, other bases)
                                | UsageIncomplete          // valued, but not complete (V-5)
                                | UsageUnpriceable         // tier set or identity set not established, malformed counts
                                | NoReservation }          // nothing to replace (unbudgeted, orphan, subscription)
```

**A structurally valid settlement is not proof of complete billing.** Only `Eligible` replaces reservation-based
exposure. An ineligible valid settlement closes the attempt in time and keeps the reservation in the conservative
measure (`amount_source = ReservationBound`, §6), and its supported partial usage stays as the low.

**Why the reservation is not a bound.** It is the producer's worst case, but its conditions are not all
producer-enforced (`RESERVATION-BOUNDS.md`):
- the output cap is enforced;
- the input bound is a byte estimate, and the producer marks it "assumed, not verified" for inline images;
- the served tier is not enforced: the proxy itself charges a response at whatever tier was served;
- no long-context rule is applied to Anthropic;
- the provider's billing of errored or aborted requests is unknown.

**Consequence (Q10, P1.30; OQ-1, P1.32):** under D5, an objective is `NotEstablished` while it owns or may own (as a
candidate) any attempt without a usage bound, or holds any commitment of kind `AtLeast` (§12.3). That covers in-flight,
unsettled, error-status, upstream-error, reservation-basis and rejected attempts, and **every admitted session of the
current producer, retired or not**, because a retired Pi session can be resumed without new objective admission
(§2.3). Only a qualified `ExecutionGate` (§2.3) releases or bounds a session's commitment. No current producer evidence
qualifies.

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
| G-8 Billing bound (R5-03, owner disposition P1.33) | to make an open or pending commitment finite, `billing_bound` must carry a **numerical enforced bound**: `amount_nano` a non-negative integer no greater than the admission cap (a bound above, or not comparable with, the authorized cap does not qualify); `currency = USD`; `scope = session` for this session; `covers` all of `forwarding`, `retries`, `partial_streams`, `failures` and `settlement`; and `binding_sha256` equal to the sha256 of the canonical JSON (sorted keys, no spaces) of `{amount_nano, authority_id, covers, currency, effective_from, effective_until, gate_id, session}`. A declared cap, a mechanism flag without a bound, or a same-UID-writable assertion does not qualify (`GateBoundMissing`, `GateBoundExceedsCap`, `GateBoundCurrency`, `GateBoundCoverage`, `GateBoundBinding`). **Authentication limit:** the digest binds the fields to one another and to the receipt. Verifying the authority's signature over them needs key material that S1 does not have, so it is the future vertical's evidence-adapter obligation (stage E) |
| G-7 Consistency | no evidence contradicts the gate: a session whose lifecycle is `reopened` after `effective_from` while `reopening` is claimed `Enforced` makes the gate **contradicted**, blocking `GateContradicted` for the admitting objectives |

A qualified gate never changes an attempt's money (stages C to X). It decides only whether a commitment is released
or finite (§12.3), so outstanding exposure still blocks through the owned lines.

**Capability profiles (corpus).** Every corpus case declares one:
- `current_producer`: what release `92a03789` can establish. No gate evidence is attached, and objectives use the
  legacy journal and closure records.
- `future_qualified_gate`: synthetic normative cases for the unfinished Rust vertical, with explicit gate and closure
  evidence from a `vertical_admission_authority`. They are `producer_unreachable` today, carry no A1.3.4 historical
  comparison, and are counted separately.

## 3. Evidence (stage E output, the core's input)

```text
EvidenceRef  = { source: SourceId, line: u32, sha256: Digest }          // SourceId = logical path + file sha256
Field<T>     = Present(T) | Null | Absent | Malformed(JsonType)
EvidenceItem = { ref, epoch: ProducerFeatures, kind: EvidenceKind }
EvidenceKind =
    Reservation  { n: Field<u64>, rid: Field<Rid>, reserved: Field<Nano>, at: Field<Stamp> }
  | Settlement   { event: Request|UpstreamError, rid: Field<Rid>, n: Field<u64>, basis: Field<Basis>,
                   charged: Field<Nano>, reserved: Field<Nano>, status: Field<u16>, usage: Field<Usage>,
                   model: Field<String>, response_model: Field<String>, response_id: Field<String>,
                   observed_tier: Field<String>, terminal: Field<String>, model_mismatch: Field<bool>,
                   labels: Labels, at: Field<Stamp> }
  | Request      { same fields as Settlement, no `usd` }                 // unbudgeted API, or subscription
  | UpstreamError{ n, labels, at, detail }                               // no `usd`
  | Admission    { rid: Field<u64>, n: Field<u64>, model: Field<String>, labels, at }
  | Measurement  { attached to its row: rid: Field<u64>, status, cost_proxy }
  | StreamError  { n: Field<u64>, shape: ReadFailure|HeaderWrite, labels, measurement: Option<Measurement>, at }
  | IdentityRefusal { n, reason, authorized: Field<String>, served: Field<String>, response_id, labels, at }
  | Refused      { status, reason, labels, at }
  | Unreadable   { reason: NotJson | NotObject | PartialFinalLine | UnknownEvent(String) }
Labels = { provider: Field<String>, auth_lane: Field<String> }
Basis  = Usage | Reservation | ErrorStatus | Unknown(String)
Usage  = OpenAi{ input, output, cached: Field<u64>, cache_write: Field<u64>, reasoning }
       | Anthropic{ input, cache_read, cache_write, output, w5m: Field<u64>, w1h: Field<u64> }
```

**Adapter obligations (stage E, specified here because the core relies on them):**
- **E-1 Totality.** Every non-blank line of every proxy log named by a turn record is exactly one `EvidenceItem`. A
  line that is not JSON, not an object, or the final line without a newline is `Unreadable`. Nothing is skipped.
- **E-2 Fidelity.** Fields are recorded as written (`Absent`, `Null`, `Malformed`). No defaults are applied in E.
- **E-3 Shape.** `StreamError.shape = ReadFailure` iff the row has a `detail` key, else `HeaderWrite`
  (`pi_container.py:1295`, `:1513`, `:1546`).
- **E-4 Producer features.** `ProducerFeatures` are feature flags detected per session, not dates:
  `usd_accounting`, `cache_write_field`, `anthropic_route`, `subscription_lane`, `cost_proxy`, `attempt_id`.
- **E-5 Inventory facts.** E also emits inventory evidence:
  - turn numbers recorded;
  - logs found on disk but not referenced, copied (byte-identical) or misplaced;
  - missing logs;
  - logs shared by two turns.

  These become `InventoryEvidence` with the session scopes that could own them (§9; R-03, A1-03, S-05, V-07).

`LaunchFacts` per session: `pinned_model` (`provider/model` on the launch row), `auth_lane`, `parent`, `turns`.

**Model canonicalisation (R5-05; rule version `mc-1`).** Pricing identity is never established by fuzzy matching.
- **MC-1 Launch model.** A launch model is `<provider>/<model_id>`, with `<provider>` one of `openai`, `openai-codex` or
  `anthropic`, split at the **first** `/`. That is exactly how the producer derives the proxy's `model_id`
  (`role_host.py:1280`: `opts["model"].split("/", 1)[1]`). The canonical pinned identity is `(provider family,
  model_id)`, where `openai` and `openai-codex` are the OpenAI family. A launch model with no `/`, or with another
  provider, has **no canonical identity**: `ModelUnresolved`.
- **MC-2 Row models.** A proxy row's `model`, `response_model` or refusal `served` value is a bare model id in the
  row's provider family (the `provider` label, or OpenAI when absent). It is used exactly as written. No case folding,
  no prefix stripping, no version or date normalisation.
- **MC-3 Equality and rates.** Two identities are equal iff their family and model id are byte-identical. Rates are
  looked up by exact model id in the family's rate file. A missing rate is `IdentityWithoutRate` (V-6). A
  `ModelUnresolved` pinned identity fails §5 condition 8 (`ModelMismatch`) and is an unpriceable identity (V-1).

## 4. Correlation (stage C)

```text
AttemptKey  = ProducerAttempt{incarnation, seq}      // proposed producer field (D7); none exist today
            | Reservation{source, rid}               // budgeted
            | Admission{source, rid}                 // subscription cost-proxy
            | Row{source, line}                      // unbudgeted request, or an orphan
Correlation = { key_join: Exact | Ambiguous{candidates: [AttemptKey]} | Unattributed,
                auxiliary: [{ evidence: EvidenceRef, rule: ShapeRule }] }      // RS-1..RS-3 joins (C-5 to C-7)
AttemptGroup = { key, correlation, reservations: [EvidenceItem], settlements: [EvidenceItem],
                 requests, errors, refusals, admissions, measurements, launch: LaunchFacts }
```

**Key join versus auxiliary rows (R-06).** `key_join` is the confidence of the attempt's own key: the
reservation–settlement join by `rid`, or the row itself for `Row` and `Admission` keys. Error and refusal rows joined
by a shape rule are listed in `auxiliary` with their rule. They add evidence (identities, diagnostics, forwarding) but
never close an obligation and never change `key_join`. F1 reads `key_join`.

| Rule | Statement |
|---|---|
| C-1 | A `Reservation` with a usable `rid` keys `Reservation{source, rid}`. Byte-identical repeats are one item (diagnostic `RepeatedIdenticalRow`). **Different rows with one `rid` are conflicting reservation evidence (R5-09):** all stay visible in the group, blocking `ReservationConflict` is raised, and the decision follows Table S1 (no settlement: `Absent`; one settlement: `Rejected[ReservationConflict…]`; two or more: `Conflicting`). Their independence is not established, so `R` is the largest of them (X-0), never their sum and never the smaller. **A reservation whose `rid` is absent or malformed** keys `Row{source, line}` with `key_join = Unattributed`. Its decision is `Absent` (no settlement can join it) and its obligation `Outstanding`, with blocking `MalformedEvidence(rid)` and `UnaccountedAttempt`. A malformed `reserved` contributes 0 to R and adds `MalformedEvidence(reserved)` (R-06) |
| C-2 | A `Settlement` joins `Reservation{source, usd.rid}` (`Exact`). An unusable `rid` keys `Row{source, line}` with the settlement only. A settlement with no reservation of that `rid` in its source keys `Row{source, line}` (`Exact`): decision `Rejected[NoReservation…]`, obligation `None{NoReservation}` (R-06) |
| C-3 | An unbudgeted `Request` (API lane, no `usd`) keys `Row{source, line}`, `Exact` (one row, one attempt) |
| C-4 | A subscription `Admission` keys `Admission{source, rid}`. A measured row joins the admission whose `rid` its measurement names, if that admission is earlier in the source (`Exact`). Otherwise it keys `Row` with `Unattributed` |
| C-5 | **Read failure (shape rule RS-1).** A `StreamError` with shape `ReadFailure`, unmeasured, joins the first later `Request` or `UpstreamError` in its source with the same `n` that no other error has claimed, **only if** `n` names exactly one forwarded attempt in the source: at most one `Request` or `UpstreamError` row and at most one distinct reservation `rid` with that `n` |
| C-6 | **Header-write failure (RS-2).** A `StreamError` not joined by C-5 joins a reservation with its `n` that no valid settlement closes and no other error has claimed, under the same uniqueness condition |
| C-7 | **Refusal pairing (RS-3).** An `IdentityRefusal` joins the first later `Request` with the same `n` and the same served model, one to one. Otherwise it joins a unique unsettled reservation with its `n` (RS-2's condition). Otherwise `Row`, `Unattributed` |
| C-8 | **Ambiguity.** An error whose `n` fails the uniqueness condition is its own `Row` attempt with `Ambiguous{candidates}`. An error with a missing or malformed `n` is a `Row` attempt, `Unattributed`. Ambiguity is never resolved by order, closeness or amounts |
| C-9 | `Refused` rows are `NonAttemptEvidence`; they never key, close or value an attempt and carry lane evidence only. A refused row with a **preflight** reason (every reason except `upstream unreachable` and an identity-refusal reason) records a request the proxy did not forward. A refused row with `upstream unreachable` or an identity-refusal reason is the proxy's client-side report **after** an attempt (`pi_container.py:1218`, `:1326`, `:1502`) and proves nothing about forwarding (R-13) |
| C-10 | **Measurement subtypes.** A measurement row is a `Request`, `UpstreamError` or `StreamError` with an attached `Measurement`; the attachment does not change the row's kind |
| C-11 | **Correlation never crosses sources.** A source is one turn's log; `n` and `rid` restart per proxy incarnation |

## 5. Settlement decision (stage S)

```text
SettlementDecision = Absent
                   | Valid{ basis: Usage|Reservation|UpstreamError|ErrorStatus, charged: Nano, settlement: EvidenceRef }
                   | Rejected{ reasons: [RejectReason], claimed: [Nano] }
                   | Conflicting{ settlements: [EvidenceRef], claimed: [Nano] }
                   | NotApplicable          // unbudgeted, subscription, or orphan attempt
```

**`Valid` iff every condition holds, otherwise `Rejected` with every failing reason** (codes in brackets):
1. Exactly one reservation and exactly one distinct settlement in the group [`ReservationConflict`, `NoReservation`].
2. The settlement is appended after the reservation [`Order`].
3. `n` is a positive integer on both (never boolean, string, float or zero) and equal [`AttemptNumber`].
4. `charged` and `reserved` are well-formed amounts [`AmountType`], and `reserved` equals the reservation's amount
   exactly [`ReservedMismatch`].
5. `basis` is present and known [`BasisMissing`, `BasisUnknown`]. An `UpstreamError` requires `Reservation`.
6. `Reservation` or `UpstreamError`: charged equals the reservation exactly [`KeepAll`].
7. **Status, by settlement path (R5-06, owner disposition P1.33).** A missing status is neither an automatic failure nor
   a universal exemption. It is accepted only where the producer writes no status:

   | Path | Settlement row | Status required | Absent or null | Producer evidence |
   |---|---|---|---|---|
   | `Usage` | `request` | integer 200 | `StatusType` | every request row carries `http_status: resp.status` (`pi_container.py:1549`, unchanged since `385f3174`; 1,588 of 1,588 real request rows are integers) |
   | `ErrorStatus` | `request` | integer, not 200 | `StatusType` | as above |
   | `Reservation` | `request` | integer | `StatusType` | as above |
   | `Reservation` (`UpstreamError`) | `upstream_error` | none | **accepted** | the producer writes no status on `upstream_error` rows (`:1497-1502`, `:1215-1217`) |

   A status that is present but not an integer (string, float, boolean, object) is `StatusType` on every path. An
   integer that contradicts the basis (non-200 with `Usage`, 200 with `ErrorStatus`) is `ErrorCharge`. `ErrorStatus`
   also needs charged = 0 [`ErrorCharge`].
8. `Usage`:
   - the usage is priceable under the producer's own rule (§7.1) [`UsageUnpriceable`];
   - the request `model` equals the canonical pinned `model_id` (MC-1 to MC-3) [`ModelMismatch`];
   - `producer_price < 10^15` nano [`ToleranceRange`], and `charged ≥ producer_price − T_round` [`Undercharge`].

`Conflicting` replaces `Valid` or `Rejected` when two distinct settlements name one reservation.

**Permissible transitions as evidence is added (the decision is a function of the evidence set; this lists how it may
change):**

| From | Adding | To |
|---|---|---|
| Absent | a well-formed valid settlement | Valid |
| Absent | an invalid settlement | Rejected |
| Valid or Rejected | a second, distinct settlement | Conflicting |
| Valid | a second, different reservation with the same `rid` | Rejected(`ReservationConflict`) |
| any | a byte-identical repeat | unchanged |
| Conflicting | anything | Conflicting |

**No other transition exists.** In particular there is no `Conflicting → Valid`, no `Rejected → Valid` without
removing evidence, and nothing makes a decision `Valid` because a numeric amount is present (`Valid` requires all eight
conditions).

## 6. Obligation: releasing versus replacing (stage S)

```text
Obligation = Outstanding                 // the reservation is an open obligation in time (§11.2)
           | Closed{ by: EvidenceRef, amount_source: SettlementAmount | ReservationConfirmed | ReservationBound }
           | None{ cause: Unbudgeted | NoReservation | Orphan | Subscription }   // no reservation exists (R-06)
```

`Obligation` is an **execution-time** fact: whether a reservation is still open in time. It is not a financial
disposition (§6.2).

**`amount_source` (R5-08).** It records where the conservative measure's amount comes from. It is constructed by stage
S from the decision and the replacement fact, and nothing else sets it:

| Variant | Constructed when | Meaning |
|---|---|---|
| `SettlementAmount` | `Valid(Usage)` with `Replacement::Eligible` | the reservation is replaced by the settlement and usage valuation |
| `ReservationConfirmed` | `Valid(Reservation)` or `Valid(UpstreamError)` (KeepAll: charged = R) | the proxy kept the whole reservation as its charge; the reservation stays as a known amount, `AtLeast` |
| `ReservationBound` | `Valid(ErrorStatus)`; `Valid(Usage)` with `Replacement::Ineligible`; `Absent`, `Rejected`, `Conflicting` with a reservation | the reservation is retained as provisional exposure without being the proxy's charge |

`amount_source` is absent only for `Obligation::None` (no reservation exists). Its JSON spellings are
`settlement_amount`, `reservation_confirmed` and `reservation_bound`.

The five distinctions the owner asked for:

| # | Concept | Contract term |
|---|---|---|
| 1 | Releasing a reservation | `Obligation::Closed`: the attempt has ended in time (its interval ends at the settlement). Only `Valid` closes. Closing never establishes billing (§6.2, R-07) |
| 2 | Replacing provisional exposure with a validated settlement | only `Valid(Usage)` with `Replacement::Eligible` (a usage bound, FB-1) replaces the reservation (`SettlementAmount`). An ineligible usage settlement, `Valid(ErrorStatus)` and the invalid decisions keep it (`ReservationBound`, R5-01, D4). `Valid(Reservation)` and `Valid(UpstreamError)` confirm it (`ReservationConfirmed`) |
| 3 | Retaining independent usage evidence | `Valuation` (§7) is computed from usage evidence whatever the settlement decision, and supplies the supported low bound wherever §8 says so |
| 4 | Classifying unknown additional exposure | `High::AtLeast` (§8), with a blocking `UnresolvedExposure` |
| 5 | Deciding whether admission is established | stage R (§12), outside S to X |

`Absent`, `Rejected` and `Conflicting` leave the obligation `Outstanding`.

### 6.2 Execution versus billing (R-07, owner disposition P1.30)

```text
Execution = Terminal | NotTerminal      // Terminal: a settlement, request, upstream_error, stream_error or response_model_refused row is in the attempt's evidence
Replacement = Eligible | Ineligible{reason}   // §2.1 (R5-01): decides whether the reservation may be replaced
Billing   = Established | Unresolved | NotApplicable   // Established iff the high is Exact (FB-1); NotApplicable for the Codex pool
```

Five facts are kept apart and never stand in for one another:

| Fact | Contract term |
|---|---|
| execution completion | `Execution::Terminal` |
| valid proxy settlement | `SettlementDecision::Valid` (and `Obligation::Closed`) |
| usage valuation | `Valuation` (§7); a usage bound when complete over established sets (FB-1) |
| verified provider billing | `AmountBasis::ProviderBilling`: no source exists; never produced in S1 |
| unresolved additional exposure | `High::AtLeast`, so `Billing::Unresolved` |

`Billing::Established` means established under the usage-valuation evidentiary standard (§2), not verified billing.
An attempt that is `Terminal` and `Closed` can still be `Unresolved` (an upstream error, an error status, a
reservation-basis settlement, incomplete usage). Rollup, predecessor and commitment rules read `Billing`, never
`Execution` or `Obligation`, to decide whether money is resolved.

**Error-status attempts (Q2).**
- The settlement's charge of 0 is the proxy's own accounting (`ProxyCharge` 0), never the provider's bill.
- The reserved amount R is kept as a known amount, and the additional billing is unknown: exposure
  `low 0, high AtLeast(R [ProxyReservation])`, diagnostic `NonBillingUnverified`.
- `Exact(0)` is permitted only for an error class with authoritative evidence of non-billing (D4). That class is empty.

### 6.1 Forwarding fact (Q6)

```text
Forwarding = Forwarded{evidence}        // a provider response exists: a request row with an upstream HTTP status, a response id or model,
                                        // or usage; a response_model_refused row; any stream_error row (every producer stream_error
                                        // writer runs after the upstream response arrived: pi_container.py:1295, :1513, :1546) (R-13)
           | Unknown                     // a reservation or admission with no response row; an upstream_error row (the request may or
                                        // may not have reached the provider)
NotForwarded                             // only for requests that are not attempts: a refused row with a preflight reason (C-9)
```

Forwarding is derived only from the attempt's own rows (its key rows and `auxiliary` rows), never from a `refused`
row.

Forwarding is recorded on every attempt. It is a fact, not a finding, and it decides nothing about money or identity:
- an identity refusal is `Forwarded` (the provider answered with another model);
- an unresolved identity contradiction still blocks, whatever the forwarding fact.

A request proven never to have reached the provider (a `refused` row with a preflight reason) has no exposure and no
identity finding; it is not an attempt.

## 7. Valuation (stage V)

### 7.1 Pricing primitives (leaf functions)

- **`openai_price(usage, model, tier_mult)`** = `(input − cached − write) × r_in + cached × r_cached + write × r_write
  + output × r_out`. The tier multiplier is applied to all rates. When `input > 272,000`, input-side rates × 2 and
  output × 1.5. All in exact nano per token.
- **`anthropic_price(usage, model, residual)`** = `input × r_in + cache_read × r_read + w5m × r_5m + w1h × r_1h
  + (cache_write − w5m − w1h) × residual(r_5m, r_1h) + output × r_out`, with `residual ∈ {min, max}`. The producer
  consistency price uses `max` (the producer's rule). The valuation range uses `min` for the low and `max` for the
  high (V-4, R-15). A null or absent `w5m`/`w1h` counts as 0, as in the producer.
- **`producer_price(settlement, pinned)`.** The proxy's own rule:
  - OpenAI: tier = the `observed_tier` multiplier if recognised, else the **highest** multiplier in the table; a null
    cache count counts as 0 (`pi_container.py:1001`).
  - Anthropic: defined only for a `standard`, null or absent tier (`service_tier in (None, "standard")`,
    `pi_container.py:1052-1054`), an integer status, integer counts, and `w5m + w1h ≤ cache_write`.
  - Otherwise undefined, which gives `UsageUnpriceable`.
- **Unknown model.** A model absent from the rate files has no rate. `claude-haiku-5-5` is absent: producer-unpriceable
  (N-08).

### 7.2 Valuation result

```text
Valuation = NotReported                                   // no usage on the attempt's rows
          | Valued{ identities: [Model], low: Bound, high: Bound, splits: [Split], complete: bool }
          | Unpriceable{ reasons: [UnpriceableReason], supported: Option<Bound /*low*/>, priced_high: Option<Bound> }
```

| Rule | Statement |
|---|---|
| V-1 Identities | the distinct canonical identities (MC-1 to MC-3) among the launch pinned model, request `model`, `response_model` and any joined `IdentityRefusal.served`. A malformed name or a `ModelUnresolved` pinned model is an unpriceable identity |
| V-2 Range | `low` = minimum and `high` = maximum of the price over every identity × every allowed split × residual rate × the tier set. The range is a usage bound only when the tier set and the identity set are established and the usage is complete (FB-1) |
| V-3 Tier set (OpenAI) | **recognised** `observed_tier` (a key of the rate table's `tier_multiplier`): that one multiplier; the tier set is established by the response. **Missing** (`null` or absent) **or unrecognised**: the tier set is **not established**. The rate table is not evidence that every servable tier is known (owner disposition R-03/Q5, P1.30), and the producer charging such a tier at the table's highest multiplier fixes its charge, not the set of tiers. Valuation `Unpriceable{TierUnestablished(Missing \| Unrecognised), supported: None, priced_high}`: the known valuation at the dearest table multiplier is kept as `priced_high`, no minimum is supported (an unknown tier could be cheaper), and §8 gives `AtLeast` |
| V-3 Tier set (Anthropic) | `standard`, `null` or absent gives standard, as the producer accepts (R-14). Any other string gives `Unpriceable{NonStandardTier}`. Any other non-string gives `Unpriceable{MalformedTier}` (repairs N-02) |
| V-4 Splits (OpenAI) (R5-04) | each of `cached_tokens` and `cache_write_tokens` is in one state. **Present integer ≥ 0** (including 0): that value. **Null**: unknown (the provider omitted it; the current producer writes `null`, `pi_container.py:1563-1564`, and prices it as 0 for its own charge only). **Absent key**: unknown (written by producers before `ab2a0f84`, which did not log `cache_write_tokens`; 59 real unbudgeted rows). It is **never assumed zero**. **Malformed** (present, not an integer ≥ 0): `Unpriceable{MalformedCount}`. Unknown counts are enumerated over the feasible set `{cached, write ≥ 0, cached + write ≤ input}`, with any known count fixed. The range is the minimum and maximum price over its vertices; all vertices are priceable, so the range is a usage bound when the usage is complete (FB-1). Known counts that violate `cached + write ≤ input` are `Unpriceable{MalformedCount}` |
| V-4 Splits (Anthropic) | `w5m + w1h ≤ cache_write`: the reported split, with the residual writes `cache_write − w5m − w1h` at min(r_5m, r_1h) for the low and max for the high (R-15). Otherwise all writes at min for the low and max for the high, with diagnostic `InconsistentCacheSplit` (repairs probe H-a); the counts contradict each other, so the usage is not complete (V-5) |
| V-5 Completeness | `complete = true` only for usage from a terminal success (`response.completed`, `message_stop`, status 200) whose counts are consistent (for Anthropic, `w5m + w1h ≤ cache_write`). Otherwise the usage may understate the bill and is not a usage bound (FB-1). Completeness is computed from the usage evidence, never from a diagnostic |
| V-6 Partial pricing | if some identity has no rate: `Unpriceable{IdentityWithoutRate, supported: None, priced_high: high over priced identities}`. No minimum is supported, because the unpriced identity could be cheaper, so §8's low is 0 |
| V-7 Excluded sources | usage not on a proxy row (e.g. a Pi session file's `cacheWrite`) is not evidence (A1-06) |
| V-8 Several observations (R-05, R5-04) | an attempt with more than one usage observation (conflicting settlements; a settlement plus a joined refusal) has `Valued{low = min of lows, high = max of highs, complete = all complete, identities = union}`. If any observation is `Unpriceable`, the result is `Unpriceable{reasons = union}` with **`supported = None`** (the unpriceable observation could be the cheapest) and **`priced_high` = max of every valued high and every priced high** (absent when none exists). Adding an observation never lowers `high` |

## 8. Exposure (stage X)

Pool: `PiApi` for API lanes and for any dollar evidence. `Codex` for subscription attempts on an established
subscription lane; these carry no dollars and billing is `Unknown`. A contradicted lane is treated as `PiApi`.

| Decision \ evidence | `low` | `high` |
|---|---|---|
| `Valid(Usage)`, `Valued`, `complete`, one identity | valuation low | `Exact(max(charged [ProxyCharge], valuation high [UsageValuation]))` (FB-1) |
| `Valid(Usage)`, `Valued`, `complete`, several identities | valuation low (cheapest identity) | `Exact(max(charged, dearest))`, plus blocking `IdentityContradiction` (D6) |
| `Valid(Usage)`, `Valued`, not `complete` (`Ineligible{UsageIncomplete}`) | valuation low | `AtLeast(max(R [ProxyReservation], charged [ProxyCharge], valuation high [UsageValuation]))`, blocking. The reservation is retained (R5-01) |
| `Valid(Usage)`, `Unpriceable` (`Ineligible{UsageUnpriceable}`: identity without a rate, tier set not established, malformed counts, …) | supported low or 0 | `AtLeast(max(R, charged, priced_high or 0))`, blocking. The reservation is retained (R5-01) |
| `Valid(Reservation)`, any valuation | valuation low if `Valued`, else supported low or 0 | `AtLeast(max(R [ProxyReservation], valuation high or priced_high or 0))` (FB-2). The producer keeps the reservation only when its own usage rule fails (invalid counts, a contradictory Anthropic split, a non-standard Anthropic tier, an incomplete exchange), so there is no usage bound whatever V-5 says (R-01) |
| `Valid(UpstreamError)` | 0 | `AtLeast(R)` (FB-2) |
| `Valid(ErrorStatus)` | valuation low if `Valued`, else supported low or 0 | `AtLeast(max(R, valuation high or priced_high or 0))` (FB-2, Q2, R5-02; diagnostic `NonBillingUnverified`). The proxy's charge of 0 is a proxy amount, not billing; reported usage keeps its provenance as a usage valuation |
| `Absent` (reservation, no settlement) | 0 | `AtLeast(R)` (FB-2) |
| `Rejected` or `Conflicting`, with a reservation | 0 | `AtLeast(max(R, every well-formed claimed charge, every usage observation's valuation high or priced_high))`, blocking (FB-2, R-05). A claimed amount that is not well-formed is ignored for the maximum; the rejection itself blocks |
| `NoReservation` (settlement only) | 0 | `AtLeast(max(well-formed charged, valuation high or priced_high))`, else `AtLeast(0)`; blocking |
| `NotApplicable`, unbudgeted, `Valued`, `complete`, one identity | valuation low | `Exact(valuation high)` |
| `NotApplicable`, unbudgeted, `Valued`, `complete`, several identities | valuation low (cheapest) | `Exact(dearest)`, blocking `IdentityContradiction` |
| `NotApplicable`, unbudgeted, not `complete`, or `Unpriceable` | supported low | `AtLeast(valuation or priced high)`, blocking |
| `NotApplicable`, unbudgeted non-200 status, no usage | 0 | `AtLeast(0)`, blocking `UnresolvedExposure{UnverifiedNonBilling}` (D4) |
| Orphan or ambiguous error attempt (no reservation of its own) | 0 | `AtLeast(0)`, blocking |
| Subscription attempt, lane established | — | pool `Codex`, billing `Unknown`; no dollar exposure; may carry a cost-proxy measurement |
| Subscription attempt, lane contradicted | 0 | `PiApi`, `AtLeast(0)`, blocking `LaneContradiction` |

**Rules:**
- **X-0. Notation.**
  - `R` is the **largest of the group's distinct well-formed reservation amounts** (R5-09). Normally there is one. When
    one `rid` names different reservations, their independence is not established, so they are neither summed nor
    reduced to the smaller: all stay visible, `R` is the largest, and `ReservationConflict` blocks. A malformed amount
    contributes 0 and adds `MalformedEvidence(reserved)` (R-06).
  - `max(a [basis_a], b [basis_b])` takes the basis of the larger. **On a tie the basis is `UsageValuation`, falling
    back to `ProxyReservation`, then `ProxyCharge`** (revision 5 corrects the order to match the KeepAll case, where the
    charge is the reservation). An `Exact` high may carry basis `ProxyCharge` only when a usage bound exists below it
    (FB-1, R-02).
- **X-1.** `low.amount ≤ high.amount` always. If a valuation low would exceed the chosen high (impossible under §7,
  stated for completeness), that is blocking `ArithmeticFailure`.
- **X-2. Unknown never becomes finite.** No sum, projection or rollup turns `AtLeast` into `Exact`, and a known partial
  amount never caps an unresolved one. The only `AtLeast` → `Exact` transition is replacement by an eligible usage
  settlement (FB-1, F3 exception, R5-01), when evidence is added to an attempt.
- **X-3. Findings never feed money.** Exposure is a function of (decision, obligation, valuation, lane, `R`) only. No
  finding is an input.

## 9. Classification (stage K)

`classify(facts) → (Vec<Uncertainty>, Vec<Diagnostic>)`.

**Blocking `Uncertainty`** (each carries evidence refs and the session scopes that could own them):

| Code | Raised when |
|---|---|
| `SettlementRejected(reason)` | decision `Rejected` |
| `SettlementConflict` | decision `Conflicting` |
| `ReservationConflict` | reservations with one `rid` differ |
| `SettlementWithoutReservation` | `NoReservation` |
| `IdentityContradiction` | ≥ 2 identities, or an unpaired refusal naming another model (D6) |
| `LaneContradiction` | a present label contradicts the session lane or provider; any `IdentityRefusal` in a subscription session; dollar evidence in a subscription session; subscription labels in an API session |
| `UnadmittedMeasurement` | a measurement with no earlier admission |
| `UnaccountedAttempt{cause}` | orphan or ambiguous error; unbudgeted request with no response and no charge |
| `UnresolvedExposure{cause}` | any `AtLeast` high (D5) |
| `UnreadableEvidence` | any `Unreadable` item in an owned source |
| `InventoryIncomplete{kind}` | turn gap, unrecorded turn file, unreferenced, misplaced or shared log, missing log |
| `DuplicateResponseIdentity` | one provider response id in two attempts |
| `AttributionContradiction` | an owned attempt is placed wholly outside its owner's window (an admission/time contradiction) |
| `ReopenedWithoutAdmission` | a session whose lifecycle has `reopened` (a deliver after a retire, D2), for the objectives that admitted it; persists after a later retirement (R-10) |
| `UnadmittedExecution` | a vertical-produced attempt that no admission covers (O-8); no current case, because no producer writes `attempt_id` yet |
| `CommitmentInvalid` | §12.3 A-1 or A-2 |
| `UnenforcedCommitment` | a commitment of kind `AtLeast` (§12.3): pending, launched-open, retired-resumable or reopened, without a qualified `ExecutionGate` (R-04, OQ-1) |
| `GateContradicted` | gate evidence claims `reopening` enforced but the session was reopened after `effective_from` (§2.3 G-7) |
| `ArithmeticFailure` | — |
| `RateTableUnusable` | — |

`Unreadable` and inventory findings are scoped to every session that could own the source: the path's session id,
the ledger's naming session, or both (repairs S-05; V-07 is scoped this way instead of globally).

**Additional blocking code:** `MalformedEvidence(field)`: a malformed field on evidence that no settlement rejection
already covers (e.g. a reservation's `n`, an unbudgeted row's non-object `usage`).

**Non-blocking `Diagnostic`:** `RepeatedIdenticalRow`, `InconsistentCacheSplit`, `NonBillingUnverified`,
`IdentityAbsent`, `ProducerLogCopy`, `ErrorStatusSettlement`, `ReadFailurePaired`, `PartialUsage`,
`UnfinishedAdmission` (an admitted subscription request with no observation: an unknown Codex call),
`UncertainCodexCall` (a measured, producer-labelled subscription error).

**Rule K-1.** Deleting any diagnostic from an output changes no other field. Classification is downstream only.

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
| A-2 | One `pending` per admission. `released` with `launched` or `runner_starting` is `Invalid`. Pending without `launched` or `released` is committed in full (`committed = ceiling`: the cap, or a qualified G-8 bound, R5-03). A valid `released` row releases it: no session was launched, so nothing can resume (committed 0, `Exact`) |
| A-3 (OQ-1) | **Launched.** `committed = max(0, ceiling − the session's owned high)`, where `ceiling` is the admission cap, or the qualified G-8 `amount_nano` (never above the cap) when a qualified billing-bound gate exists (R5-03). The state follows the lifecycle: `launched_open` (not retired, not reopened), `reopened` (reopened, not retired), or, when the lifecycle state is `Retired`: `retired` if a qualified gate establishes release (G-5), otherwise **`retired_resumable`**, which keeps the commitment (L-7). Only `retired` (committed 0, `Exact`) releases. Owned spend above the cap is `NotEstablished{CommitmentInvalid}` |
| A-4 | A session with `reopened = Some`: whatever the state, the objective is `NotEstablished{ReopenedWithoutAdmission}` until a qualified admission covers the reopening (D2). A reopening never releases a commitment and never establishes safety. If a gate claims `reopening` enforced, the reopening contradicts it (G-7) |
| A-5 (R-04, Q10, OQ-1, OQ-3, R5-03) | **Kind.** `pending`, `launched_open`, `retired_resumable` and `reopened` commitments are `AtLeast(committed)` with blocking `UnenforcedCommitment`, unless a qualified gate with an enforced, numerically bound `billing_bound` (G-5, G-8) makes a `pending` or `launched_open` commitment `Exact(committed)`. `retired`, `released` and `invalid` are `Exact(0)`. Every commitment records `gate.qualified` and the failed rules (§2.3). For the current producer: `qualified = false`, reason `NoGateEvidence` |

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

## 13. Canonical output and determinism

- Outputs are canonical JSON: sorted keys, amounts as decimal strings of nano-USD integers, lists ordered by
  (source, line) or by `AttemptKey` order.
- Equal inputs give byte-identical output.
- Session and source presentation order MUST NOT change any output beyond declared append-order rules within a source.

## 14. Transition specification

`TRANSITIONS.md` gives the factorised complete tables (per stage, plus composition), and `INVARIANTS.md` the
executable invariants.
