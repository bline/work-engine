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

