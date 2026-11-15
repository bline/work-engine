# Generated C–K normative view

Source: spec/ck.json; contract pi-accounting-contract/1.5-s1; input SHA256 67225cbd03437a2dfd077497f9fc6137f76c4103a4f37ac05e19547c92ec011a.

This rendering checks declared consistency only. It is not a reducer, financial proof, live trust qualification, freeze or S2a entry.

## Stage interfaces

| Stage | Reads | Outputs |
|---|---|---|
| E |  | typed_evidence |
| C | E | groups, raw_identity, lane, reservations |
| S | C | decision, temporal_obligation, execution, forwarding |
| V | C | valuation |
| F | C, S, V | replacement, obligation |
| X | C, S, V, F | exposure, billing |
| K | C, S, V, F, X | blocking, diagnostics |

## Fixed tagged types

Tag/field encoding below describes typed specification constructors. The corpus projection is declared in the serialization interface.

### Field r1

```json
{
  "id": "Field",
  "revision": 1,
  "variants": [
    {
      "name": "present",
      "fields": {
        "value": "Json"
      }
    },
    {
      "name": "null",
      "fields": {}
    },
    {
      "name": "absent",
      "fields": {}
    },
    {
      "name": "malformed",
      "fields": {
        "json_type": "String",
        "raw": "Json"
      }
    }
  ]
}
```

### Model r1

```json
{
  "id": "Model",
  "revision": 1,
  "variants": [
    {
      "name": "model",
      "fields": {
        "family": "ProviderFamily",
        "model_id": "String"
      }
    }
  ]
}
```

### Bound r1

```json
{
  "id": "Bound",
  "revision": 1,
  "variants": [
    {
      "name": "bound",
      "fields": {
        "basis": "String",
        "evidence": "List<String>",
        "nano": "Nano"
      }
    }
  ]
}
```

### High r1

```json
{
  "id": "High",
  "revision": 1,
  "variants": [
    {
      "name": "exact",
      "fields": {
        "bound": "Bound"
      }
    },
    {
      "name": "at_least",
      "fields": {
        "bound": "Bound"
      }
    }
  ]
}
```

### Replacement r1

```json
{
  "id": "Replacement",
  "revision": 1,
  "variants": [
    {
      "name": "eligible",
      "fields": {
        "reason": "String"
      }
    },
    {
      "name": "ineligible",
      "fields": {
        "reason": "String"
      }
    }
  ]
}
```

### SettlementDecision r1

```json
{
  "id": "SettlementDecision",
  "revision": 1,
  "variants": [
    {
      "name": "absent",
      "fields": {}
    },
    {
      "name": "valid",
      "fields": {
        "basis": "String",
        "charged": "Nano",
        "settlement": "String"
      }
    },
    {
      "name": "rejected",
      "fields": {
        "claimed": "List<Nano>",
        "reasons": "List<RejectReason>"
      }
    },
    {
      "name": "conflicting",
      "fields": {
        "claimed": "List<Nano>",
        "settlements": "List<String>"
      }
    },
    {
      "name": "not_applicable",
      "fields": {}
    }
  ]
}
```

### TemporalObligation r1

```json
{
  "id": "TemporalObligation",
  "revision": 1,
  "variants": [
    {
      "name": "outstanding",
      "fields": {}
    },
    {
      "name": "closed",
      "fields": {
        "by": "String"
      }
    },
    {
      "name": "none",
      "fields": {
        "cause": "String"
      }
    }
  ]
}
```

### Obligation r1

```json
{
  "id": "Obligation",
  "revision": 1,
  "variants": [
    {
      "name": "outstanding",
      "fields": {
        "amount_source": "AmountSource"
      }
    },
    {
      "name": "closed",
      "fields": {
        "amount_source": "AmountSource",
        "by": "String"
      }
    },
    {
      "name": "none",
      "fields": {
        "amount_source": "AmountSource",
        "cause": "NoneCause"
      }
    }
  ]
}
```

### Valuation r1

```json
{
  "id": "Valuation",
  "revision": 1,
  "variants": [
    {
      "name": "not_reported",
      "fields": {}
    },
    {
      "name": "valued",
      "fields": {
        "complete": "Bool",
        "high": "Bound",
        "identities": "List<String>",
        "low": "Bound",
        "splits": "List<Json>"
      }
    },
    {
      "name": "unpriceable",
      "fields": {
        "priced_high": "Option<Bound>",
        "reasons": "List<UnpriceableReason>",
        "supported_low": "Option<Bound>"
      }
    }
  ]
}
```

### Exposure r1

```json
{
  "id": "Exposure",
  "revision": 1,
  "variants": [
    {
      "name": "pi_api",
      "fields": {
        "high": "High",
        "low": "Bound"
      }
    },
    {
      "name": "codex",
      "fields": {
        "billing": "String"
      }
    }
  ]
}
```

### Correlation r1

```json
{
  "id": "Correlation",
  "revision": 1,
  "variants": [
    {
      "name": "exact",
      "fields": {
        "auxiliary": "List<Json>"
      }
    },
    {
      "name": "ambiguous",
      "fields": {
        "auxiliary": "List<Json>",
        "candidates": "List<String>"
      }
    },
    {
      "name": "unattributed",
      "fields": {
        "auxiliary": "List<Json>"
      }
    }
  ]
}
```

### GateVerification r1

```json
{
  "id": "GateVerification",
  "revision": 1,
  "variants": [
    {
      "name": "unverified",
      "fields": {}
    },
    {
      "name": "rejected",
      "fields": {
        "reasons": "List<String>"
      }
    },
    {
      "name": "verified",
      "fields": {
        "authority_id": "String",
        "payload_digest": "String",
        "provenance": "String",
        "trust_policy_ref": "String",
        "verification_evidence": "String",
        "verifier_identity": "String",
        "verifier_version": "String"
      }
    }
  ]
}
```

### ProviderFamily r1

```json
{
  "id": "ProviderFamily",
  "revision": 1,
  "variants": [
    {
      "name": "openai",
      "fields": {}
    },
    {
      "name": "anthropic",
      "fields": {}
    }
  ]
}
```

### AmountSource r1

```json
{
  "id": "AmountSource",
  "revision": 1,
  "variants": [
    {
      "name": "settlement_amount",
      "fields": {}
    },
    {
      "name": "reservation_confirmed",
      "fields": {}
    },
    {
      "name": "reservation_bound",
      "fields": {}
    },
    {
      "name": "none",
      "fields": {}
    }
  ]
}
```

### NoneCause r1

```json
{
  "id": "NoneCause",
  "revision": 1,
  "variants": [
    {
      "name": "unbudgeted",
      "fields": {}
    },
    {
      "name": "no_reservation",
      "fields": {}
    },
    {
      "name": "orphan",
      "fields": {}
    },
    {
      "name": "subscription",
      "fields": {}
    }
  ]
}
```

### Basis r1

```json
{
  "id": "Basis",
  "revision": 1,
  "variants": [
    {
      "name": "usage",
      "fields": {}
    },
    {
      "name": "reservation",
      "fields": {}
    },
    {
      "name": "upstream_error",
      "fields": {}
    },
    {
      "name": "error_status",
      "fields": {}
    }
  ]
}
```

### RejectReason r1

```json
{
  "id": "RejectReason",
  "revision": 1,
  "variants": [
    {
      "name": "ReservationConflict",
      "fields": {}
    },
    {
      "name": "NoReservation",
      "fields": {}
    },
    {
      "name": "Order",
      "fields": {}
    },
    {
      "name": "AttemptNumber",
      "fields": {}
    },
    {
      "name": "AmountType",
      "fields": {}
    },
    {
      "name": "ReservedMismatch",
      "fields": {}
    },
    {
      "name": "BasisMissing",
      "fields": {}
    },
    {
      "name": "BasisUnknown",
      "fields": {}
    },
    {
      "name": "KeepAll",
      "fields": {}
    },
    {
      "name": "StatusType",
      "fields": {}
    },
    {
      "name": "ErrorCharge",
      "fields": {}
    },
    {
      "name": "UsageUnpriceable",
      "fields": {}
    },
    {
      "name": "ModelMismatch",
      "fields": {}
    },
    {
      "name": "ToleranceRange",
      "fields": {}
    },
    {
      "name": "Undercharge",
      "fields": {}
    }
  ]
}
```

### UnpriceableReason r1

```json
{
  "id": "UnpriceableReason",
  "revision": 1,
  "variants": [
    {
      "name": "IdentityWithoutRate",
      "fields": {}
    },
    {
      "name": "ModelUnresolved",
      "fields": {}
    },
    {
      "name": "TierUnestablished(Missing)",
      "fields": {}
    },
    {
      "name": "TierUnestablished(Unrecognised)",
      "fields": {}
    },
    {
      "name": "NonStandardTier",
      "fields": {}
    },
    {
      "name": "MalformedTier",
      "fields": {}
    },
    {
      "name": "MalformedCount",
      "fields": {
        "field": "String"
      }
    },
    {
      "name": "MissingCount",
      "fields": {
        "field": "String"
      }
    },
    {
      "name": "UsageSchemaMismatch",
      "fields": {}
    },
    {
      "name": "UsageShape",
      "fields": {}
    },
    {
      "name": "UnknownUsageSchema",
      "fields": {}
    },
    {
      "name": "RateTableUnusable",
      "fields": {}
    }
  ]
}
```

