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

