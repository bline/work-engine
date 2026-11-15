# Generated C–K normative view

Source: spec/ck.json; contract pi-accounting-contract/1.7-s1; input SHA256 87ff8454d427faf9a0ff995f7749bbfec0cd28bab18fd5ca993aa910a8f77184.

This rendering checks declared consistency only. It is not a reducer, financial proof, live trust qualification, freeze or S2a entry.

## Stage interfaces

| Stage | Reads | Outputs |
|---|---|---|
| E |  | typed_evidence |
| C | E | groups, raw_identity, lane, reservations, settlement_validity, primary_groups, enriched_groups |
| S | C | validity_ref, temporal_obligation, execution, forwarding |
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
    },
    {
      "name": "not_applicable",
      "fields": {}
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

### SettlementForward r1

```json
{
  "id": "SettlementForward",
  "revision": 1,
  "variants": [
    {
      "name": "forward",
      "fields": {
        "validity_ref": "String"
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

### TOLERANCE r1

T_round=501 nano; producer_price<10^15 and charged>=producer_price-501; otherwise ToleranceRange or Undercharge.

Inputs: producer_price, charged.

### BASIS-TIE r1

max takes basis of largest; ties prefer UsageValuation, then ProxyReservation, then ProxyCharge. Nonzero supported low has usage basis.

Inputs: components.

### COMMITMENT-RESIDUAL r1

max(0,B-known session owned high) is separate allocation arithmetic conditional on qualified trusted whole-session USD bound; never changes C-K amounts; AtLeast owned aggregate remains unresolved even with finite residual; P>B alone is not contradictory bill evidence.

Inputs: B, owned_high.

### ERROR-EXPOSURE r2

Reference to X-COMPONENTS arithmetic and X-DECISION result-bearing component selection. This compatibility identifier owns no separate branch or formula.

Inputs: components.

### ELIGIBLE-EXPOSURE r2

Reference to X-COMPONENTS arithmetic and X-DECISION result-bearing component selection. This compatibility identifier owns no separate branch or formula.

Inputs: components.

### INELIGIBLE-EXPOSURE r2

Reference to X-COMPONENTS arithmetic and X-DECISION result-bearing component selection. This compatibility identifier owns no separate branch or formula.

Inputs: components.

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

### CORRELATION r2

E inventories immutable origins (lineage, source origin, original position, raw bytes SHA256). Same-origin reobservation is idempotent. InitialReferenceContract fixes admitted namespace/scope/duplicate rule per origin; source revision and producer identity cannot replace it. Exact-copy equivalence is scoped raw-line-byte equivalence under its own admitted guarantee; decoded JSON or equal amounts are insufficient.
C Atomize(B) iff any non-base admitted identity binding, independently distinct-origin fact or retained contradiction affects immutable B. BreakCopies(B) iff distinctness/contradiction separates initially equivalent copies; implies Atomize. Without Atomize one base hold; Atomize one immutable evidence-atom hold; BreakCopies one original-origin hold. Final immutable fact presence controls predicates regardless of arrival order. Matching P labels never remerge. Parent inactive and every origin has one active high owner.
C-primary logical obligations are separate from high units. Count distinct reservation/settlement evidence under admitted exact equivalence, never distinct amounts. C-validity owns immutable witness; S forwards validity_ref exactly and has no writable SettlementDecision constructor. Auxiliary attachment never modifies primary validity. Late source-earlier R may complete missing prerequisites; malformed/order/cardinality/undercharge failures remain.
C admits a context candidate on same logical source/usable n, exact ticket/measurement/producer link, or same source and missing n absent immutable qualified unrelatedness. Later display attachment or narrowed identities never remove a previously justified priced high. Context is read-only pricing evidence, not another bill.

### SETTLEMENT r1

## 5. Settlement decision (primary settlement-validity boundary)

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
The single-owned SETTLEMENT-WITNESS is computed after primary grouping and before auxiliary attachment. S-DECISION declarations expose that witness only; S consumes it.

### TEMPORAL-AND-FINALIZATION r1

## 6. Obligation: releasing versus replacing (stage S)

TemporalObligation = Outstanding | Closed{by} | None{cause}, owned by S. External Obligation = Outstanding{amount_source} | Closed{by,amount_source} | None{cause,amount_source:none}, projected by F. Every variant is representable and carries amount_source.


`Obligation` is an **execution-time** fact: whether a reservation is still open in time. It is not a financial
disposition (§6.2).

**`amount_source` (A-08).** Selected exclusively by the result-bearing finalization bindings owned by F-PROJECTION. All variants and legal combinations are declared by total projections.

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
Financial provisionality is exclusively R-FINANCIAL-PROVISIONAL; closure/retirement does not establish financial finality. Obligation amount_source is selected by F-PROJECTION, never by generic enum sampling.

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

X-DECISION is the sole monetary branch owner. Each legal typed fact tuple has one result: low selects a supported usage component, high uses max of its declared available components. Reservation retained except EligibleUsageBound. Lane contradiction does not erase dollar components. Subscription-clean outputs Codex unknown.

### CLASSIFICATION r1

K emits attempt-local findings from C/S/V/F/X only: malformed evidence, identity/lane contradiction, settlement failures, usage valuation failures and forwarding diagnostics. L owns lifecycle/session findings; T owns objective ownership/admission/window findings; R owns aggregate/commitment/budget findings. Shared code vocabulary never grants K access to L/T/R facts.

### USAGE-PRESENCE r1

Pinned producer 92a03789 pi_container.py writes normalized usage fields. OpenAI billing-bearing: input_tokens, output_tokens, cached_tokens, cache_write_tokens; required when reported: input_tokens/output_tokens. reasoning_tokens is auxiliary. Anthropic billing-bearing: input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens, ephemeral_5m_input_tokens, ephemeral_1h_input_tokens; required: input_tokens/output_tokens/cache_read_input_tokens/cache_creation_input_tokens. Writer mapping verified against 92a03789 pi_container.py ANTHROPIC_USAGE_KEYS and request writer lines1318-1320, budget consumer1048-1054; OpenAI writer1562-1566. Service tier is metadata, not a count; server_tool_use is auxiliary metadata. Schema comes from explicit evidenced family, never fixture shape. Absent provider follows MC-2 OpenAI default, not shape inference. Non-null fields exclusive to the other declared family imply UsageSchemaMismatch. Shared fields and absent/null fields do not.

Check schema conflict, nonobject usage (UsageShape), unknown schema (UnknownUsageSchema), and malformed billing-bearing counts (MalformedCount(field)) before all-null classification. Absent/null usage and all-absent/null billing counts give NotReported; auxiliary alone does not establish reporting. Numeric zero is present. Reported usage missing required counts gives MissingCount(field), never zero. OpenAI optional cache absent/null means unknown feasible splits, Anthropic optional lifetime absent/null means zero. Malformed auxiliary adds MalformedEvidence(field) blocking but preserves independently computable primary usage and cannot select money. Raw states survive.

Completeness requires all required well-formed billing counts, established priced identity/tier sets, integer HTTP200 and provider-recognized successful terminal (OpenAI response.completed/response.done, Anthropic message_stop), with consistent counts. Partial/missing/incomplete terminal or non200 can preserve supported valuation but cannot replace reservation. V-8 partial pricing remains independent. Full family/model inequality fails ModelMismatch. Independently unpriceable/unestablished identity/schema/tier sets cannot replace; established complete fully priced multiple identities retain finite bounds while downstream IdentityContradiction blocks.

### INV-F1 r1

| **F1 Release** | `Obligation::Closed` ⇒ `SettlementDecision::Valid` with an `Exact` **key join** to the attempt's reservation (auxiliary RS-rule rows do not count, R-06). A numeric amount alone never releases | a reservation exists | VC01 (usage settlement closes); N01 (closed with `key_join` Exact and an RS-3 auxiliary row) | S01 (charge 0 against 0.8 under `reservation`) | rejected: `Outstanding`, high `AtLeast(R)`, blocking | 20 cases incl. S01×3, S02×2, U01, R02, C4, S06, V01, V02, U05, N02, R06 | for random settlements failing any §5 condition, `obligation = Outstanding` | drop one §5 condition (e.g. skip the `KeepAll` check) |

### INV-F2 r1

| **F2 No double count** | Per attempt, exposure counts the reservation or the settlement-derived amount, never both. Totals are sums over distinct `AttemptKey`s; byte-identical rows are one item | — | VC01 (high 0.015, not 0.515) | summing the reservation and the settlement charge | Σ exposure = Σ per-attempt | VC01, VC06, VC12, R04, C3, DUP | Σ over attempts = rollup; adding an identical copy changes nothing | add the settlement charge on top of the reservation |

### INV-F3 r1

Correlation/refinement cannot lower held high or remove unresolvedness. Complete affirmative F coverage alone may replace exactly covered holds regardless of which prerequisite arrives last. This explicitly amends former blanket transition prohibitions.

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

X local supported usage lows remain; R computes the separate conservative supported-credit floor per scope under SUPPORTED-LOW. A financial unit is not necessarily an independent physical bill.

### BOUNDARY-CONSUMERS r2

T owns origin/unit/slot attribution and admitted/possible memberships, never money. R sums unique active highs (max per unit; AtLeast0 preserves unresolvedness), recomputes SUPPORTED-LOW within each scope, and owns commitment/provisionality. Findings do not determine money.

### S2A-SCOPE r1

Select by raw API-launched source dollar reservation/settlement evidence including malformed rid/amount, orphan/no-reservation/conflicting groups and applicable RS1-3 rows. Dollar evidence with missing launch is selected negative unresolved lane/identity control. Never select from expected keys. Exclude genuinely unbudgeted/subscription-only sources. Profiles project separately after raw facts freeze. E1 unreadable remains explicit and never silently drops its case. S2a pure C/S/V/F/X/K only; E/L/T/R/gates/I/O excluded. Entry requires reviewed frozen contract, exact complete A/C verdicts no unresolved P1 and relevant B clearance, independent corpus qualification, production crate/location decision and authorized budget. None is claimed here.

### SERIALIZATION r2

{"accounting_evidence": {"attempt_reference": "accounting_result_ref", "kinds": ["accounting_input_proof", "accounting_result", "accounting_expected_assertion"], "low_semantics": "scope supported credits; never sum local attempt lows", "rollup_reference": "supported_low_evidence_ref", "schema": "spec/accounting.schema.json"}, "classification": {"additional_blocking": ["MalformedEvidence(field)"], "existing_codes": "preserved from contract revision5"}, "contract": "pi-accounting-contract/1.7-s1", "encoding": {"evidence": "<alias>/turn-<n>:<line>", "keys": "sorted", "money": "nonnegative decimal nano-USD integer string, checked i128", "reason_case": "existing PascalCase strings; parameterized reason as Code(field)"}, "field": {"tag": "state", "variants": {"absent": [], "malformed": ["json_type", "raw"], "null": [], "present": ["value"]}}, "gate_output_additions": {"verification_payload_sha256": "exact verified payload digest when present", "verification_source": "test_mock when supplied by separate harness; otherwise unverified"}, "gate_reasons": ["GateVerificationMissing", "GateVerificationRejected", "GateVerificationBinding", "GateWriterNotAuthority", "GateAuthorityMismatch", "GateSessionMismatch", "GateScopeInsufficient", "GatePeriodInsufficient", "GateMechanismMissing", "GateReceiptMissing", "GateBoundMissing", "GateBoundExceedsCap", "GateBoundCurrency", "GateBoundCoverage", "GateBoundBinding"], "gate_verification": {"rejected": {"reasons": "string[]"}, "tag": "state", "unverified": {}, "verified": {"authority_id": "string", "payload_digest": "sha256", "provenance": "test_mock|trusted_adapter", "trust_policy_ref": "string", "verification_evidence": "string", "verifier_identity": "string", "verifier_version": "string"}}, "limits": "Serialization agreement only. No financial derivation, fixture approval, production verifier or S2a readiness.", "obligation": {"closed": {"amount_source": ["settlement_amount", "reservation_confirmed", "reservation_bound"], "by": "EvidenceRef", "state": "closed"}, "none": {"amount_source": "none", "cause": ["unbudgeted", "no_reservation", "orphan", "subscription"], "state": "none"}, "outstanding": {"amount_source": "reservation_bound", "state": "outstanding"}}, "replacement": {"eligible": "bool", "reason": ["UsageBound", "NotUsageSettlement", "UsageIncomplete", "UsageUnpriceable", "NoReservation"]}, "revision": 8, "schema": "pi-accounting-interface/1", "settlement": {"basis": ["usage", "reservation", "upstream_error", "error_status"], "decision": ["absent", "valid", "rejected", "conflicting", "not_applicable"], "reasons": ["ReservationConflict", "NoReservation", "Order", "AttemptNumber", "AmountType", "ReservedMismatch", "BasisMissing", "BasisUnknown", "KeepAll", "StatusType", "ErrorCharge", "UsageUnpriceable", "ModelMismatch", "ToleranceRange", "Undercharge"]}, "trusted_fixture_inputs": {"placement": "top-level sibling of inputs; never inside producer log rows", "schema": {"gate_verifications": {"<gate_id>": {"authority_id": "string", "payload_sha256": "64 lowercase hex", "policy_ref": "string", "session": "string", "source": "test_mock", "verification_evidence": "string"}}}}, "valuation": {"existing_fields": ["identities", "low", "high", "complete", "derivations", "reasons", "supported_low", "priced_high"], "identities": "List<String>; exact provider-qualified openai/<model_id> or anthropic/<model_id>; sorted and distinct; derivations retain provider/model", "reasons": ["IdentityWithoutRate", "ModelUnresolved", "TierUnestablished(Missing)", "TierUnestablished(Unrecognised)", "NonStandardTier", "MalformedTier", "MalformedCount(field)", "MissingCount(field)", "UsageSchemaMismatch", "UsageShape", "UnknownUsageSchema", "RateTableUnusable"], "state": ["valued", "unpriceable", "not_reported", "not_applicable"]}}

### CORPUS-ENVELOPE r2

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

### X-COMPONENTS r2

Typed available components: reservation R=max distinct well-formed amounts; charge=max applicable well-formed claimed settlement charges; usage_low/high from valued; supported_low/priced_high from unpriceable (absent optional amount contributes zero); zero=0. high=max declared components with declared exact/at_least tag; low is selected component. No K input. Arithmetic and independent examples require semantic review. Error-status example: AtLeast(max(R, valued.high or priced_high or 0)).

### SETTLEMENT-WITNESS r2

C-primary logical obligations are separate from high units. Count distinct reservation/settlement evidence under admitted exact equivalence, never distinct amounts. C-validity owns immutable witness; S forwards validity_ref exactly and has no writable SettlementDecision constructor. Auxiliary attachment never modifies primary validity. Late source-earlier R may complete missing prerequisites; malformed/order/cardinality/undercharge failures remain.

### F-PROJECTION r1

Single-owned result-bearing finalization bindings select Obligation.amount_source and preserved fields. Projection records enforce legal serialized cross-field combinations. No other table selects this amount source.

### R-FINANCIAL-PROVISIONAL r2

R owns provisional=true iff an owned attempt has Outstanding obligation OR high AtLeast; predecessor_obligations.high is AtLeast; any current non-invalid commitment is AtLeast; OR an uncertain/unadmitted PiApi candidate for this objective has unresolved billing. Finite Exact commitments alone do not imply unresolvedness. Malformed/invalid evidence blocks Established under existing reason, independently. Temporal closure and retirement cannot clear financial unresolvedness.

### INITIAL-REFERENCE r1

E inventories immutable origins (lineage, source origin, original position, raw bytes SHA256). Same-origin reobservation is idempotent. InitialReferenceContract fixes admitted namespace/scope/duplicate rule per origin; source revision and producer identity cannot replace it. Exact-copy equivalence is scoped raw-line-byte equivalence under its own admitted guarantee; decoded JSON or equal amounts are insufficient.

### ATOMIZATION r1

C Atomize(B) iff any non-base admitted identity binding, independently distinct-origin fact or retained contradiction affects immutable B. BreakCopies(B) iff distinctness/contradiction separates initially equivalent copies; implies Atomize. Without Atomize one base hold; Atomize one immutable evidence-atom hold; BreakCopies one original-origin hold. Final immutable fact presence controls predicates regardless of arrival order. Matching P labels never remerge. Parent inactive and every origin has one active high owner.

### LOGICAL-PRIMARY r1

C-primary logical obligations are separate from high units. Count distinct reservation/settlement evidence under admitted exact equivalence, never distinct amounts. C-validity owns immutable witness; S forwards validity_ref exactly and has no writable SettlementDecision constructor. Auxiliary attachment never modifies primary validity. Late source-earlier R may complete missing prerequisites; malformed/order/cardinality/undercharge failures remain.

### FINANCIAL-CANDIDATES r1

C admits a context candidate on same logical source/usable n, exact ticket/measurement/producer link, or same source and missing n absent immutable qualified unrelatedness. Later display attachment or narrowed identities never remove a previously justified priced high. Context is read-only pricing evidence, not another bill.

### DEFINITE-LINK r1

Exact row co-reference consumes only an admitted exact_row_coreference grant for the exact origins/extent. RS/n uniqueness, row adjacency, labels and writer hashes are insufficient. Independence, scope, copy, continuity and co-reference guarantees cannot substitute for one another.

### WHOLE-COVERAGE r1

F alone releases exactly complete current hold units with complete origin coverage, C-validity and complete V witnesses and admitted exact links for all auxiliary anchors. Retain known charge/context; no partial-overlap release. Independent physical bills cannot be merged. Conflicting overlapping coverage retains original/refined holds. Disjoint/identical exact coverage composes/idempotent. Complete local unbudgeted usage cannot discharge an unlinked AtLeast0 anchor.

### SUPPORTED-LOW r1

X local usage lows survive rejected/conflicting settlement. R uses separate physical-credit lows, never sum(attempt lows). For scope U and positive usage seed, single_floor=V8Low(U); each admitted independently billed owned slot p has Alternatives(p)=all U not affirmatively physically distinct. independent_floor=sum(V8Low(Alternatives(p))); R.low=max(single_floor, independent_floor). V8Low is min across complete alternatives, zero if unsupported. Contradictory/unestablished frame admits no slot addition. T provides owned/possible memberships; R recomputes for each scope. Ties prefer independent_slots if nonempty. Positive basis usage_valuation, otherwise none.

### CONTINUITY r1

Successor preserves every predecessor origin, immutable initial contract, trigger fact and conflict under exact origin mapping. Omission/rewriting/unsupported many-to-one fails admission; last admitted financial subject retained. Digest rename cannot create fresh bases. Live continuity remains unestablished.

### EXACT-WIRE r2

Actual external projections: exposure.pool; high.kind/nano/basis with flattened Bound; decimal nano strings; valuation low/high as decimal strings and derivations from splits; reason strings qualified. Obligation none preserves cause/amount_source:none and closed preserves by. Per-attempt accounting_result_ref and rollup supported_low_evidence_ref refer to computed results, never input grants or expected assertions. Finite corpus refs use oracle/r8-completion-A|B/<case_id>.bundle.json#result:<case_id>, with units selected by exact attempt.key and exposure_ref case:<case_id>#attempt:<key>. Every evaluation/objective/owned_closed|owned_outstanding scope, including empty scopes, uses bundle-path#<case_id>/<evaluation>/<objective>/<label> and resolves exactly one LowRollup. Input source_revision_ref binds SHA256 of sorted compact JSON case.inputs (ensure_ascii=true). Whole corpus consumption requires175cases/374scopes and independent raw-bound owned-unit/seed/alternative memberships. Exact PiApi wire units require accepted complete F coverage and resolved C/valuation witnesses; expected literals never supply eligibility.

### RAW-FIELD-PREDICATES r1

Usage presence, JSON type, count cardinality, malformed identity/tier, exact duplicate identity, atomization, financial candidates, exact linkage, low alternatives, full origin coverage and continuity are semantic predicates. Their actual consumers must invalidate on revisions, independently of whole-file byte drift.

### SCHEMA-ACCOUNTING r1

{"$defs": {"AccountingExpectedAssertion": {"additionalProperties": false, "properties": {"assertions": {"items": {"$ref": "#/$defs/Assertion"}, "type": "array", "uniqueItems": true}, "authored_by": {"minLength": 1, "type": "string"}, "exact_subject_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "id": {"minLength": 1, "type": "string"}, "kind": {"const": "accounting_expected_assertion"}, "policy_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "raw_subject_ref": {"minLength": 1, "type": "string"}, "version": {"const": "1.7-s1"}}, "required": ["kind", "id", "version", "raw_subject_ref", "policy_refs", "authored_by", "exact_subject_refs", "assertions"], "type": "object"}, "AccountingInputProof": {"additionalProperties": false, "properties": {"continuity": {"$ref": "#/$defs/Continuity"}, "exact_links": {"items": {"$ref": "#/$defs/InputLink"}, "type": "array", "uniqueItems": true}, "grants": {"items": {"$ref": "#/$defs/Grant"}, "type": "array", "uniqueItems": true}, "id": {"minLength": 1, "type": "string"}, "identity_bindings": {"items": {"$ref": "#/$defs/IdentityBinding"}, "type": "array", "uniqueItems": true}, "independence_frame": {"$ref": "#/$defs/IndependenceFrame"}, "initial_contracts": {"items": {"$ref": "#/$defs/InitialReferenceContract"}, "type": "array", "uniqueItems": true}, "kind": {"const": "accounting_input_proof"}, "lineage_id": {"minLength": 1, "type": "string"}, "origins": {"items": {"$ref": "#/$defs/Origin"}, "type": "array", "uniqueItems": true}, "version": {"const": "1.7-s1"}}, "required": ["kind", "id", "version", "lineage_id", "origins", "initial_contracts", "grants", "identity_bindings", "exact_links", "continuity"], "type": "object"}, "AccountingResult": {"additionalProperties": false, "properties": {"bases": {"items": {"$ref": "#/$defs/Base"}, "type": "array", "uniqueItems": true}, "continuity_result": {"enum": ["established", "conditional", "unestablished"]}, "coverage_decisions": {"items": {"$ref": "#/$defs/Coverage"}, "type": "array", "uniqueItems": true}, "definite_links": {"items": {"$ref": "#/$defs/DefiniteLink"}, "type": "array", "uniqueItems": true}, "failure_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "financial_candidates": {"items": {"$ref": "#/$defs/Candidate"}, "type": "array", "uniqueItems": true}, "id": {"minLength": 1, "type": "string"}, "input_proof_ref": {"minLength": 1, "type": "string"}, "kind": {"const": "accounting_result"}, "lineage_id": {"minLength": 1, "type": "string"}, "low_rollups": {"items": {"$ref": "#/$defs/LowRollup"}, "type": "array", "uniqueItems": true}, "origins": {"items": {"$ref": "#/$defs/OriginOwner"}, "type": "array", "uniqueItems": true}, "primary_validities": {"items": {"$ref": "#/$defs/PrimaryValidity"}, "type": "array", "uniqueItems": true}, "unbudgeted_applicabilities": {"items": {"$ref": "#/$defs/UnbudgetedApplicability"}, "type": "array", "uniqueItems": true}, "unbudgeted_completions": {"items": {"$ref": "#/$defs/UnbudgetedCompletion"}, "type": "array", "uniqueItems": true}, "units": {"items": {"$ref": "#/$defs/Unit"}, "type": "array", "uniqueItems": true}, "valuation_records": {"items": {"$ref": "#/$defs/ValuationRecord"}, "type": "array", "uniqueItems": true}, "version": {"const": "1.7-s1"}}, "required": ["kind", "id", "version", "input_proof_ref", "lineage_id", "origins", "bases", "units", "primary_validities", "financial_candidates", "definite_links", "coverage_decisions", "low_rollups", "continuity_result", "failure_refs"], "type": "object"}, "Assertion": {"additionalProperties": false, "properties": {"claim_id": {"minLength": 1, "type": "string"}, "expected_fields_or_relations": {"type": "object"}, "negative_control_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "raw_derivation_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}}, "required": ["claim_id", "raw_derivation_refs", "expected_fields_or_relations", "negative_control_refs"], "type": "object"}, "Base": {"additionalProperties": false, "properties": {"atomize": {"type": "boolean"}, "atomize_fact_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "base_id": {"minLength": 1, "type": "string"}, "break_copies": {"type": "boolean"}, "break_copies_fact_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}}, "required": ["base_id", "origin_refs", "atomize", "atomize_fact_refs", "break_copies", "break_copies_fact_refs"], "type": "object"}, "Candidate": {"additionalProperties": false, "properties": {"context_origin_ref": {"minLength": 1, "type": "string"}, "predicate_ref": {"minLength": 1, "type": "string"}, "usage_origin_ref": {"minLength": 1, "type": "string"}}, "required": ["usage_origin_ref", "context_origin_ref", "predicate_ref"], "type": "object"}, "Continuity": {"additionalProperties": false, "properties": {"exact_origin_map_ref": {"minLength": 1, "type": "string"}, "predecessor_origin_inventory_ref": {"minLength": 1, "type": "string"}, "predecessor_subject_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "preserved_fact_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "preserved_initial_contract_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}}, "required": ["predecessor_subject_refs", "preserved_initial_contract_refs", "preserved_fact_refs", "exact_origin_map_ref"], "type": "object"}, "Coverage": {"additionalProperties": false, "properties": {"anchor_origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "completion_ref": {"minLength": 1, "type": "string"}, "coverage_id": {"minLength": 1, "type": "string"}, "definite_link_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "monetary_origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "obligation_id": {"minLength": 1, "type": "string"}, "reason_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "result_unit_ref": {"minLength": 1, "type": "string"}, "retained_component_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "route": {"enum": ["budgeted", "unbudgeted_completion"]}, "status": {"enum": ["accepted", "rejected"]}, "unit_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "validity_ref": {"minLength": 1, "type": "string"}, "valuation_ref": {"minLength": 1, "type": "string"}}, "required": ["coverage_id", "status", "unit_refs", "monetary_origin_refs", "anchor_origin_refs", "definite_link_refs", "retained_component_refs", "reason_refs", "route"], "type": "object"}, "Credit": {"additionalProperties": false, "properties": {"alternative_origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "low": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}, "slot_ref": {"minLength": 1, "type": "string"}, "valuation_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}}, "required": ["slot_ref", "alternative_origin_refs", "low", "valuation_refs"], "type": "object"}, "DefiniteLink": {"additionalProperties": false, "properties": {"admitted_input_link_ref": {"minLength": 1, "type": "string"}, "from_origin_ref": {"minLength": 1, "type": "string"}, "to_origin_ref": {"minLength": 1, "type": "string"}}, "required": ["from_origin_ref", "to_origin_ref", "admitted_input_link_ref"], "type": "object"}, "Grant": {"additionalProperties": false, "properties": {"admission": {"enum": ["conditional_fixture", "established"]}, "assumption_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "authority_ref": {"minLength": 1, "type": "string"}, "covered_extent_ref": {"minLength": 1, "type": "string"}, "covered_origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "exact_payload_ref": {"minLength": 1, "type": "string"}, "grant_id": {"minLength": 1, "type": "string"}, "guarantee_kind": {"enum": ["initial_reference_scope", "exact_copy_equivalence", "producer_attempt_binding", "exact_row_coreference", "physical_bill_independence", "origin_slot_binding", "origin_slot_distinctness", "continuity"]}, "incarnation_ref": {"minLength": 1, "type": "string"}, "observer_or_harness_ref": {"minLength": 1, "type": "string"}, "producer_revision_ref": {"minLength": 1, "type": "string"}}, "required": ["grant_id", "guarantee_kind", "admission", "exact_payload_ref", "covered_origin_refs", "covered_extent_ref", "authority_ref", "observer_or_harness_ref", "assumption_refs"], "type": "object"}, "IdentityBinding": {"additionalProperties": false, "properties": {"grant_ref": {"minLength": 1, "type": "string"}, "origin_ref": {"minLength": 1, "type": "string"}, "producer_attempt_id": {"minLength": 1, "type": "string"}}, "required": ["origin_ref", "producer_attempt_id", "grant_ref"], "type": "object"}, "IndependenceFrame": {"additionalProperties": false, "properties": {"currency": {"const": "USD"}, "distinct_origin_slot": {"items": {"$ref": "#/$defs/SlotBinding"}, "type": "array", "uniqueItems": true}, "frame_id": {"minLength": 1, "type": "string"}, "grant_ref": {"minLength": 1, "type": "string"}, "origin_slots": {"items": {"$ref": "#/$defs/SlotBinding"}, "type": "array", "uniqueItems": true}, "slots": {"items": {"$ref": "#/$defs/Slot"}, "type": "array", "uniqueItems": true}}, "required": ["frame_id", "currency", "grant_ref", "slots", "origin_slots", "distinct_origin_slot"], "type": "object"}, "InitialReferenceContract": {"additionalProperties": false, "properties": {"contract_id": {"minLength": 1, "type": "string"}, "duplicate_rule": {"enum": ["scope_exact_line", "same_origin_only"]}, "grant_ref": {"minLength": 1, "type": "string"}, "reference_namespace": {"enum": ["dollar_ticket", "admission", "unestablished"]}, "scope_id": {"minLength": 1, "type": "string"}}, "required": ["contract_id", "reference_namespace", "duplicate_rule"], "type": "object"}, "InputLink": {"additionalProperties": false, "properties": {"from_origin_ref": {"minLength": 1, "type": "string"}, "grant_ref": {"minLength": 1, "type": "string"}, "to_origin_ref": {"minLength": 1, "type": "string"}}, "required": ["from_origin_ref", "to_origin_ref", "grant_ref"], "type": "object"}, "LowRollup": {"additionalProperties": false, "properties": {"alternative_origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "basis": {"enum": ["usage_valuation", "none"]}, "credit_mode": {"enum": ["independent_slots", "single_ambiguous", "none"]}, "credits": {"items": {"$ref": "#/$defs/Credit"}, "type": "array", "uniqueItems": true}, "independent_floor": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}, "scope_ref": {"minLength": 1, "type": "string"}, "seed_origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "selected_low": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}, "single_floor": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}}, "required": ["scope_ref", "seed_origin_refs", "alternative_origin_refs", "single_floor", "credits", "independent_floor", "selected_low", "credit_mode", "basis"], "type": "object"}, "Origin": {"additionalProperties": false, "properties": {"initial_contract_ref": {"minLength": 1, "type": "string"}, "origin_id": {"minLength": 1, "type": "string"}, "original_position": {"minimum": 0, "type": "integer"}, "raw_bytes_sha256": {"pattern": "^[0-9a-f]{64}$", "type": "string"}, "source_origin_id": {"minLength": 1, "type": "string"}, "source_revision_ref": {"minLength": 1, "type": "string"}}, "required": ["origin_id", "source_origin_id", "original_position", "raw_bytes_sha256", "source_revision_ref", "initial_contract_ref"], "type": "object"}, "OriginOwner": {"additionalProperties": false, "properties": {"active_high_unit_id": {"minLength": 1, "type": "string"}, "base_id": {"minLength": 1, "type": "string"}, "evidence_atom_id": {"minLength": 1, "type": "string"}, "origin_ref": {"minLength": 1, "type": "string"}}, "required": ["origin_ref", "base_id", "evidence_atom_id", "active_high_unit_id"], "type": "object"}, "PrimaryValidity": {"additionalProperties": false, "properties": {"applicable_check_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "distinct_reservation_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "distinct_settlement_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "obligation_id": {"minLength": 1, "type": "string"}, "validity_result": {"enum": ["valid", "rejected", "conflicting", "absent", "not_applicable"]}}, "required": ["obligation_id", "distinct_reservation_refs", "distinct_settlement_refs", "validity_result", "applicable_check_refs"], "type": "object"}, "Slot": {"additionalProperties": false, "properties": {"bill_occurrence_ref": {"minLength": 1, "type": "string"}, "grant_ref": {"minLength": 1, "type": "string"}, "slot_id": {"minLength": 1, "type": "string"}}, "required": ["slot_id", "bill_occurrence_ref", "grant_ref"], "type": "object"}, "SlotBinding": {"additionalProperties": false, "properties": {"grant_ref": {"minLength": 1, "type": "string"}, "origin_ref": {"minLength": 1, "type": "string"}, "slot_id": {"minLength": 1, "type": "string"}}, "required": ["origin_ref", "slot_id", "grant_ref"], "type": "object"}, "UnbudgetedApplicability": {"additionalProperties": false, "properties": {"applicability_id": {"minLength": 1, "type": "string"}, "classification": {"const": "genuine_unbudgeted"}, "dollar_origin_refs": {"maxItems": 0, "type": "array"}, "raw_subject_ref": {"minLength": 1, "type": "string"}, "usage_origin_ref": {"minLength": 1, "type": "string"}}, "required": ["applicability_id", "usage_origin_ref", "raw_subject_ref", "classification", "dollar_origin_refs"], "type": "object"}, "UnbudgetedCompletion": {"additionalProperties": false, "properties": {"applicability_ref": {"minLength": 1, "type": "string"}, "complete": {"const": true}, "completion_id": {"minLength": 1, "type": "string"}, "usage_origin_ref": {"minLength": 1, "type": "string"}, "valuation_ref": {"minLength": 1, "type": "string"}}, "required": ["completion_id", "usage_origin_ref", "applicability_ref", "valuation_ref", "complete"], "type": "object"}, "Unit": {"additionalProperties": false, "properties": {"exposure_ref": {"minLength": 1, "type": "string"}, "origin_refs": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "state": {"enum": ["active", "inactive"]}, "successor_ref": {"minLength": 1, "type": "string"}, "unit_id": {"minLength": 1, "type": "string"}}, "required": ["unit_id", "state", "origin_refs", "exposure_ref"], "type": "object"}, "ValuationRecord": {"additionalProperties": false, "properties": {"complete": {"type": "boolean"}, "usage_origin_refs": {"items": {"minLength": 1, "type": "string"}, "minItems": 1, "type": "array", "uniqueItems": true}, "valuation_id": {"minLength": 1, "type": "string"}, "wire_ref": {"minLength": 1, "type": "string"}}, "required": ["valuation_id", "usage_origin_refs", "complete", "wire_ref"], "type": "object"}}, "$id": "urn:pi-accounting:evidence:1.7-s1", "$schema": "https://json-schema.org/draft/2020-12/schema", "oneOf": [{"$ref": "#/$defs/AccountingInputProof"}, {"$ref": "#/$defs/AccountingResult"}, {"$ref": "#/$defs/AccountingExpectedAssertion"}]}

### SCHEMA-WIRE r1

{"$schema": "https://json-schema.org/draft/2020-12/schema", "additionalProperties": false, "properties": {"accounting_result_ref": {"minLength": 1, "type": "string"}, "exposure": {"oneOf": [{"additionalProperties": false, "properties": {"high": {"additionalProperties": false, "properties": {"basis": {"enum": ["usage_valuation", "proxy_reservation", "proxy_charge", "none"]}, "kind": {"enum": ["exact", "at_least"]}, "nano": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}}, "required": ["kind", "nano", "basis"], "type": "object"}, "low": {"additionalProperties": false, "properties": {"basis": {"enum": ["usage_valuation", "proxy_reservation", "proxy_charge", "none"]}, "nano": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}}, "required": ["nano", "basis"], "type": "object"}, "pool": {"const": "pi_api"}}, "required": ["pool", "low", "high"], "type": "object"}, {"additionalProperties": false, "properties": {"billing": {"minLength": 1, "type": "string"}, "pool": {"const": "codex"}}, "required": ["pool"], "type": "object"}]}, "obligation": {"oneOf": [{"additionalProperties": false, "properties": {"amount_source": {"const": "none"}, "cause": {"enum": ["unbudgeted", "no_reservation", "orphan", "subscription"]}, "state": {"const": "none"}}, "required": ["state", "amount_source", "cause"], "type": "object"}, {"additionalProperties": false, "properties": {"amount_source": {"const": "reservation_bound"}, "state": {"const": "outstanding"}}, "required": ["state", "amount_source"], "type": "object"}, {"additionalProperties": false, "properties": {"amount_source": {"enum": ["settlement_amount", "reservation_confirmed", "reservation_bound"]}, "by": {"minLength": 1, "type": "string"}, "state": {"const": "closed"}}, "required": ["state", "amount_source", "by"], "type": "object"}]}, "valuation": {"oneOf": [{"additionalProperties": false, "properties": {"state": {"enum": ["not_reported", "not_applicable"]}}, "required": ["state"], "type": "object"}, {"additionalProperties": false, "properties": {"complete": {"type": "boolean"}, "derivations": {"type": "array"}, "high": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}, "identities": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "low": {"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}, "reasons": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "state": {"const": "valued"}}, "required": ["state", "low", "high", "identities", "complete", "derivations"], "type": "object"}, {"additionalProperties": false, "properties": {"derivations": {"type": "array"}, "identities": {"items": {"minLength": 1, "type": "string"}, "type": "array", "uniqueItems": true}, "priced_high": {"oneOf": [{"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}, {"type": "null"}]}, "reasons": {"items": {"oneOf": [{"enum": ["IdentityWithoutRate", "ModelUnresolved", "TierUnestablished(Missing)", "TierUnestablished(Unrecognised)", "NonStandardTier", "MalformedTier", "UsageSchemaMismatch", "UsageShape", "UnknownUsageSchema", "RateTableUnusable"]}, {"pattern": "^(MalformedCount|MissingCount)\\([A-Za-z_][A-Za-z0-9_]*\\)$", "type": "string"}]}, "type": "array", "uniqueItems": true}, "state": {"const": "unpriceable"}, "supported_low": {"oneOf": [{"pattern": "^(0|[1-9][0-9]*)$", "type": "string"}, {"type": "null"}]}}, "required": ["state", "reasons"], "type": "object"}]}}, "required": ["exposure"], "type": "object"}

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

### S-zero-zero r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass","fail"],"reservations":["zero"],"settlements":["zero"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-zero-one-pass r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass"],"reservations":["zero"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-zero-one-fail r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["fail"],"reservations":["zero"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-zero-many r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass","fail"],"reservations":["zero"],"settlements":["many"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-unique-zero r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass","fail"],"reservations":["unique"],"settlements":["zero"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-unique-one-pass r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass"],"reservations":["unique"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-unique-one-fail r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["fail"],"reservations":["unique"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-unique-many r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass","fail"],"reservations":["unique"],"settlements":["many"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-conflicting-zero r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass","fail"],"reservations":["conflicting"],"settlements":["zero"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-conflicting-one-pass r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass"],"reservations":["conflicting"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-conflicting-one-fail r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["fail"],"reservations":["conflicting"],"settlements":["one"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-conflicting-many r2 (S)

S forwards the exact immutable C-validity witness reference. This symbolic row owns no settlement decision; its checks/cardinality dimensions cannot construct or change validity.

Guard: `{"checks":["pass","fail"],"reservations":["conflicting"],"settlements":["many"]}`. Constructor: `[{"type_id":"SettlementForward","variant":"forward","fields":["validity_ref"],"bindings":{"validity_ref":"C-validity.validity_ref"}}]`. Formulas: .

### S-TIME-1 r1 (S)

No reservation; preserve cause.

Guard: `{"temporal_fact":["no_reservation"]}`. Constructor: `[{"type_id":"TemporalObligation","variant":"none","fields":["cause"],"bindings":{}}]`. Formulas: .

### S-TIME-2 r1 (S)

Only valid exact key join closes; auxiliary never closes.

Guard: `{"temporal_fact":["valid_exact_join"]}`. Constructor: `[{"type_id":"TemporalObligation","variant":"closed","fields":["by"],"bindings":{}}]`. Formulas: .

### S-TIME-3 r1 (S)

Reservation remains Outstanding.

Guard: `{"temporal_fact":["other_reserved"]}`. Constructor: `[{"type_id":"TemporalObligation","variant":"outstanding","fields":[],"bindings":{}}]`. Formulas: .

### V-PRESENCE-1 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["absent_or_null"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"not_reported","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-2 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["all_billing_absent_null"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"not_reported","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-3 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["auxiliary_only"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"not_reported","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-4 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["schema_conflict"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-5 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["nonobject"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-6 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["unknown_schema"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-7 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["malformed_billing"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-8 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["missing_required"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"unpriceable","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### V-PRESENCE-9 r1 (V)

USAGE-PRESENCE ordered predicates; preserve raw field states.

Guard: `{"usage_fact":["reported"]}`. Constructor: `[{"type_id":"UsagePresence","variant":"reported","fields":[],"bindings":{}}]`. Formulas: PRICE-OPENAI, PRICE-ANTHROPIC.

### F-REPLACEMENT-1 r1 (F)

Replacement reason NoReservation

Guard: `{"replacement_fact":["no_reservation"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"],"bindings":{"reason":"NoReservation"}}]`. Formulas: .

### F-REPLACEMENT-2 r1 (F)

Replacement reason NotUsageSettlement

Guard: `{"replacement_fact":["not_valid_usage"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"],"bindings":{"reason":"NotUsageSettlement"}}]`. Formulas: .

### F-REPLACEMENT-3 r1 (F)

Replacement reason UsageUnpriceable

Guard: `{"replacement_fact":["usage_unpriceable"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"],"bindings":{"reason":"UsageUnpriceable"}}]`. Formulas: .

### F-REPLACEMENT-4 r1 (F)

Replacement reason UsageIncomplete

Guard: `{"replacement_fact":["usage_incomplete"]}`. Constructor: `[{"type_id":"Replacement","variant":"ineligible","fields":["reason"],"bindings":{"reason":"UsageIncomplete"}}]`. Formulas: .

### F-REPLACEMENT-5 r1 (F)

Replacement reason UsageBound

Guard: `{"replacement_fact":["usage_bound"]}`. Constructor: `[{"type_id":"Replacement","variant":"eligible","fields":["reason"],"bindings":{"reason":"UsageBound"}}]`. Formulas: .

### K-FINDINGS-1 r1 (K)

Emit each applicable CLASSIFICATION finding with evidence and scope; no financial feedback.

Guard: `{"predicate":["true"]}`. Constructor: `[]`. Formulas: .

### K-FINDINGS-2 r1 (K)

Do not emit inapplicable finding.

Guard: `{"predicate":["false"]}`. Constructor: `[]`. Formulas: .

## Result-bearing decision X-DECISION

```json
{
  "id": "X-DECISION",
  "revision": 1,
  "stage": "X",
  "dimensions": {
    "charge": [
      "none",
      "present"
    ],
    "lane": [
      "pi_api",
      "unbudgeted",
      "subscription"
    ],
    "reservation": [
      "none",
      "retained"
    ],
    "settlement": [
      "usage_eligible",
      "usage_ineligible",
      "reservation",
      "upstream_error",
      "error_status",
      "absent",
      "rejected",
      "conflicting",
      "not_applicable"
    ],
    "valuation": [
      "complete",
      "incomplete",
      "unpriceable",
      "not_reported",
      "not_applicable"
    ]
  },
  "branches": [
    {
      "id": "X-BRANCH-001",
      "revision": 1,
      "guard": {
        "charge": [
          "none",
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "error_status",
          "reservation",
          "upstream_error",
          "usage_eligible",
          "usage_ineligible"
        ],
        "valuation": [
          "complete",
          "incomplete",
          "not_applicable",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "Valid settlement requires unique reservation and a well-formed charge; witness owned by settlement validity.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-002",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "absent",
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "complete",
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-003",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "absent",
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-004",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "absent",
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "not_applicable",
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "zero"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-0",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-1",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-2",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "zero"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-3",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "zero"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-4",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-5",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "reservation",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-6",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "reservation",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-7",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "reservation"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-8",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "reservation"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-9",
      "revision": 2,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "reservation",
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-10",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "charge",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-11",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "charge",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-12",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "charge"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-13",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "charge"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-14",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "charge",
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-15",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "reservation",
          "charge",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-16",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "reservation",
          "charge",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-17",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "reservation",
          "charge"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-18",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "reservation",
          "charge"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-005-anchor-19",
      "revision": 2,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "reservation",
          "charge",
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-006",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "error_status",
          "reservation",
          "upstream_error",
          "usage_eligible",
          "usage_ineligible"
        ],
        "valuation": [
          "complete",
          "incomplete",
          "not_applicable",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "Valid settlement requires unique reservation and a well-formed charge; witness owned by settlement validity.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-007",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "absent",
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "complete",
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "reservation",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-008",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "absent",
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "reservation",
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-009",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "absent",
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "not_applicable",
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "reservation"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-010",
      "revision": 1,
      "guard": {
        "charge": [
          "none",
          "present"
        ],
        "lane": [
          "unbudgeted"
        ],
        "reservation": [
          "none",
          "retained"
        ],
        "settlement": [
          "absent",
          "conflicting",
          "error_status",
          "rejected",
          "reservation",
          "upstream_error",
          "usage_eligible",
          "usage_ineligible"
        ],
        "valuation": [
          "complete",
          "incomplete",
          "not_applicable",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "Unbudgeted requests have no reservation-backed settlement or settlement charge.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-011",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "unbudgeted"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "exact",
        "components": [
          "usage_high"
        ],
        "billing": "established",
        "witnesses": [
          "CompleteUnbudgetedUsage"
        ]
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-012",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "unbudgeted"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-013",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "unbudgeted"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-014",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "unbudgeted"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable",
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "zero"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-015",
      "revision": 1,
      "guard": {
        "charge": [
          "none",
          "present"
        ],
        "lane": [
          "unbudgeted"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete",
          "incomplete",
          "not_applicable",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "Unbudgeted requests have no reservation-backed settlement or settlement charge.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-016",
      "revision": 1,
      "guard": {
        "charge": [
          "none",
          "present"
        ],
        "lane": [
          "subscription"
        ],
        "reservation": [
          "none",
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete",
          "incomplete",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "NotApplicable clean subscription has no dollar components; a dollar-evidence group has applicable settlement absent/rejected/conflicting/valid, regardless of launch lane.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-017",
      "revision": 1,
      "guard": {
        "charge": [
          "none"
        ],
        "lane": [
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "codex",
        "low": null,
        "high": null,
        "components": [],
        "billing": "unknown",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-018",
      "revision": 1,
      "guard": {
        "charge": [
          "none",
          "present"
        ],
        "lane": [
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable"
        ]
      },
      "exclusion": "NotApplicable clean subscription has no dollar components; a dollar-evidence group has applicable settlement absent/rejected/conflicting/valid, regardless of launch lane.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-019",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none",
          "retained"
        ],
        "settlement": [
          "absent"
        ],
        "valuation": [
          "complete",
          "incomplete",
          "not_applicable",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "Absent settlement has no claimed settlement charge.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-020",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "complete",
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "charge",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-021",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "charge",
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-022",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "conflicting",
          "rejected"
        ],
        "valuation": [
          "not_applicable",
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "charge"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-023",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "usage_eligible"
        ],
        "valuation": [
          "complete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "exact",
        "components": [
          "charge",
          "usage_high"
        ],
        "billing": "established",
        "witnesses": [
          "EligibleUsageBound"
        ]
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-024",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "usage_eligible"
        ],
        "valuation": [
          "incomplete",
          "not_applicable",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "EligibleUsageBound requires complete established fully priced usage.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-025",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "conflicting",
          "error_status",
          "rejected",
          "reservation",
          "upstream_error",
          "usage_ineligible"
        ],
        "valuation": [
          "complete",
          "incomplete"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "usage_low",
        "high": "at_least",
        "components": [
          "reservation",
          "charge",
          "usage_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-026",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "conflicting",
          "error_status",
          "rejected",
          "reservation",
          "upstream_error",
          "usage_ineligible"
        ],
        "valuation": [
          "unpriceable"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "supported_low",
        "high": "at_least",
        "components": [
          "reservation",
          "charge",
          "priced_high"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-027",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "pi_api",
          "subscription"
        ],
        "reservation": [
          "retained"
        ],
        "settlement": [
          "conflicting",
          "error_status",
          "rejected",
          "reservation",
          "upstream_error",
          "usage_ineligible"
        ],
        "valuation": [
          "not_applicable",
          "not_reported"
        ]
      },
      "exclusion": null,
      "result": {
        "pool": "pi_api",
        "low": "zero",
        "high": "at_least",
        "components": [
          "reservation",
          "charge"
        ],
        "billing": "unresolved",
        "witnesses": []
      },
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-028",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "unbudgeted"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "complete",
          "incomplete",
          "not_applicable",
          "not_reported",
          "unpriceable"
        ]
      },
      "exclusion": "Unbudgeted requests have no reservation-backed settlement or settlement charge.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    },
    {
      "id": "X-BRANCH-029",
      "revision": 1,
      "guard": {
        "charge": [
          "present"
        ],
        "lane": [
          "subscription"
        ],
        "reservation": [
          "none"
        ],
        "settlement": [
          "not_applicable"
        ],
        "valuation": [
          "not_applicable"
        ]
      },
      "exclusion": "NotApplicable clean subscription has no dollar components; a dollar-evidence group has applicable settlement absent/rejected/conflicting/valid, regardless of launch lane.",
      "result": null,
      "refs": [
        "X-COMPONENTS",
        "SETTLEMENT-WITNESS",
        "Valuation",
        "Exposure"
      ]
    }
  ],
  "refs": [
    "C",
    "S",
    "V",
    "F",
    "X-COMPONENTS"
  ]
}
```

## Finalization bindings

```json
[
  {
    "temporal": "none",
    "witness": "eligible_usage",
    "amount_source": "none",
    "preserve": [
      "cause"
    ]
  },
  {
    "temporal": "none",
    "witness": "valid_keepall",
    "amount_source": "none",
    "preserve": [
      "cause"
    ]
  },
  {
    "temporal": "none",
    "witness": "other",
    "amount_source": "none",
    "preserve": [
      "cause"
    ]
  },
  {
    "temporal": "outstanding",
    "witness": "eligible_usage",
    "amount_source": "reservation_bound",
    "preserve": []
  },
  {
    "temporal": "outstanding",
    "witness": "valid_keepall",
    "amount_source": "reservation_bound",
    "preserve": []
  },
  {
    "temporal": "outstanding",
    "witness": "other",
    "amount_source": "reservation_bound",
    "preserve": []
  },
  {
    "temporal": "closed",
    "witness": "eligible_usage",
    "amount_source": "settlement_amount",
    "preserve": [
      "by"
    ]
  },
  {
    "temporal": "closed",
    "witness": "valid_keepall",
    "amount_source": "reservation_confirmed",
    "preserve": [
      "by"
    ]
  },
  {
    "temporal": "closed",
    "witness": "other",
    "amount_source": "reservation_bound",
    "preserve": [
      "by"
    ]
  }
]
```

Projection: `{"id":"PROJECT-Field-present","revision":1,"type_id":"Field","variant":"present","external_state":"present","wire_tag":"state","wire_value":"present","fields":{"value":{"type":"Json","source":"value"}},"refs":["Field"]}`

Projection: `{"id":"PROJECT-Field-null","revision":1,"type_id":"Field","variant":"null","external_state":"null","wire_tag":"state","wire_value":"null","fields":{},"refs":["Field"]}`

Projection: `{"id":"PROJECT-Field-absent","revision":1,"type_id":"Field","variant":"absent","external_state":"absent","wire_tag":"state","wire_value":"absent","fields":{},"refs":["Field"]}`

Projection: `{"id":"PROJECT-Field-malformed","revision":1,"type_id":"Field","variant":"malformed","external_state":"malformed","wire_tag":"state","wire_value":"malformed","fields":{"json_type":{"type":"String","source":"json_type"},"raw":{"type":"Json","source":"raw"}},"refs":["Field"]}`

Projection: `{"id":"PROJECT-Model-model","revision":1,"type_id":"Model","variant":"model","external_state":"model","wire_tag":"variant","wire_value":"model","fields":{"family":{"type":"ProviderFamily","source":"family"},"model_id":{"type":"String","source":"model_id"}},"refs":["Model"]}`

Projection: `{"id":"PROJECT-Bound-bound","revision":1,"type_id":"Bound","variant":"bound","external_state":"bound","wire_tag":"variant","wire_value":"bound","fields":{"basis":{"type":"String","source":"basis"},"evidence":{"type":"List<String>","source":"evidence"},"nano":{"type":"Nano","source":"nano"}},"refs":["Bound"]}`

Projection: `{"id":"PROJECT-High-exact","revision":1,"type_id":"High","variant":"exact","external_state":"exact","wire_tag":"kind","wire_value":"exact","fields":{"basis":{"type":"String","source":"bound.basis"},"nano":{"type":"Nano","source":"bound.nano"}},"refs":["High"]}`

Projection: `{"id":"PROJECT-High-at_least","revision":1,"type_id":"High","variant":"at_least","external_state":"at_least","wire_tag":"kind","wire_value":"at_least","fields":{"basis":{"type":"String","source":"bound.basis"},"nano":{"type":"Nano","source":"bound.nano"}},"refs":["High"]}`

Projection: `{"id":"PROJECT-Replacement-eligible","revision":1,"type_id":"Replacement","variant":"eligible","external_state":"eligible","wire_tag":"eligible","wire_value":true,"fields":{"reason":{"type":"String","source":"reason"}},"refs":["Replacement"]}`

Projection: `{"id":"PROJECT-Replacement-ineligible","revision":1,"type_id":"Replacement","variant":"ineligible","external_state":"ineligible","wire_tag":"eligible","wire_value":false,"fields":{"reason":{"type":"String","source":"reason"}},"refs":["Replacement"]}`

Projection: `{"id":"PROJECT-SettlementDecision-absent","revision":1,"type_id":"SettlementDecision","variant":"absent","external_state":"absent","wire_tag":"decision","wire_value":"absent","fields":{},"refs":["SettlementDecision"]}`

Projection: `{"id":"PROJECT-SettlementDecision-valid","revision":1,"type_id":"SettlementDecision","variant":"valid","external_state":"valid","wire_tag":"decision","wire_value":"valid","fields":{"basis":{"type":"String","source":"basis"},"charged":{"type":"Nano","source":"charged"},"settlement":{"type":"String","source":"settlement"}},"refs":["SettlementDecision"]}`

Projection: `{"id":"PROJECT-SettlementDecision-rejected","revision":1,"type_id":"SettlementDecision","variant":"rejected","external_state":"rejected","wire_tag":"decision","wire_value":"rejected","fields":{"claimed":{"type":"List<Nano>","source":"claimed"},"reasons":{"type":"List<RejectReason>","source":"reasons"}},"refs":["SettlementDecision"]}`

Projection: `{"id":"PROJECT-SettlementDecision-conflicting","revision":1,"type_id":"SettlementDecision","variant":"conflicting","external_state":"conflicting","wire_tag":"decision","wire_value":"conflicting","fields":{"claimed":{"type":"List<Nano>","source":"claimed"},"settlements":{"type":"List<String>","source":"settlements"}},"refs":["SettlementDecision"]}`

Projection: `{"id":"PROJECT-SettlementDecision-not_applicable","revision":1,"type_id":"SettlementDecision","variant":"not_applicable","external_state":"not_applicable","wire_tag":"decision","wire_value":"not_applicable","fields":{},"refs":["SettlementDecision"]}`

Projection: `{"id":"PROJECT-TemporalObligation-outstanding","revision":1,"type_id":"TemporalObligation","variant":"outstanding","external_state":"outstanding","wire_tag":"state","wire_value":"outstanding","fields":{},"refs":["TemporalObligation"]}`

Projection: `{"id":"PROJECT-TemporalObligation-closed","revision":1,"type_id":"TemporalObligation","variant":"closed","external_state":"closed","wire_tag":"state","wire_value":"closed","fields":{"by":{"type":"String","source":"by"}},"refs":["TemporalObligation"]}`

Projection: `{"id":"PROJECT-TemporalObligation-none","revision":1,"type_id":"TemporalObligation","variant":"none","external_state":"none","wire_tag":"state","wire_value":"none","fields":{"cause":{"type":"String","source":"cause"}},"refs":["TemporalObligation"]}`

Projection: `{"id":"PROJECT-Obligation-outstanding","revision":1,"type_id":"Obligation","variant":"outstanding","external_state":"outstanding","wire_tag":"state","wire_value":"outstanding","fields":{"amount_source":{"type":"AmountSource","source":"amount_source","allowed":["reservation_bound"]}},"refs":["Obligation"]}`

Projection: `{"id":"PROJECT-Obligation-closed","revision":1,"type_id":"Obligation","variant":"closed","external_state":"closed","wire_tag":"state","wire_value":"closed","fields":{"amount_source":{"type":"AmountSource","source":"amount_source","allowed":["settlement_amount","reservation_confirmed","reservation_bound"]},"by":{"type":"String","source":"by"}},"refs":["Obligation"]}`

Projection: `{"id":"PROJECT-Obligation-none","revision":1,"type_id":"Obligation","variant":"none","external_state":"none","wire_tag":"state","wire_value":"none","fields":{"amount_source":{"type":"AmountSource","source":"amount_source","allowed":["none"]},"cause":{"type":"NoneCause","source":"cause"}},"refs":["Obligation"]}`

Projection: `{"id":"PROJECT-Valuation-not_reported","revision":1,"type_id":"Valuation","variant":"not_reported","external_state":"not_reported","wire_tag":"state","wire_value":"not_reported","fields":{},"refs":["Valuation"]}`

Projection: `{"id":"PROJECT-Valuation-valued","revision":1,"type_id":"Valuation","variant":"valued","external_state":"valued","wire_tag":"state","wire_value":"valued","fields":{"complete":{"type":"Bool","source":"complete"},"derivations":{"type":"List<Json>","source":"splits"},"high":{"type":"Nano","source":"high.nano"},"identities":{"type":"List<String>","source":"identities"},"low":{"type":"Nano","source":"low.nano"}},"refs":["Valuation"]}`

Projection: `{"id":"PROJECT-Valuation-unpriceable","revision":1,"type_id":"Valuation","variant":"unpriceable","external_state":"unpriceable","wire_tag":"state","wire_value":"unpriceable","fields":{"priced_high":{"type":"Option<Nano>","source":"priced_high.nano"},"reasons":{"type":"List<UnpriceableReason>","source":"reasons"},"supported_low":{"type":"Option<Nano>","source":"supported_low.nano"}},"refs":["Valuation"]}`

Projection: `{"id":"PROJECT-Valuation-not_applicable","revision":1,"type_id":"Valuation","variant":"not_applicable","external_state":"not_applicable","wire_tag":"state","wire_value":"not_applicable","fields":{},"refs":["Valuation"]}`

Projection: `{"id":"PROJECT-Exposure-pi_api","revision":1,"type_id":"Exposure","variant":"pi_api","external_state":"pi_api","wire_tag":"pool","wire_value":"pi_api","fields":{"high":{"type":"High","source":"high"},"low":{"type":"Bound","source":"low"}},"refs":["Exposure"]}`

Projection: `{"id":"PROJECT-Exposure-codex","revision":1,"type_id":"Exposure","variant":"codex","external_state":"codex","wire_tag":"pool","wire_value":"codex","fields":{"billing":{"type":"String","source":"billing"}},"refs":["Exposure"]}`

Projection: `{"id":"PROJECT-Correlation-exact","revision":1,"type_id":"Correlation","variant":"exact","external_state":"exact","wire_tag":"variant","wire_value":"exact","fields":{"auxiliary":{"type":"List<Json>","source":"auxiliary"}},"refs":["Correlation"]}`

Projection: `{"id":"PROJECT-Correlation-ambiguous","revision":1,"type_id":"Correlation","variant":"ambiguous","external_state":"ambiguous","wire_tag":"variant","wire_value":"ambiguous","fields":{"auxiliary":{"type":"List<Json>","source":"auxiliary"},"candidates":{"type":"List<String>","source":"candidates"}},"refs":["Correlation"]}`

Projection: `{"id":"PROJECT-Correlation-unattributed","revision":1,"type_id":"Correlation","variant":"unattributed","external_state":"unattributed","wire_tag":"variant","wire_value":"unattributed","fields":{"auxiliary":{"type":"List<Json>","source":"auxiliary"}},"refs":["Correlation"]}`

Projection: `{"id":"PROJECT-GateVerification-unverified","revision":1,"type_id":"GateVerification","variant":"unverified","external_state":"unverified","wire_tag":"state","wire_value":"unverified","fields":{},"refs":["GateVerification"]}`

Projection: `{"id":"PROJECT-GateVerification-rejected","revision":1,"type_id":"GateVerification","variant":"rejected","external_state":"rejected","wire_tag":"state","wire_value":"rejected","fields":{"reasons":{"type":"List<String>","source":"reasons"}},"refs":["GateVerification"]}`

Projection: `{"id":"PROJECT-GateVerification-verified","revision":1,"type_id":"GateVerification","variant":"verified","external_state":"verified","wire_tag":"state","wire_value":"verified","fields":{"authority_id":{"type":"String","source":"authority_id"},"payload_digest":{"type":"String","source":"payload_digest"},"provenance":{"type":"String","source":"provenance"},"trust_policy_ref":{"type":"String","source":"trust_policy_ref"},"verification_evidence":{"type":"String","source":"verification_evidence"},"verifier_identity":{"type":"String","source":"verifier_identity"},"verifier_version":{"type":"String","source":"verifier_version"}},"refs":["GateVerification"]}`

Projection: `{"id":"PROJECT-ProviderFamily-openai","revision":1,"type_id":"ProviderFamily","variant":"openai","external_state":"openai","wire_tag":"variant","wire_value":"openai","fields":{},"refs":["ProviderFamily"]}`

Projection: `{"id":"PROJECT-ProviderFamily-anthropic","revision":1,"type_id":"ProviderFamily","variant":"anthropic","external_state":"anthropic","wire_tag":"variant","wire_value":"anthropic","fields":{},"refs":["ProviderFamily"]}`

Projection: `{"id":"PROJECT-AmountSource-settlement_amount","revision":1,"type_id":"AmountSource","variant":"settlement_amount","external_state":"settlement_amount","wire_tag":"variant","wire_value":"settlement_amount","fields":{},"refs":["AmountSource"]}`

Projection: `{"id":"PROJECT-AmountSource-reservation_confirmed","revision":1,"type_id":"AmountSource","variant":"reservation_confirmed","external_state":"reservation_confirmed","wire_tag":"variant","wire_value":"reservation_confirmed","fields":{},"refs":["AmountSource"]}`

Projection: `{"id":"PROJECT-AmountSource-reservation_bound","revision":1,"type_id":"AmountSource","variant":"reservation_bound","external_state":"reservation_bound","wire_tag":"variant","wire_value":"reservation_bound","fields":{},"refs":["AmountSource"]}`

Projection: `{"id":"PROJECT-AmountSource-none","revision":1,"type_id":"AmountSource","variant":"none","external_state":"none","wire_tag":"variant","wire_value":"none","fields":{},"refs":["AmountSource"]}`

Projection: `{"id":"PROJECT-NoneCause-unbudgeted","revision":1,"type_id":"NoneCause","variant":"unbudgeted","external_state":"unbudgeted","wire_tag":"variant","wire_value":"unbudgeted","fields":{},"refs":["NoneCause"]}`

Projection: `{"id":"PROJECT-NoneCause-no_reservation","revision":1,"type_id":"NoneCause","variant":"no_reservation","external_state":"no_reservation","wire_tag":"variant","wire_value":"no_reservation","fields":{},"refs":["NoneCause"]}`

Projection: `{"id":"PROJECT-NoneCause-orphan","revision":1,"type_id":"NoneCause","variant":"orphan","external_state":"orphan","wire_tag":"variant","wire_value":"orphan","fields":{},"refs":["NoneCause"]}`

Projection: `{"id":"PROJECT-NoneCause-subscription","revision":1,"type_id":"NoneCause","variant":"subscription","external_state":"subscription","wire_tag":"variant","wire_value":"subscription","fields":{},"refs":["NoneCause"]}`

Projection: `{"id":"PROJECT-Basis-usage","revision":1,"type_id":"Basis","variant":"usage","external_state":"usage","wire_tag":"variant","wire_value":"usage","fields":{},"refs":["Basis"]}`

Projection: `{"id":"PROJECT-Basis-reservation","revision":1,"type_id":"Basis","variant":"reservation","external_state":"reservation","wire_tag":"variant","wire_value":"reservation","fields":{},"refs":["Basis"]}`

Projection: `{"id":"PROJECT-Basis-upstream_error","revision":1,"type_id":"Basis","variant":"upstream_error","external_state":"upstream_error","wire_tag":"variant","wire_value":"upstream_error","fields":{},"refs":["Basis"]}`

Projection: `{"id":"PROJECT-Basis-error_status","revision":1,"type_id":"Basis","variant":"error_status","external_state":"error_status","wire_tag":"variant","wire_value":"error_status","fields":{},"refs":["Basis"]}`

Projection: `{"id":"PROJECT-RejectReason-ReservationConflict","revision":1,"type_id":"RejectReason","variant":"ReservationConflict","external_state":"ReservationConflict","wire_tag":"variant","wire_value":"ReservationConflict","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-NoReservation","revision":1,"type_id":"RejectReason","variant":"NoReservation","external_state":"NoReservation","wire_tag":"variant","wire_value":"NoReservation","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-Order","revision":1,"type_id":"RejectReason","variant":"Order","external_state":"Order","wire_tag":"variant","wire_value":"Order","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-AttemptNumber","revision":1,"type_id":"RejectReason","variant":"AttemptNumber","external_state":"AttemptNumber","wire_tag":"variant","wire_value":"AttemptNumber","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-AmountType","revision":1,"type_id":"RejectReason","variant":"AmountType","external_state":"AmountType","wire_tag":"variant","wire_value":"AmountType","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-ReservedMismatch","revision":1,"type_id":"RejectReason","variant":"ReservedMismatch","external_state":"ReservedMismatch","wire_tag":"variant","wire_value":"ReservedMismatch","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-BasisMissing","revision":1,"type_id":"RejectReason","variant":"BasisMissing","external_state":"BasisMissing","wire_tag":"variant","wire_value":"BasisMissing","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-BasisUnknown","revision":1,"type_id":"RejectReason","variant":"BasisUnknown","external_state":"BasisUnknown","wire_tag":"variant","wire_value":"BasisUnknown","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-KeepAll","revision":1,"type_id":"RejectReason","variant":"KeepAll","external_state":"KeepAll","wire_tag":"variant","wire_value":"KeepAll","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-StatusType","revision":1,"type_id":"RejectReason","variant":"StatusType","external_state":"StatusType","wire_tag":"variant","wire_value":"StatusType","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-ErrorCharge","revision":1,"type_id":"RejectReason","variant":"ErrorCharge","external_state":"ErrorCharge","wire_tag":"variant","wire_value":"ErrorCharge","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-UsageUnpriceable","revision":1,"type_id":"RejectReason","variant":"UsageUnpriceable","external_state":"UsageUnpriceable","wire_tag":"variant","wire_value":"UsageUnpriceable","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-ModelMismatch","revision":1,"type_id":"RejectReason","variant":"ModelMismatch","external_state":"ModelMismatch","wire_tag":"variant","wire_value":"ModelMismatch","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-ToleranceRange","revision":1,"type_id":"RejectReason","variant":"ToleranceRange","external_state":"ToleranceRange","wire_tag":"variant","wire_value":"ToleranceRange","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-RejectReason-Undercharge","revision":1,"type_id":"RejectReason","variant":"Undercharge","external_state":"Undercharge","wire_tag":"variant","wire_value":"Undercharge","fields":{},"refs":["RejectReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-IdentityWithoutRate","revision":1,"type_id":"UnpriceableReason","variant":"IdentityWithoutRate","external_state":"IdentityWithoutRate","wire_tag":"variant","wire_value":"IdentityWithoutRate","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-ModelUnresolved","revision":1,"type_id":"UnpriceableReason","variant":"ModelUnresolved","external_state":"ModelUnresolved","wire_tag":"variant","wire_value":"ModelUnresolved","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-TierUnestablished(Missing)","revision":1,"type_id":"UnpriceableReason","variant":"TierUnestablished(Missing)","external_state":"TierUnestablished(Missing)","wire_tag":"variant","wire_value":"TierUnestablished(Missing)","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-TierUnestablished(Unrecognised)","revision":1,"type_id":"UnpriceableReason","variant":"TierUnestablished(Unrecognised)","external_state":"TierUnestablished(Unrecognised)","wire_tag":"variant","wire_value":"TierUnestablished(Unrecognised)","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-NonStandardTier","revision":1,"type_id":"UnpriceableReason","variant":"NonStandardTier","external_state":"NonStandardTier","wire_tag":"variant","wire_value":"NonStandardTier","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-MalformedTier","revision":1,"type_id":"UnpriceableReason","variant":"MalformedTier","external_state":"MalformedTier","wire_tag":"variant","wire_value":"MalformedTier","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-MalformedCount","revision":1,"type_id":"UnpriceableReason","variant":"MalformedCount","external_state":"MalformedCount","wire_tag":"variant","wire_value":"MalformedCount","fields":{"field":{"type":"String","source":"field"}},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-MissingCount","revision":1,"type_id":"UnpriceableReason","variant":"MissingCount","external_state":"MissingCount","wire_tag":"variant","wire_value":"MissingCount","fields":{"field":{"type":"String","source":"field"}},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-UsageSchemaMismatch","revision":1,"type_id":"UnpriceableReason","variant":"UsageSchemaMismatch","external_state":"UsageSchemaMismatch","wire_tag":"variant","wire_value":"UsageSchemaMismatch","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-UsageShape","revision":1,"type_id":"UnpriceableReason","variant":"UsageShape","external_state":"UsageShape","wire_tag":"variant","wire_value":"UsageShape","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-UnknownUsageSchema","revision":1,"type_id":"UnpriceableReason","variant":"UnknownUsageSchema","external_state":"UnknownUsageSchema","wire_tag":"variant","wire_value":"UnknownUsageSchema","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UnpriceableReason-RateTableUnusable","revision":1,"type_id":"UnpriceableReason","variant":"RateTableUnusable","external_state":"RateTableUnusable","wire_tag":"variant","wire_value":"RateTableUnusable","fields":{},"refs":["UnpriceableReason"]}`

Projection: `{"id":"PROJECT-UsagePresence-not_reported","revision":1,"type_id":"UsagePresence","variant":"not_reported","external_state":"not_reported","wire_tag":"variant","wire_value":"not_reported","fields":{},"refs":["UsagePresence"]}`

Projection: `{"id":"PROJECT-UsagePresence-reported","revision":1,"type_id":"UsagePresence","variant":"reported","external_state":"reported","wire_tag":"variant","wire_value":"reported","fields":{},"refs":["UsagePresence"]}`

Projection: `{"id":"PROJECT-UsagePresence-unpriceable","revision":1,"type_id":"UsagePresence","variant":"unpriceable","external_state":"unpriceable","wire_tag":"variant","wire_value":"unpriceable","fields":{},"refs":["UsagePresence"]}`

Projection: `{"id":"PROJECT-FieldCount-present","revision":1,"type_id":"FieldCount","variant":"present","external_state":"present","wire_tag":"state","wire_value":"present","fields":{"value":{"type":"U64","source":"value"}},"refs":["FieldCount"]}`

Projection: `{"id":"PROJECT-FieldCount-null","revision":1,"type_id":"FieldCount","variant":"null","external_state":"null","wire_tag":"state","wire_value":"null","fields":{},"refs":["FieldCount"]}`

Projection: `{"id":"PROJECT-FieldCount-absent","revision":1,"type_id":"FieldCount","variant":"absent","external_state":"absent","wire_tag":"state","wire_value":"absent","fields":{},"refs":["FieldCount"]}`

Projection: `{"id":"PROJECT-FieldCount-malformed","revision":1,"type_id":"FieldCount","variant":"malformed","external_state":"malformed","wire_tag":"state","wire_value":"malformed","fields":{"json_type":{"type":"String","source":"json_type"},"raw":{"type":"Json","source":"raw"}},"refs":["FieldCount"]}`

Projection: `{"id":"PROJECT-Usage-openai","revision":1,"type_id":"Usage","variant":"openai","external_state":"openai","wire_tag":"variant","wire_value":"openai","fields":{"cache_write_tokens":{"type":"FieldCount","source":"cache_write_tokens"},"cached_tokens":{"type":"FieldCount","source":"cached_tokens"},"input_tokens":{"type":"FieldCount","source":"input_tokens"},"output_tokens":{"type":"FieldCount","source":"output_tokens"},"reasoning_tokens":{"type":"FieldCount","source":"reasoning_tokens"}},"refs":["Usage"]}`

Projection: `{"id":"PROJECT-Usage-anthropic","revision":1,"type_id":"Usage","variant":"anthropic","external_state":"anthropic","wire_tag":"variant","wire_value":"anthropic","fields":{"cache_creation_input_tokens":{"type":"FieldCount","source":"cache_creation_input_tokens"},"cache_read_input_tokens":{"type":"FieldCount","source":"cache_read_input_tokens"},"ephemeral_1h_input_tokens":{"type":"FieldCount","source":"ephemeral_1h_input_tokens"},"ephemeral_5m_input_tokens":{"type":"FieldCount","source":"ephemeral_5m_input_tokens"},"input_tokens":{"type":"FieldCount","source":"input_tokens"},"output_tokens":{"type":"FieldCount","source":"output_tokens"}},"refs":["Usage"]}`

Projection: `{"id":"PROJECT-SettlementForward-forward","revision":1,"type_id":"SettlementForward","variant":"forward","external_state":"forward","wire_tag":"variant","wire_value":"forward","fields":{"validity_ref":{"type":"String","source":"validity_ref"}},"refs":["SETTLEMENT-WITNESS"]}`

## Serialization interface

```json
{
  "accounting_evidence": {
    "attempt_reference": "accounting_result_ref",
    "kinds": [
      "accounting_input_proof",
      "accounting_result",
      "accounting_expected_assertion"
    ],
    "low_semantics": "scope supported credits; never sum local attempt lows",
    "rollup_reference": "supported_low_evidence_ref",
    "schema": "spec/accounting.schema.json"
  },
  "classification": {
    "additional_blocking": [
      "MalformedEvidence(field)"
    ],
    "existing_codes": "preserved from contract revision5"
  },
  "contract": "pi-accounting-contract/1.7-s1",
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
  "revision": 8,
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

Accepted PLAN43d0e7c8 and ROOT-ADOPTION-R6 bind revision8, retaining P1–P6/D3. Immutable reference bases, final-fact atomization, supported low credits and exact F coverage explicitly amend former policies. No reducer/solver or live qualification.
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