### UsagePresence r1

```json
{
  "id": "UsagePresence",
  "revision": 1,
  "variants": [
    {
      "name": "not_reported",
      "fields": {}
    },
    {
      "name": "reported",
      "fields": {}
    },
    {
      "name": "unpriceable",
      "fields": {}
    }
  ]
}
```

### FieldCount r1

```json
{
  "id": "FieldCount",
  "revision": 1,
  "variants": [
    {
      "name": "present",
      "fields": {
        "value": "U64"
      }
    },
    {
      "name": "null",
      "fields": {}
    },
    {
      "name": "absent",
      "fields": {}
    },
    {
      "name": "malformed",
      "fields": {
        "json_type": "String",
        "raw": "Json"
      }
    }
  ]
}
```

### Usage r1

```json
{
  "id": "Usage",
  "revision": 1,
  "variants": [
    {
      "name": "openai",
      "fields": {
        "cache_write_tokens": "FieldCount",
        "cached_tokens": "FieldCount",
        "input_tokens": "FieldCount",
        "output_tokens": "FieldCount",
        "reasoning_tokens": "FieldCount"
      }
    },
    {
      "name": "anthropic",
      "fields": {
        "cache_creation_input_tokens": "FieldCount",
        "cache_read_input_tokens": "FieldCount",
        "ephemeral_1h_input_tokens": "FieldCount",
        "ephemeral_5m_input_tokens": "FieldCount",
        "input_tokens": "FieldCount",
        "output_tokens": "FieldCount"
      }
    }
  ]
}
```

### PRICE-OPENAI r1

((input-cached-write)*r_in + cached*r_cached + write*r_write + output*r_out)*tier; input>272000: input-side rates*2 and output rate*1.5; checked integer nano arithmetic.

Inputs: input, cached, write, output, r_in, r_cached, r_write, r_out, tier.

### PRICE-ANTHROPIC r1

input*r_in + cache_read*r_read + w5m*r_5m + w1h*r_1h + (cache_write-w5m-w1h)*residual(r_5m,r_1h) + output*r_out; residual=min for low, max for high and producer consistency.

Inputs: input, cache_read, w5m, w1h, cache_write, output, r_in, r_read, r_5m, r_1h, r_out.

### RESERVATION-MAX r1

R = max({distinct well-formed reservation amounts} union {0}); never sum/min; malformed amounts contribute 0 and blocking MalformedEvidence(reserved).

Inputs: C.reservations.

### ERROR-EXPOSURE r1

low = valued.low or supported_low or 0; high = AtLeast(max(R, valued.high or priced_high or 0)); valid ErrorStatus closes time and retains ReservationBound.

Inputs: C.reservations, S.decision, V.valuation.

### ELIGIBLE-EXPOSURE r1

low = valuation.low; high = Exact(max(charged,valuation.high)); finite only with complete usage over established fully priced identity and tier sets.

Inputs: S.decision, V.valuation, F.replacement.

### INELIGIBLE-EXPOSURE r1

low and known components follow X-TABLE; high = AtLeast(max(R, applicable well-formed claimed charges, usage high or priced_high or 0)); no financial decision reads K.

Inputs: C.reservations, S.decision, V.valuation, F.replacement.

### TOLERANCE r1

T_round=501 nano; producer_price<10^15 and charged>=producer_price-501; otherwise ToleranceRange or Undercharge.

Inputs: producer_price, charged.

### BASIS-TIE r1

max takes basis of largest; ties prefer UsageValuation, then ProxyReservation, then ProxyCharge. Nonzero supported low has usage basis.

Inputs: components.

### COMMITMENT-RESIDUAL r1

max(0,B-known session owned high) is separate allocation arithmetic conditional on qualified trusted whole-session USD bound; never changes C-K amounts; AtLeast owned aggregate remains unresolved even with finite residual; P>B alone is not contradictory bill evidence.

Inputs: B, owned_high.

### ENCODING r1

## 13. Canonical output and determinism

- Outputs are canonical JSON: sorted keys, amounts as decimal strings of nano-USD integers, lists ordered by
  (source, line) or by `AttemptKey` order.
- Equal inputs give byte-identical output.
- Session and source presentation order MUST NOT change any output beyond declared append-order rules within a source.

### MONEY r1

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

Typed constructors are owned by types High, Bound, Exposure and Replacement.


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

**Monetary measures (A-01, A-02, owner disposition P1.33).** Six facts are kept apart for every attempt. None stands in
for another:

| Fact | Where it is recorded | Measure |
|---|---|---|
| terminal execution | `Execution` (§6.2) | none (a time fact) |
| structurally valid settlement | `SettlementDecision::Valid` (§5) | none (a validity fact) |
| complete and priceable usage | `Valuation` with `complete` and established sets (§7, FB-1) | **usage valuation**: what the reported usage costs at published rates |
| eligibility to replace the reservation | `Replacement` (below) | none (a decision fact) |
| proxy charge and proxy reservation | `settlement.charged`, `R` | **proxy amounts**: what the proxy kept and set aside |
| independently established provider billing | `AmountBasis::ProviderBilling` | never produced in S1 |

**Reservation-backed exposure is one measure: conservative provisional exposure**, the amount the admission accounting must hold for the reservation-backed attempt. Unbudgeted/orphan cases are owned by X-TABLE. Its `low` is supported usage valuation, never a proxy amount. Its `high` is either:
- `Exact(…)`, when the reservation is **replaced** by an eligible usage settlement (FB-1); or
- `AtLeast(known)`, where `known` is the largest of the known components the conservative policy must hold: the
  retained reservation `R`, the proxy charge, and the usage valuation's high or `priced_high`. Each component is a
  lower bound of what the policy holds, with its own basis (X-0). The maximum is taken within this one measure, not
  across measures: provider billing is never claimed, and the usage valuation keeps its own record in `Valuation`.

Typed constructors are owned by types High, Bound, Exposure and Replacement.


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

### EVIDENCE r1

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

**Model canonicalisation (A-05; rule version `mc-1`).** Pricing identity is never established by fuzzy matching.
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

### CORRELATION r1

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
| C-1 | A `Reservation` with a usable `rid` keys `Reservation{source, rid}`. Byte-identical repeats are one item (diagnostic `RepeatedIdenticalRow`). **Different rows with one `rid` are conflicting reservation evidence (A-09):** all stay visible in the group, blocking `ReservationConflict` is raised, and the decision follows Table S1 (no settlement: `Absent`; one settlement: `Rejected[ReservationConflict…]`; two or more: `Conflicting`). Their independence is not established, so `R` is the largest of them (X-0), never their sum and never the smaller. **A reservation whose `rid` is absent or malformed** keys `Row{source, line}` with `key_join = Unattributed`. Its decision is `Absent` (no settlement can join it) and its obligation `Outstanding`, with blocking `MalformedEvidence(rid)` and `UnaccountedAttempt`. A malformed `reserved` contributes 0 to R and adds `MalformedEvidence(reserved)` (R-06) |
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

### SETTLEMENT r1

## 5. Settlement decision (stage S)

Typed constructor: SettlementDecision in types.


**`Valid` iff every condition holds, otherwise `Rejected` with every failing reason** (codes in brackets):
1. Exactly one reservation and exactly one distinct settlement in the group [`ReservationConflict`, `NoReservation`].
2. With UniqueReservation, the settlement is appended after the reservation [`Order`].
3. `n` is a positive integer on both (never boolean, string, float or zero) and, with UniqueReservation, equal [`AttemptNumber`].
4. `charged` and `reserved` are well-formed amounts [`AmountType`], and, with UniqueReservation, `reserved` equals its amount exactly [`ReservedMismatch`].
5. `basis` is present and known [`BasisMissing`, `BasisUnknown`]. An `UpstreamError` requires `Reservation`.
6. `Reservation` or `UpstreamError`: with UniqueReservation, charged equals the reservation exactly [`KeepAll`].
7. **Status, by settlement path (A-06, owner disposition P1.33).** A missing status is neither an automatic failure nor
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
   - the full canonical family-qualified request identity equals the pinned identity (MC-1 to MC-3) [`ModelMismatch`];
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


UniqueReservation is an exactly-one distinct reservation witness. Zero/conflicting reservations make reservation-relative Order, equality and KeepAll comparisons inapplicable, without suppressing applicable local amount, n, status, basis and producer checks. Rejected carries all applicable failures; multiple distinct settlements take Conflicting precedence.

### TEMPORAL-AND-FINALIZATION r1

## 6. Obligation: releasing versus replacing (stage S)

TemporalTemporalObligation = Outstanding | Closed{by} | None{cause}, owned by S. External TemporalObligation = Outstanding{amount_source} | Closed{by,amount_source} | None{cause,amount_source:none}, projected by F. Every variant is representable and carries amount_source.


`Obligation` is an **execution-time** fact: whether a reservation is still open in time. It is not a financial
disposition (§6.2).

