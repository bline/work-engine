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

### ENCODING r1

## 13. Canonical output and determinism

- Outputs are canonical JSON: sorted keys, amounts as decimal strings of nano-USD integers, lists ordered by
  (source, line) or by `AttemptKey` order.
- Equal inputs give byte-identical output.
- Session and source presentation order MUST NOT change any output beyond declared append-order rules within a source.

### S2A-SCOPE r1

Select by raw API-launched source dollar reservation/settlement evidence including malformed rid/amount, orphan/no-reservation/conflicting groups and applicable RS1-3 rows. Dollar evidence with missing launch is selected negative unresolved lane/identity control. Never select from expected keys. Exclude genuinely unbudgeted/subscription-only sources. Profiles project separately after raw facts freeze. E1 unreadable remains explicit and never silently drops its case. S2a pure C/S/V/F/X/K only; E/L/T/R/gates/I/O excluded. Entry requires reviewed frozen contract, exact complete A/C verdicts no unresolved P1 and relevant B clearance, independent corpus qualification, production crate/location decision and authorized budget. None is claimed here.

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


## Finite declared rule partitions

Guards partition symbolic facts supplied by the declared stage interface. Coverage does not prove these predicates faithfully classify all raw evidence.

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