**`amount_source` (A-08).** It records where the conservative measure's amount comes from. It is constructed by stage F from the decision, temporal obligation and valuation-derived replacement fact:

| Variant | Constructed when | Meaning |
|---|---|---|
| `SettlementAmount` | `Valid(Usage)` with `Replacement::Eligible` | the reservation is replaced by the settlement and usage valuation |
| `ReservationConfirmed` | `Valid(Reservation)` or `Valid(UpstreamError)` (KeepAll: charged = R) | the proxy kept the whole reservation as its charge; the reservation stays as a known amount, `AtLeast` |
| `ReservationBound` | `Valid(ErrorStatus)`; `Valid(Usage)` with `Replacement::Ineligible`; `Absent`, `Rejected`, `Conflicting` with a reservation | the reservation is retained as provisional exposure without being the proxy's charge |

`amount_source` is `none` for `Obligation::None` (no reservation exists). Its JSON spellings are
`settlement_amount`, `reservation_confirmed` and `reservation_bound`.

The five distinctions the owner asked for:

| # | Concept | Contract term |
|---|---|---|
| 1 | Releasing a reservation | `Obligation::Closed`: the attempt has ended in time (its interval ends at the settlement). Only `Valid` closes. Closing never establishes billing (§6.2, R-07) |
| 2 | Replacing provisional exposure with a validated settlement | only `Valid(Usage)` with `Replacement::Eligible` (a usage bound, FB-1) replaces the reservation (`SettlementAmount`). An ineligible usage settlement, `Valid(ErrorStatus)` and the invalid decisions keep it (`ReservationBound`, A-01, D4). `Valid(Reservation)` and `Valid(UpstreamError)` confirm it (`ReservationConfirmed`) |
| 3 | Retaining independent usage evidence | `Valuation` (§7) is computed from usage evidence whatever the settlement decision, and supplies the supported low bound wherever §8 says so |
| 4 | Classifying unknown additional exposure | `High::AtLeast` (§8), with a blocking `UnresolvedExposure` |
| 5 | Deciding whether admission is established | stage R (§12), outside S to X |

With a reservation, `Absent`, `Rejected` and `Conflicting` leave the obligation `Outstanding`; without one the temporal result is `None` with its explicit cause.

### 6.2 Execution versus billing (R-07, owner disposition P1.30)

```text
Execution = Terminal | NotTerminal      // Terminal: a settlement, request, upstream_error, stream_error or response_model_refused row is in the attempt's evidence
Replacement = Eligible | Ineligible{reason}   // §2.1 (A-01): decides whether the reservation may be replaced
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
  the formula ERROR-EXPOSURE, diagnostic `NonBillingUnverified`.
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

### VALUATION r1

## 7. Valuation (stage V)

### 7.1 Pricing primitives (leaf functions)

Pricing formulas: PRICE-OPENAI and PRICE-ANTHROPIC; canonical definitions are not restated here.
- **`producer_price(settlement, pinned)`.** The proxy's own rule:
  - OpenAI: tier = the `observed_tier` multiplier if recognised, else the **highest** multiplier in the table; a null
    cache count counts as 0 (`pi_container.py:1001`).
  - Anthropic: defined only for a `standard`, null or absent tier (`service_tier in (None, "standard")`,
    `pi_container.py:1052-1054`), an integer status, integer counts, and `w5m + w1h ≤ cache_write`.
  - Otherwise undefined, which gives `UsageUnpriceable`.
- **Unknown model.** A model absent from the rate files has no rate. `claude-haiku-5-5` is absent: producer-unpriceable
  (N-08).

### 7.2 Valuation result

Typed constructors: Valuation in types.


| Rule | Statement |
|---|---|
| V-1 Identities | the distinct canonical identities (MC-1 to MC-3) among the launch pinned model, request `model`, `response_model` and any joined `IdentityRefusal.served`. A malformed name or a `ModelUnresolved` pinned model is an unpriceable identity |
| V-2 Range | `low` = minimum and `high` = maximum of the price over every identity × every allowed split × residual rate × the tier set. The range is a usage bound only when the tier set and the identity set are established and the usage is complete (FB-1) |
| V-3 Tier set (OpenAI) | **recognised** `observed_tier` (a key of the rate table's `tier_multiplier`): that one multiplier; the tier set is established by the response. **Missing** (`null` or absent) **or unrecognised**: the tier set is **not established**. The rate table is not evidence that every servable tier is known (owner disposition R-03/Q5, P1.30), and the producer charging such a tier at the table's highest multiplier fixes its charge, not the set of tiers. Valuation `Unpriceable{TierUnestablished(Missing \| Unrecognised), supported: None, priced_high}`: the known valuation at the dearest table multiplier is kept as `priced_high`, no minimum is supported (an unknown tier could be cheaper), and §8 gives `AtLeast` |
| V-3 Tier set (Anthropic) | `standard`, `null` or absent gives standard, as the producer accepts (R-14). Any other string gives `Unpriceable{NonStandardTier}`. Any other non-string gives `Unpriceable{MalformedTier}` (repairs N-02) |
| V-4 Splits (OpenAI) (A-04) | each of `cached_tokens` and `cache_write_tokens` is in one state. **Present integer ≥ 0** (including 0): that value. **Null**: unknown (the provider omitted it; the current producer writes `null`, `pi_container.py:1563-1564`, and prices it as 0 for its own charge only). **Absent key**: unknown (written by producers before `ab2a0f84`, which did not log `cache_write_tokens`; 59 real unbudgeted rows). It is **never assumed zero**. **Malformed** (present, not an integer ≥ 0): `Unpriceable{MalformedCount}`. Unknown counts are enumerated over the feasible set `{cached, write ≥ 0, cached + write ≤ input}`, with any known count fixed. The range is the minimum and maximum price over its vertices; all vertices are priceable, so the range is a usage bound when the usage is complete (FB-1). Known counts that violate `cached + write ≤ input` are `Unpriceable{MalformedCount}` |
| V-4 Splits (Anthropic) | `w5m + w1h ≤ cache_write`: the reported split, with the residual writes `cache_write − w5m − w1h` at min(r_5m, r_1h) for the low and max for the high (R-15). Otherwise all writes at min for the low and max for the high, with diagnostic `InconsistentCacheSplit` (repairs probe H-a); the counts contradict each other, so the usage is not complete (V-5) |
| V-5 Completeness | `complete = true` only for usage from a terminal success (OpenAI `response.completed` or `response.done`, Anthropic `message_stop`, integer status 200) whose counts are consistent (for Anthropic, `w5m + w1h ≤ cache_write`). Otherwise the usage may understate the bill and is not a usage bound (FB-1). Completeness is computed from the usage evidence, never from a diagnostic |
| V-6 Partial pricing | if some identity has no rate: `Unpriceable{IdentityWithoutRate, supported: None, priced_high: high over priced identities}`. No minimum is supported, because the unpriced identity could be cheaper, so §8's low is 0 |
| V-7 Excluded sources | usage not on a proxy row (e.g. a Pi session file's `cacheWrite`) is not evidence (A1-06) |
| V-8 Several observations (R-05, A-04) | an attempt with more than one usage observation (conflicting settlements; a settlement plus a joined refusal) has `Valued{low = min of lows, high = max of highs, complete = all complete, identities = union}`. If any observation is `Unpriceable`, the result is `Unpriceable{reasons = union}` with **`supported = None`** (the unpriceable observation could be the cheapest) and **`priced_high` = max of every valued high and every priced high** (absent when none exists). Adding an observation never lowers `high` |

### X-TABLE r1

## 8. Exposure (stage X)

Pool: `PiApi` for API lanes and for any dollar evidence. `Codex` for subscription attempts on an established
subscription lane; these carry no dollars and billing is `Unknown`. A contradicted lane is treated as `PiApi`.

| Decision \ evidence | `low` | `high` |
|---|---|---|
| `Valid(Usage)`, `Valued`, `complete`, one identity | valuation low | `Exact(max(charged [ProxyCharge], valuation high [UsageValuation]))` (FB-1) |
| `Valid(Usage)`, `Valued`, `complete`, several identities | valuation low (cheapest identity) | `Exact(max(charged, dearest))`, plus blocking `IdentityContradiction` (D6) |
| `Valid(Usage)`, `Valued`, not `complete` (`Ineligible{UsageIncomplete}`) | valuation low | `AtLeast(max(R [ProxyReservation], charged [ProxyCharge], valuation high [UsageValuation]))`, blocking. The reservation is retained (A-01) |
| `Valid(Usage)`, `Unpriceable` (`Ineligible{UsageUnpriceable}`: identity without a rate, tier set not established, malformed counts, …) | supported low or 0 | `AtLeast(max(R, charged, priced_high or 0))`, blocking. The reservation is retained (A-01) |
| `Valid(Reservation)`, any valuation | valuation low if `Valued`, else supported low or 0 | `AtLeast(max(R [ProxyReservation], valuation high or priced_high or 0))` (FB-2). The producer keeps the reservation only when its own usage rule fails (invalid counts, a contradictory Anthropic split, a non-standard Anthropic tier, an incomplete exchange), so there is no usage bound whatever V-5 says (R-01) |
| `Valid(UpstreamError)` | 0 | `AtLeast(R)` (FB-2) |
| `Valid(ErrorStatus)` | valuation low if `Valued`, else supported low or 0 | `AtLeast(max(R, valuation high or priced_high or 0))` (FB-2, Q2, A-02; diagnostic `NonBillingUnverified`). The proxy's charge of 0 is a proxy amount, not billing; reported usage keeps its provenance as a usage valuation |
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
  - `R` is the **largest of the group's distinct well-formed reservation amounts** (A-09). Normally there is one. When
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
  settlement (FB-1, F3 exception, A-01), when evidence is added to an attempt.
- **X-3. Findings never feed money.** Exposure is a function of (decision, obligation, valuation, lane, `R`) only. No
  finding is an input.

### CLASSIFICATION r1

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

### USAGE-PRESENCE r1

Pinned producer 92a03789 pi_container.py writes normalized usage fields. OpenAI billing-bearing: input_tokens, output_tokens, cached_tokens, cache_write_tokens; required when reported: input_tokens/output_tokens. reasoning_tokens is auxiliary. Anthropic billing-bearing: input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens, ephemeral_5m_input_tokens, ephemeral_1h_input_tokens; required: input_tokens/output_tokens/cache_read_input_tokens/cache_creation_input_tokens. Writer mapping verified against 92a03789 pi_container.py ANTHROPIC_USAGE_KEYS and request writer lines1318-1320, budget consumer1048-1054; OpenAI writer1562-1566. Service tier is metadata, not a count; server_tool_use is auxiliary metadata. Schema comes from explicit evidenced family, never fixture shape. Absent provider follows MC-2 OpenAI default, not shape inference. Non-null fields exclusive to the other declared family imply UsageSchemaMismatch. Shared fields and absent/null fields do not.

Check schema conflict, nonobject usage (UsageShape), unknown schema (UnknownUsageSchema), and malformed billing-bearing counts (MalformedCount(field)) before all-null classification. Absent/null usage and all-absent/null billing counts give NotReported; auxiliary alone does not establish reporting. Numeric zero is present. Reported usage missing required counts gives MissingCount(field), never zero. OpenAI optional cache absent/null means unknown feasible splits, Anthropic optional lifetime absent/null means zero. Malformed auxiliary adds MalformedEvidence(field) blocking but preserves independently computable primary usage and cannot select money. Raw states survive.

Completeness requires all required well-formed billing counts, established priced identity/tier sets, integer HTTP200 and provider-recognized successful terminal (OpenAI response.completed/response.done, Anthropic message_stop), with consistent counts. Partial/missing/incomplete terminal or non200 can preserve supported valuation but cannot replace reservation. V-8 partial pricing remains independent. Full family/model inequality fails ModelMismatch. Independently unpriceable/unestablished identity/schema/tier sets cannot replace; established complete fully priced multiple identities retain finite bounds while downstream IdentityContradiction blocks.

### INV-F1 r1

| **F1 Release** | `Obligation::Closed` ⇒ `SettlementDecision::Valid` with an `Exact` **key join** to the attempt's reservation (auxiliary RS-rule rows do not count, R-06). A numeric amount alone never releases | a reservation exists | VC01 (usage settlement closes); N01 (closed with `key_join` Exact and an RS-3 auxiliary row) | S01 (charge 0 against 0.8 under `reservation`) | rejected: `Outstanding`, high `AtLeast(R)`, blocking | 20 cases incl. S01×3, S02×2, U01, R02, C4, S06, V01, V02, U05, N02, R06 | for random settlements failing any §5 condition, `obligation = Outstanding` | drop one §5 condition (e.g. skip the `KeepAll` check) |

### INV-F2 r1

| **F2 No double count** | Per attempt, exposure counts the reservation or the settlement-derived amount, never both. Totals are sums over distinct `AttemptKey`s; byte-identical rows are one item | — | VC01 (high 0.015, not 0.515) | summing the reservation and the settlement charge | Σ exposure = Σ per-attempt | VC01, VC06, VC12, R04, C3, DUP | Σ over attempts = rollup; adding an identical copy changes nothing | add the settlement charge on top of the reservation |

### INV-F3 r1

| **F3 Monotone high** | Adding evidence never lowers `high` (order of §2) in an attempt or any total, **except** a valid usage settlement with `Replacement::Eligible` (a usage bound), which may replace `AtLeast(R)` with the validated `Exact` amount. A valid but ineligible usage settlement, an error-status settlement, and conflicting or rejected settlements keep `R` and only add terms to the maximum (A-01, A-02, R-05) | evidence added to an existing set | S04 (refused row added: high stays `AtLeast(R)`); U08 (incomplete usage: high stays `AtLeast(500M)`) | a refused row closing the interval (deac8099 S-04); incomplete usage lowering a 500M reservation to 30M (P1.32 A-01) | `high(E ∪ e) ≥ high(E)` unless the added row is an eligible usage settlement | 24+ cases incl. N01, VC03, S01, U08, Q5, P2, P130-R01, P133-A01, P133-A02 | random base plus random row other than an eligible usage settlement: high non-decreasing | let incomplete usage replace the reservation |

### INV-F4 r1

| **F4 Finding inertness** | No monetary field depends on the presence, absence or order of any finding. Removing all findings from an output leaves every amount unchanged, and deleting an input row whose only effect is a diagnostic changes no amount | — | Ha (inconsistent split: low 0.018 kept, diagnostic only) | A1.3.3 N-01 (diagnostic deletes the low bound and lowers high); A1.3.4 H-a (low → 0) | amounts equal with and without the diagnostic | N01×3, Ha, V01-other-model, DUP | compute money, strip findings, recompute: equal (structural in Rust: classification is downstream) | choose the representation by `findings.is_empty()` (the A1.3.2 guard) |

### INV-F5 r1

| **F5 Unknown stays unknown** | `AtLeast` survives every sum and projection; the only `AtLeast` → `Exact` transition is replacement by an eligible usage settlement (X-2). Reservations, proxy charges and declared caps are never a finite maximum (FB-2); unknown cache counts, tiers and identities are enumerated or left unpriceable, never assumed (V-3, V-4) | `AtLeast` present | N01-variant-reservation-above-both; U08; P133-A04 absent cache counts (bounded over vertices) | collapsing to `Exact(max(R, dearest))`; pricing a missing tier or an absent cache count as zero | `high.kind = at_least`, plus `UnresolvedExposure` | N01×4, N08, D4, U08, Q5, P130-R01 | sum or project any set containing `AtLeast`: result `AtLeast` | replace `AtLeast(x)` by `Exact(x)` when x ≥ R |

### INV-F6 r1

| **F6 Totality** | Every evidence line lands in exactly one attempt, non-attempt or unreadable record; nothing is silently dropped or double-assigned | E-1 adapter | R05 (null and [] rows unreadable and blocking) | skipping non-object rows (A1.1 R-05) | validator totality check over all lines | 15 cases incl. R05, U02, N04×4, A1-01, C3 | line count in = Σ records out | `continue` on unparseable lines |

### INV-F7 r1

| **F7 Window independence** | Stages C to X and L take no objective window; their outputs are identical for every objective evaluation | — | Hb (attempt facts identical for windows from 06:00 and 06:20) | A1.3.4 probe H-b (identity finding raised only when counted) | grader: per-evaluation facts equal | Hb, C1, S03, R01, N05, U03, A04 | evaluate under random windows: stage outputs byte-equal | early-return on window placement before validation (A1.3.x `if not keep: return`) |

### INV-F8 r1

| **F8 Order independence** | Presenting sessions or logs in a different order changes nothing; append order matters only within a source | — | ORD twins | dedupe by first-seen across sessions | twin cases equal | ORD×2 | permute sessions: equal | a global `seen` set across sessions |

### INV-F9 r1

| **F9 Pool integrity** | Dollar evidence never leaves `PiApi`; a contradicted lane is never a Codex call; Codex attempts carry no money | — | VC08 (Codex, no money) | N03 (contradicted admission as an unfinished Codex call) | pool and blocking | 10 cases (N03×6, V03, U06, VC08) | any lane-contradicting label gives `PiApi` plus `LaneContradiction` | trust the launch lane over row labels |

### INV-F10 r1

| **F10 Determinism** | Equal inputs and rate table give byte-identical canonical output | — | ORD twins run twice | hash-map iteration order leaking into output | two runs compared byte for byte (harness requirement) | ORD×2 | run twice, compare bytes | an unordered collection serialised directly |

### INV-F12 r1

| **F12 Execution is not billing** | `Execution::Terminal` and `Obligation::Closed` never imply `Billing::Established`; billing is established iff the high is `Exact` (§6.2, R-07) | — | VC01 (terminal, closed, established) | VC04 (terminal, closed by upstream error, unresolved) treated as settled spend | `billing` field equals `high.kind == exact` | every `pi_api` attempt | closed attempts with `AtLeast` high stay unresolved | derive billing from the obligation state |

### INV-F13 r1

| **F13 Replacement eligibility** | `amount_source = SettlementAmount` iff `Valid(Usage)` with a usage bound; an ineligible usage settlement is `ReservationBound` and keeps `R` in the conservative measure; `ReservationConfirmed` only for `Valid(Reservation)` and `Valid(UpstreamError)` (A-01, A-08) | — | VC01 (eligible) | U08 replacing its reservation (P1.32 A-01) | `replacement` and `amount_source` follow the decision and valuation | every budgeted case | — | mark every valid usage settlement eligible |

### INV-F11 r1

| **F11 Supported low** | `low ≤ high`; a non-zero `low` is backed by a usage valuation (basis `usage_valuation`) of reported usage over plausible identities | — | VC03 (low 0.015 from usage under a reservation settlement) | low from a charge or a reservation | basis check | 6 cases incl. Ha×2, N06, A06, U05, VC03 | `low.basis = usage_valuation` whenever `low > 0` | set `low = charged` |

### BOUNDARY-CONSUMERS r1

L owns lifecycle. T owns ownership and placement, copies X money unchanged and reads K only to carry findings. R owns rollup/predecessor/commitment/admission facts; cannot change C-K money. GateVerification must be separately supplied by trusted adapter or explicit test_mock fixture harness bound to exact payload. Raw writer/authority/verified:true/hash are not verification. Real trust channel/anchor/enforcement remains unestablished. Q1/Q4 remain open outside S2a; Q3 nonbillable class empty; Q7 framing remains E-owned; Q9 missing launch stays unresolved and cannot invent identity or release R.

### S2A-SCOPE r1

Select by raw API-launched source dollar reservation/settlement evidence including malformed rid/amount, orphan/no-reservation/conflicting groups and applicable RS1-3 rows. Dollar evidence with missing launch is selected negative unresolved lane/identity control. Never select from expected keys. Exclude genuinely unbudgeted/subscription-only sources. Profiles project separately after raw facts freeze. E1 unreadable remains explicit and never silently drops its case. S2a pure C/S/V/F/X/K only; E/L/T/R/gates/I/O excluded. Entry requires reviewed frozen contract, exact complete A/C verdicts no unresolved P1 and relevant B clearance, independent corpus qualification, production crate/location decision and authorized budget. None is claimed here.

### SERIALIZATION r1

{
  "classification": {
    "additional_blocking": [
      "MalformedEvidence(field)"
    ],
    "existing_codes": "preserved from contract revision5"
  },
  "contract": "pi-accounting-contract/1.5-s1",
  "encoding": {
    "evidence": "<alias>/turn-<n>:<line>",
    "keys": "sorted",
    "money": "nonnegative decimal nano-USD integer string, checked i128",
    "reason_case": "existing PascalCase strings; parameterized reason as Code(field)"
  },
  "field": {
    "tag": "state",
    "variants": {
      "absent": [],
      "malformed": [
        "json_type",
        "raw"
      ],
      "null": [],
      "present": [
        "value"
      ]
    }
  },
  "gate_output_additions": {
    "verification_payload_sha256": "exact verified payload digest when present",
    "verification_source": "test_mock when supplied by separate harness; otherwise unverified"
  },
  "gate_reasons": [
    "GateVerificationMissing",
    "GateVerificationRejected",
    "GateVerificationBinding",
    "GateWriterNotAuthority",
    "GateAuthorityMismatch",
    "GateSessionMismatch",
    "GateScopeInsufficient",
    "GatePeriodInsufficient",
    "GateMechanismMissing",
    "GateReceiptMissing",
    "GateBoundMissing",
    "GateBoundExceedsCap",
    "GateBoundCurrency",
    "GateBoundCoverage",
    "GateBoundBinding"
  ],
  "gate_verification": {
    "rejected": {
      "reasons": "string[]"
    },
    "tag": "state",
    "unverified": {},
    "verified": {
      "authority_id": "string",
      "payload_digest": "sha256",
      "provenance": "test_mock|trusted_adapter",
      "trust_policy_ref": "string",
      "verification_evidence": "string",
      "verifier_identity": "string",
      "verifier_version": "string"
    }
  },
  "limits": "Serialization agreement only. No financial derivation, fixture approval, production verifier or S2a readiness.",
  "obligation": {
    "closed": {
      "amount_source": [
        "settlement_amount",
        "reservation_confirmed",
        "reservation_bound"
      ],
      "by": "EvidenceRef",
      "state": "closed"
    },
    "none": {
      "amount_source": "none",
      "cause": [
        "unbudgeted",
        "no_reservation",
        "orphan",
        "subscription"
      ],
      "state": "none"
    },
    "outstanding": {
      "amount_source": "reservation_bound",
      "state": "outstanding"
    }
  },
  "replacement": {
    "eligible": "bool",
    "reason": [
      "UsageBound",
      "NotUsageSettlement",
      "UsageIncomplete",
      "UsageUnpriceable",
      "NoReservation"
    ]
  },
  "revision": 6,
  "schema": "pi-accounting-interface/1",
  "settlement": {
    "basis": [
      "usage",
      "reservation",
      "upstream_error",
      "error_status"
    ],
    "decision": [
      "absent",
      "valid",
      "rejected",
      "conflicting",
      "not_applicable"
    ],
    "reasons": [
      "ReservationConflict",
      "NoReservation",
      "Order",
      "AttemptNumber",
      "AmountType",
      "ReservedMismatch",
      "BasisMissing",
      "BasisUnknown",
      "KeepAll",
      "StatusType",
      "ErrorCharge",
      "UsageUnpriceable",
      "ModelMismatch",
      "ToleranceRange",
      "Undercharge"
    ]
  },
  "trusted_fixture_inputs": {
    "placement": "top-level sibling of inputs; never inside producer log rows",
    "schema": {
      "gate_verifications": {
        "<gate_id>": {
          "authority_id": "string",
          "payload_sha256": "64 lowercase hex",
          "policy_ref": "string",
          "session": "string",
          "source": "test_mock",
          "verification_evidence": "string"
        }
      }
    }
  },
  "valuation": {
    "existing_fields": [
      "identities",
      "low",
      "high",
      "complete",
      "derivations",
      "reasons",
      "supported_low",
      "priced_high"
    ],
    "identities": "List<String>; exact provider-qualified openai/<model_id> or anthropic/<model_id>; sorted and distinct; derivations retain provider/model",
    "reasons": [
      "IdentityWithoutRate",
      "ModelUnresolved",
      "TierUnestablished(Missing)",
      "TierUnestablished(Unrecognised)",
      "NonStandardTier",
      "MalformedTier",
      "MalformedCount(field)",
      "MissingCount(field)",
      "UsageSchemaMismatch",
      "UsageShape",
      "UnknownUsageSchema",
      "RateTableUnusable"
    ],
    "state": [
      "valued",
      "unpriceable",
      "not_reported",
      "not_applicable"
    ]
  }
}

### CORPUS-ENVELOPE r1

# Corpus schema `pi-accounting-corpus/1`

One JSON file per case: `cases/<case_id>.json`, with canonical key order. Case-envelope ownership remains the independent corpus/validator lane. C-K field shapes are canonical SERIALIZATION; where this historical explanatory sketch omits a new field/reason, the canonical interface applies. Validator qualification is a separate gate.

**Money:** nano-USD integer strings. **Times:** RFC 3339 UTC. **Evidence references:** `<alias>/turn-<n>:<line>`.
**Attempt keys:** `res:<alias>/turn-<n>:rid=<r>`, `adm:<alias>/turn-<n>:rid=<r>`, `row:<alias>/turn-<n>:line=<l>`.

```text
{
  schema: "pi-accounting-corpus/1", contract: "pi-accounting-contract/1.5-s1",      // revision 5 (P1.33); 1.3-s1 P1.32, 1.2-s1 P1.30, 1.1-s1 P1.29, 1.0-s1 P1.28
  profile: current_producer | future_qualified_gate,      // OQ-1 capability profile (CONTRACT 2.3)
  case_id, group, role: finding | control | valid_control,
  class: producer_valid | historical_legacy | malformed | adversarial_synthetic | attribution_uncertain | future_vertical,
  reachability: producer_reachable | producer_unreachable | legacy | not_established,
  findings: [finding ids], provenance: { origin: "<script>::<test>", variant?, also? }, invariants: ["INV-…"],
  controls: [case ids],                                   // mandatory for role=finding
  twin_of?: case id,                                      // order-independence twins (INV-F8)
  revisions: [{rule, field, from, to}],                   // revision 2: every normative field changed from revision 1, with its contract rule
  revisions_r3: [{rule, field, from, to}],                // revision 3: every field changed from revision 2 ("__absent__" = the field did not exist)
  revisions_r4: [{rule, field, from, to}],                // revision 4: every field changed from revision 3 (top-level paths allowed: profile, evidence_pair.*)
  revisions_r6?: [{rule, field, from, to}],               // revision6 changed-policy and representation ledger
  revisions_r5: [{rule, field, from, to}],                // revision 5: every field changed from revision 4 (top-level paths: class, reachability, role, findings, controls)
  added_in?: "1.1-s1" | "1.2-s1" | "1.3-s1" | "1.4-s1",   // a case with no counterpart in the previous revision
  evidence_pair?: {other, relation: subset|superset, permitted_release: bool},   // INV-A5 pairs (R-11)
  inputs: { reference_time, rates: {file: sha256},
            gates?: [{gate_id, authority: {id, kind}, session, scope: {kind: session|request, request_id?}, effective_from, effective_until|null,
                      writer: authority|same_uid_file|declaration, mechanisms: {new_forwarding|retries|reopening|billing_bound: {enforced, receipt?: {ref, sha256},
                      bound?: {amount_nano, currency, scope, covers: [failures|forwarding|partial_streams|retries|settlement], binding_sha256}}}}],   // bound: billing_bound only (G-8)
                                                          // future_qualified_gate only (CONTRACT 2.3)
            sessions: [{ session_id, alias, launch: {model, auth_lane, parent} | null,
                         lifecycle: [{event: launch|deliver|retire, at|null, turn?}],
                         logs: {"turn-N": {lines: [raw line strings, exact bytes], final_newline}},
                         extra_files?: {name: {lines, final_newline}},       // on disk, not named by a turn record
                         ignored_sources?: {...} }] },                         // present but not evidence (V-7)
  evaluations: [{ name,
                  objectives: [{objective, started_at, closed_at|null, limit, tree_sessions, predecessor|null,
                                close?: {at, disposition, reason, set_by},                 // scope_budget close entry (current_producer, legacy cooperative trust)
                                admission_authority?: {id, kind}, closure_record?: {authority_id, at, receipt}, // future_qualified_gate only
                                admissions: [{admission_id, session, source: journal|role_host_objective_admission, rows, gate_id?}]}],
                  admission_request: {objective, new},
                  expected: { attribution: [{attempt, objective|null, ownership: owned|uncertain|unadmitted|unowned, placement, cause?,   // uncertain: candidates include every admitting objective (O-3a)
                                              candidates?, provisional_retained?}],            // unadmitted: objective null, candidates per O-7
                              objectives: [{objective, status, reasons, provisional, owned_closed, owned_outstanding,
                                            predecessor_obligations, commitments: [{admission_id, state, committed, kind: exact|at_least, gate: {qualified, reasons}}]}],   // state adds retired_resumable
                              admission: {objective, new, limit, decision} } }],
  expected: {                                             // shared by every evaluation: window-independent facts
    attempts: [{ key, correlation: {kind: exact|ambiguous|unattributed, candidates?, auxiliary?: [{evidence, rule: RS-1|RS-2|RS-3}]},
                 forwarding: forwarded|unknown, execution: terminal|not_terminal, billing: established|unresolved|not_applicable,
                 replacement: {eligible, reason: UsageBound|NotUsageSettlement|UsageIncomplete|UsageUnpriceable|NoReservation},   // 2.1 (A-01)
                 evidence: [refs], settlement: {decision, basis?, reasons, reserved?, charged?},
                 obligation: {state, amount_source: settlement_amount|reservation_confirmed|reservation_bound|none, by?: EvidenceRef, cause?},   // 6 (A-08)
                 valuation: {state: valued|unpriceable|not_reported|not_applicable, identities?, low?, high?, complete?, derivations?, reasons?, supported_low?, priced_high?},
                 exposure: {pool: pi_api, low: {nano, basis}, high: {kind: exact|at_least, nano, basis}} | {pool: codex, billing: unknown},
                 blocking: [codes], diagnostics: [codes] }],
    non_attempt: [{evidence, kind, blocking}], unreadable: [{evidence, reason, blocking}],
    inventory: [{session, kind, file?, blocking}], lifecycle: {alias: {state: open|retired|contradicted, reopened: bool, clock, reason?}} },
  ambiguity: null | {fields, note},                        // where one answer cannot be established
  notes, rationale: null | text,                           // rationale is required when historical differs
  historical: { a134_086bdec1: {decision, spent_high_nano, spent_low_nano, blocking_kinds, ...} }   // comparison only
}
```

**Derivation records** (`valuation.derivations[]`) are what the validator re-prices from the rate files:

```text
{ provider: openai|anthropic, model, usage, tier_multiplier, nano, for: low|high|both|none,
  remainder_rate?: min|max, split?: reported|ignored }
```

**Revision 3 log** (`revisions_r3`). Same field paths as revision 2, plus `lifecycle[<alias>].<field>` and
`<evaluation>/<objective>.commitments[<admission_id>].<field>`. `validate_corpus.py revisions <corpus> <revision-2 corpus>`
undoes it and requires the revision-2 case back exactly, apart from `contract`, `revisions_r3` and an appended rationale.
The revision-2 `revisions` list is carried unchanged.

**Revision log.** A `revisions` entry's `field` is `attempts[<key>].<path>` or `<evaluation>/attribution[<key>].<field>`,
`<evaluation>/admission.<field>` or `<evaluation>/<objective>.<path>`. `validate_corpus.py revisions <corpus> <previous>`
undoes every entry, newest first, and requires the result to equal the revision-1 case, apart from `contract`,
`revisions`, the attempts' `forwarding` and a rationale that must begin with the revision-1 rationale. It also requires
historical blocks to be unchanged and every revision-1 case and finding tag to be present.

**Result sets** (an implementation's output, graded by `validate_corpus.py results`):

```text
{ case_id: { evaluations: { name: { attempts, non_attempt, unreadable, inventory, lifecycle, attribution, objectives, admission } } } }
```

These are compared field by field after canonical sorting. Attempt facts must be identical across a case's
evaluations (INV-F7).


### V-PRESENCE-INTERFACE r1

V-PRESENCE partitions usage presence/shape only; reported is not a valuation constructor. Reported usage proceeds through VALUATION V1-V8 identity/tier/split/completeness rules. UsagePresence not_reported maps Valuation.not_reported; unpriceable retains explicit reason; reported may yield valued or unpriceable. Guard predicates are declared semantic facts, not a raw evidence interpreter. Their correctness and applicability require independent oracle review.

## Finite declared rule partitions

Guards partition symbolic facts supplied by the declared stage interface. Coverage does not prove these predicates faithfully classify all raw evidence.

### C-KEY-1 r1 (C)

See CORRELATION; usable_reservation

Guard: `{"key_fact":["usable_reservation"]}`. Constructor: `[]`. Formulas: .

### C-KEY-2 r1 (C)

See CORRELATION; unusable_reservation

Guard: `{"key_fact":["unusable_reservation"]}`. Constructor: `[]`. Formulas: .

### C-KEY-3 r1 (C)

See CORRELATION; settlement_no_reservation

Guard: `{"key_fact":["settlement_no_reservation"]}`. Constructor: `[]`. Formulas: .

### C-KEY-4 r1 (C)

See CORRELATION; unbudgeted

Guard: `{"key_fact":["unbudgeted"]}`. Constructor: `[]`. Formulas: .

### C-KEY-5 r1 (C)

See CORRELATION; subscription

Guard: `{"key_fact":["subscription"]}`. Constructor: `[]`. Formulas: .

### C-KEY-6 r1 (C)

See CORRELATION; auxiliary

Guard: `{"key_fact":["auxiliary"]}`. Constructor: `[]`. Formulas: .

### C-KEY-7 r1 (C)

See CORRELATION; ambiguous

Guard: `{"key_fact":["ambiguous"]}`. Constructor: `[]`. Formulas: .

### S-zero-zero r1 (S)

SETTLEMENT applicability and conflict precedence.

Guard: `{"checks":["pass","fail"],"reservations":["zero"],"settlements":["zero"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"not_applicable","fields":[]}]`. Formulas: .

### S-zero-one-pass r1 (S)

Valid iff unique witness and all applicable checks pass; preserve all applicable failures.

Guard: `{"checks":["pass"],"reservations":["zero"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"rejected","fields":["reasons","claimed"]}]`. Formulas: .

### S-zero-one-fail r1 (S)

Valid iff unique witness and all applicable checks pass; preserve all applicable failures.

Guard: `{"checks":["fail"],"reservations":["zero"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"rejected","fields":["reasons","claimed"]}]`. Formulas: .

### S-zero-many r1 (S)

SETTLEMENT applicability and conflict precedence.

Guard: `{"checks":["pass","fail"],"reservations":["zero"],"settlements":["many"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"conflicting","fields":["settlements","claimed"]}]`. Formulas: .

### S-unique-zero r1 (S)

SETTLEMENT applicability and conflict precedence.

Guard: `{"checks":["pass","fail"],"reservations":["unique"],"settlements":["zero"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"absent","fields":[]}]`. Formulas: .

### S-unique-one-pass r1 (S)

Valid iff unique witness and all applicable checks pass; preserve all applicable failures.

Guard: `{"checks":["pass"],"reservations":["unique"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"valid","fields":["basis","charged","settlement"]}]`. Formulas: .

### S-unique-one-fail r1 (S)

Valid iff unique witness and all applicable checks pass; preserve all applicable failures.

Guard: `{"checks":["fail"],"reservations":["unique"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"rejected","fields":["reasons","claimed"]}]`. Formulas: .

### S-unique-many r1 (S)

SETTLEMENT applicability and conflict precedence.

Guard: `{"checks":["pass","fail"],"reservations":["unique"],"settlements":["many"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"conflicting","fields":["settlements","claimed"]}]`. Formulas: .

### S-conflicting-zero r1 (S)

SETTLEMENT applicability and conflict precedence.

Guard: `{"checks":["pass","fail"],"reservations":["conflicting"],"settlements":["zero"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"absent","fields":[]}]`. Formulas: .

### S-conflicting-one-pass r1 (S)

Valid iff unique witness and all applicable checks pass; preserve all applicable failures.

Guard: `{"checks":["pass"],"reservations":["conflicting"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"rejected","fields":["reasons","claimed"]}]`. Formulas: .

### S-conflicting-one-fail r1 (S)

Valid iff unique witness and all applicable checks pass; preserve all applicable failures.

Guard: `{"checks":["fail"],"reservations":["conflicting"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"rejected","fields":["reasons","claimed"]}]`. Formulas: .

### S-conflicting-many r1 (S)

SETTLEMENT applicability and conflict precedence.

Guard: `{"checks":["pass","fail"],"reservations":["conflicting"],"settlements":["many"]}`. Constructor: `[{"type_id":"SettlementDecision","variant":"conflicting","fields":["settlements","claimed"]}]`. Formulas: .

### S-TIME-1 r1 (S)

No reservation; preserve cause.

Guard: `{"temporal_fact":["no_reservation"]}`. Constructor: `[{"type_id":"TemporalObligation","variant":"none","fields":["cause"]}]`. Formulas: .

### S-TIME-2 r1 (S)

Only valid exact key join closes; auxiliary never closes.

Guard: `{"temporal_fact":["valid_exact_join"]}`. Constructor: `[{"type_id":"TemporalObligation","variant":"closed","fields":["by"]}]`. Formulas: .

### S-TIME-3 r1 (S)

Reservation remains Outstanding.

Guard: `{"temporal_fact":["other_reserved"]}`. Constructor: `[{"type_id":"TemporalObligation","variant":"outstanding","fields":[]}]`. Formulas: .

### V-PRESENCE-1 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["absent_or_null"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"not_reported","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-2 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["all_billing_absent_null"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"not_reported","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-3 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["auxiliary_only"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"not_reported","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-4 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["schema_conflict"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-5 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["nonobject"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-6 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["unknown_schema"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-7 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["malformed_billing"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-8 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["missing_required"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-9 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["reported"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"reported","fields":[]}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### F-REPLACEMENT-1 r1 (F)

Replacement reason NoReservation

Guard: `{"replacement_fact":["no_reservation"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"]}]`. Formulas: .

### F-REPLACEMENT-2 r1 (F)

Replacement reason NotUsageSettlement

Guard: `{"replacement_fact":["not_valid_usage"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"]}]`. Formulas: .

### F-REPLACEMENT-3 r1 (F)

Replacement reason UsageUnpriceable

Guard: `{"replacement_fact":["usage_unpriceable"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"]}]`. Formulas: .

### F-REPLACEMENT-4 r1 (F)

Replacement reason UsageIncomplete

Guard: `{"replacement_fact":["usage_incomplete"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"]}]`. Formulas: .

### F-REPLACEMENT-5 r1 (F)

Replacement reason UsageBound

Guard: `{"replacement_fact":["usage_bound"]}`. Constructor: `[{"type_id":"Replacement","variant":"eligible","fields":["reason"]}]`. Formulas: .

### F-OBLIGATION-1 r1 (F)

External projection preserves temporal state/cause/by; all variants carry amount_source. none:none; outstanding:reservation_bound; closed eligible Usage:settlement_amount, valid KeepAll:reservation_confirmed, other:reservation_bound.

Guard: `{"temporal":["none"]}`. Constructor: `[{"type_id":"Obligation","variant":"none","fields":["cause","amount_source"]}]`. Formulas: .

### F-OBLIGATION-2 r1 (F)

External projection preserves temporal state/cause/by; all variants carry amount_source. none:none; outstanding:reservation_bound; closed eligible Usage:settlement_amount, valid KeepAll:reservation_confirmed, other:reservation_bound.

Guard: `{"temporal":["outstanding"]}`. Constructor: `[{"type_id":"Obligation","variant":"outstanding","fields":["amount_source"]}]`. Formulas: .

### F-OBLIGATION-3 r1 (F)

External projection preserves temporal state/cause/by; all variants carry amount_source. none:none; outstanding:reservation_bound; closed eligible Usage:settlement_amount, valid KeepAll:reservation_confirmed, other:reservation_bound.

Guard: `{"temporal":["closed"]}`. Constructor: `[{"type_id":"Obligation","variant":"closed","fields":["by","amount_source"]}]`. Formulas: .

### X-EXPOSURE-1 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["eligible_usage"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: ELIGIBLE-EXPOSURE.

### X-EXPOSURE-2 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["error_status"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: ERROR-EXPOSURE.

### X-EXPOSURE-3 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["reservation"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### X-EXPOSURE-4 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["upstream_error"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### X-EXPOSURE-5 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["absent"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### X-EXPOSURE-6 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["rejected_or_conflicting"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### X-EXPOSURE-7 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["unbudgeted_complete"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: ELIGIBLE-EXPOSURE.

### X-EXPOSURE-8 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["unbudgeted_incomplete"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### X-EXPOSURE-9 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["orphan"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### X-EXPOSURE-10 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["subscription"]}`. Constructor: `[{"type_id":"Exposure","variant":"codex","fields":["billing"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### X-EXPOSURE-11 r1 (X)

Apply exact X-TABLE row and FB invariants; K never feeds money.

Guard: `{"exposure_fact":["lane_contradicted"]}`. Constructor: `[{"type_id":"Exposure","variant":"pi_api","fields":["low","high"]}]`. Formulas: INELIGIBLE-EXPOSURE.

### K-FINDINGS-1 r1 (K)

Emit each applicable CLASSIFICATION finding with evidence and scope; no financial feedback.

Guard: `{"predicate":["true"]}`. Constructor: `[]`. Formulas: .

### K-FINDINGS-2 r1 (K)

Do not emit inapplicable finding.

Guard: `{"predicate":["false"]}`. Constructor: `[]`. Formulas: .

## Serialization interface

```json
{
  "classification": {
    "additional_blocking": [
      "MalformedEvidence(field)"
    ],
    "existing_codes": "preserved from contract revision5"
  },
  "contract": "pi-accounting-contract/1.5-s1",
  "encoding": {
    "evidence": "<alias>/turn-<n>:<line>",
    "keys": "sorted",
    "money": "nonnegative decimal nano-USD integer string, checked i128",
    "reason_case": "existing PascalCase strings; parameterized reason as Code(field)"
  },
  "field": {
    "tag": "state",
    "variants": {
      "absent": [],
      "malformed": [
        "json_type",
        "raw"
      ],
      "null": [],
      "present": [
        "value"
      ]
    }
  },
  "gate_output_additions": {
    "verification_payload_sha256": "exact verified payload digest when present",
    "verification_source": "test_mock when supplied by separate harness; otherwise unverified"
  },
  "gate_reasons": [
    "GateVerificationMissing",
    "GateVerificationRejected",
    "GateVerificationBinding",
    "GateWriterNotAuthority",
    "GateAuthorityMismatch",
    "GateSessionMismatch",
    "GateScopeInsufficient",
    "GatePeriodInsufficient",
    "GateMechanismMissing",
    "GateReceiptMissing",
    "GateBoundMissing",
    "GateBoundExceedsCap",
    "GateBoundCurrency",
    "GateBoundCoverage",
    "GateBoundBinding"
  ],
  "gate_verification": {
    "rejected": {
      "reasons": "string[]"
    },
    "tag": "state",
    "unverified": {},
    "verified": {
      "authority_id": "string",
      "payload_digest": "sha256",
      "provenance": "test_mock|trusted_adapter",
      "trust_policy_ref": "string",
      "verification_evidence": "string",
      "verifier_identity": "string",
      "verifier_version": "string"
    }
  },
  "limits": "Serialization agreement only. No financial derivation, fixture approval, production verifier or S2a readiness.",
  "obligation": {
    "closed": {
      "amount_source": [
        "settlement_amount",
        "reservation_confirmed",
        "reservation_bound"
      ],
      "by": "EvidenceRef",
      "state": "closed"
    },
    "none": {
      "amount_source": "none",
      "cause": [
        "unbudgeted",
        "no_reservation",
        "orphan",
        "subscription"
      ],
      "state": "none"
    },
    "outstanding": {
      "amount_source": "reservation_bound",
      "state": "outstanding"
    }
  },
  "replacement": {
    "eligible": "bool",
    "reason": [
      "UsageBound",
      "NotUsageSettlement",
      "UsageIncomplete",
      "UsageUnpriceable",
      "NoReservation"
    ]
  },
  "revision": 6,
  "schema": "pi-accounting-interface/1",
  "settlement": {
    "basis": [
      "usage",
      "reservation",
      "upstream_error",
      "error_status"
    ],
    "decision": [
      "absent",
      "valid",
      "rejected",
      "conflicting",
      "not_applicable"
    ],
    "reasons": [
      "ReservationConflict",
      "NoReservation",
      "Order",
      "AttemptNumber",
      "AmountType",
      "ReservedMismatch",
      "BasisMissing",
      "BasisUnknown",
      "KeepAll",
      "StatusType",
      "ErrorCharge",
      "UsageUnpriceable",
      "ModelMismatch",
      "ToleranceRange",
      "Undercharge"
    ]
  },
  "trusted_fixture_inputs": {
    "placement": "top-level sibling of inputs; never inside producer log rows",
    "schema": {
      "gate_verifications": {
        "<gate_id>": {
          "authority_id": "string",
          "payload_sha256": "64 lowercase hex",
          "policy_ref": "string",
          "session": "string",
          "source": "test_mock",
          "verification_evidence": "string"
        }
      }
    }
  },
  "valuation": {
    "existing_fields": [
      "identities",
      "low",
      "high",
      "complete",
      "derivations",
      "reasons",
      "supported_low",
      "priced_high"
    ],
    "identities": "List<String>; exact provider-qualified openai/<model_id> or anthropic/<model_id>; sorted and distinct; derivations retain provider/model",
    "reasons": [
      "IdentityWithoutRate",
      "ModelUnresolved",
      "TierUnestablished(Missing)",
      "TierUnestablished(Unrecognised)",
      "NonStandardTier",
      "MalformedTier",
      "MalformedCount(field)",
      "MissingCount(field)",
      "UsageSchemaMismatch",
      "UsageShape",
      "UnknownUsageSchema",
      "RateTableUnusable"
    ],
    "state": [
      "valued",
      "unpriceable",
      "not_reported",
      "not_applicable"
    ]
  }
}
```

## POLICY-R6 owner-policy reference

# Root approval: S1 bounded specification realization

Root01a1d6e2 accepts PLAN.md SHA25652883acb09bcb7d0eb944fe73cc9ba5a3d06932f27a15c430d01bee9f2780a2c with the dispositions below. Authority is the user's delegated ownership of review and future development and $100/four-hour grant to2026-11-14T01:51:14Z. This is approval of architecture-led S1 tooling/contract/corpus work, not freeze, semantic acceptance, production integration or reducer entry. Plan inputs74d72102 and independent raw audit freeze fd3f2e1a/49faf158 remain evidence with stated limits.

New isolated revision:/home/bline/cbm-work/p136-s1r6/s1, contract version1.5-s1, revision6. Predecessor immutable1.4-s1/c2017453. Root owns policy approval, spec builder owns canonical definitions/views; separate oracle builder owns raw-derived acceptance assertions/corpus qualification; Astra owns independent review findings. Neither generator nor builder may overwrite the independent expected lane to pass.

P1: preserve provider family and exact canonical model ID, explicit provider/schema evidence and MC2 absent-provider default. No shape-based cross-provider inference. Full request/launch identity inequality fails ModelMismatch. An independently unpriceable or unestablished identity/schema/tier set cannot provide eligible replacement. Preserve approved complete established fully priced multi-identity finite bounds, with downstream IdentityContradiction blocking, unless independently disproved. Provider-exclusive incompatible non-null billing fields must be reported as UsageSchemaMismatch/Unpriceable; don't treat shared field names or absent/null metadata alone as contrary-family proof. Exact per-family field mapping must be explicit and tied to pinned producer, not inferred by fixture shape.

P2: separately declare each family's billing-bearing and auxiliary fields. Absent/null usage and all-absent/null billing-bearing counts mean NotReported; auxiliary reasoning count alone does not establish a priceable report. Present numeric0 differs. Non-object usage, unknown schema, or malformed billing-bearing counts mean Unpriceable with explicit reason. Missing required reported counts mean MissingCount, never zero. Optional OpenAI cache/write absent/null are unknown feasible splits; Anthropic optional lifetime splits absent/null are zero under existing rule; required aggregate counts remain required. Malformed auxiliary metadata adds MalformedEvidence(field) blocking but does not erase independently computable primary usage or itself select money. Preserve raw malformed states.

Completeness: complete usage bound needs all required well-formed billing counts, established priced identity/tier sets, integer HTTP200, and a recognized successful terminal for the declared provider. OpenAI response.completed/response.done; Anthropic message_stop. A priced partial/incomplete/missing terminal yields supported valuation but not complete replacement. Non200 never ordinary eligible Usage replacement, even with terminal/usage. Newly changed expectations must carry independent raw-policy derivations; don't disguise this explicit V5 ambiguity resolution as representation-only.

P3: require a UniqueReservation witness for reservation-relative equality/order/KeepAll comparisons. With zero/conflicting reservations those checks are inapplicable; record NoReservation/ReservationConflict and all other applicable local field/status/basis failures. Distinct multiple settlements retain Conflicting precedence. R=max distinct well-formed reservation amounts, never sum/min. Preserve evidence identity and malformed-key rows.

P4: approve E→C→{S,V}→F→X→K, with shared pricing primitives and raw input references declared. S owns temporal obligation/execution/forwarding; F owns replacement and amount source after V; external obligation projection includes state and amount_source for every variant (none uses amount_source:none). Closed also preserves its settlement/by reference. Exact serialized names use existing snake_case conventions and are schema-declared. ErrorStatus keeps R and supported usage low/high: AtLeast(max(R,usage high or priced_high)); it closes time without resolving money. K findings never feed financial decisions.

P5: accept the plan's explicitly separate commitment allocation max(0,B−known session owned high) only conditional on a verified whole-session numeric USD billing ceiling with exact scope/period/coverage/authority and trusted adapter provenance. It never changes C-K amounts or makes unresolved attempts Exact. P>B is not alone a contradictory bill claim. If owned high is AtLeast, aggregate remains unresolved regardless of finite mock residual; no false positive admission. Raw authority fields, writer labels, hashes and verified:true do not establish verification. Future fixture positives may use a separate top-level trusted_fixture_inputs mapping supplied by a test-only harness, bound to exact payload; no producer row can mint it. Record test_mock provenance in output. It proves conditional policy only, never live gate qualification. Real verifier/authority/channel/enforcement are outside S1 and remain unestablished.

P6: select API-launched raw sources with usd_reserved or a dollar settlement/usd field, including malformed rid/amount, orphan/no-reservation and conflicting groups plus applicable RS1–3 rows. Do not read expected keys. Additionally include dollar-evidence/missing-launch negative controls as unresolved-lane/identity cases; exclude genuine unbudgeted/subscription-only sources. Keep profile projections separate after independent raw facts are frozen. Raw input invalid under E1 remains explicitly unreadable/excluded evidence, but its case is not silently dropped. Preserve original synthetic raw cases; add writer-consistent positives with independent witnesses instead of relabeling inconsistent bytes as reachable.

D3 explicit scoped amendment: one standalone isolated Rust specification renderer/structural-check crate at tooling/spec-check is permitted in S1, including separately authored independent acceptance assertion tests. It contains no reference financial reducer, expression interpreter, solver, production adapter or workspace integration. Generator has no output destination in oracle/. Shared JSON/serialization shapes are permissible mechanics; oracle arithmetic/semantic assertions cannot import generator/rule-evaluation logic. New executable tooling Rust; existing copied Python corpus tools may be edited, no new shipped Python framework. Production crate/name and S2a budget/entry are separate.

No missing old question is auto-resolved: Q1/Q4 outside S2a remain open; Q3 nonbillable class empty; Q7 E1 framing remains E-owned; Q9 dollar evidence with missing launch is explicitly unresolved, cannot invent priced identity or release R.

Root accepts a small organization revision: two coherent non-overlapping builders under the original S1 objective; no objective addition. Spec builder owns spec/,tooling/spec-check source and manifest,contract/,corpus/SCHEMA.md; oracle builder owns corpus/ excludingSCHEMA.md,oracle/,and tooling/spec-check/tests/independent_*.rs only. Spec builder must not overwrite oracle test files. Root owns shared handoff, joined qualification and review launch.

Freeze remains blocked until reproducible generation, total declared guards/types/read-sets, meaningful historical negatives/controls, independent selected-case coverage, legacy required validatorcheck/selftest/revisions/sabotage, and current exact ModuleA/C plus relevantB clearance. Structural consistency is not financial correctness. If scope becomes a reference reducer, unresolved policy or repeated serious boundary regression, stop that dependent work for root design assessment. Infrastructure failures never count as rejection.

## SOURCE-IDENTITIES owner-policy reference

[
  {
    "git_revision": "92a03789",
    "path": "skills/manual-orchestration/scripts/pi_container.py",
    "sha256": "780837341790780f84163ee6151b8ce66cff56f5a3f6ddf8c15f939dba50f470"
  },
  {
    "git_revision": "92a03789",
    "path": "skills/manual-orchestration/scripts/role_host.py",
    "sha256": "cbca1bc9656ed6de2c1cd43e1e357d7ed27dbaa08b178a3131cc329ed3433475"
  },
  {
    "git_revision": "92a03789",
    "path": "skills/manual-orchestration/scripts/openai_api_rates.json",
    "sha256": "c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58"
  },
  {
    "git_revision": "92a03789",
    "path": "skills/manual-orchestration/scripts/anthropic_api_rates.json",
    "sha256": "8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5"
  }
]
