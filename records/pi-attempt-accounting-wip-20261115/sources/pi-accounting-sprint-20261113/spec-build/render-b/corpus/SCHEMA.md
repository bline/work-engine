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

### ENCODING r1

## 13. Canonical output and determinism

- Outputs are canonical JSON: sorted keys, amounts as decimal strings of nano-USD integers, lists ordered by
  (source, line) or by `AttemptKey` order.
- Equal inputs give byte-identical output.
- Session and source presentation order MUST NOT change any output beyond declared append-order rules within a source.

### S2A-SCOPE r1

Select by raw API-launched source dollar reservation/settlement evidence including malformed rid/amount, orphan/no-reservation/conflicting groups and applicable RS1-3 rows. Dollar evidence with missing launch is selected negative unresolved lane/identity control. Never select from expected keys. Exclude genuinely unbudgeted/subscription-only sources. Profiles project separately after raw facts freeze. E1 unreadable remains explicit and never silently drops its case. S2a pure C/S/V/F/X/K only; E/L/T/R/gates/I/O excluded. Entry requires reviewed frozen contract, exact complete A/C verdicts no unresolved P1 and relevant B clearance, independent corpus qualification, production crate/location decision and authorized budget. None is claimed here.

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


## Finite declared rule partitions

Guards partition symbolic facts supplied by the declared stage interface. Coverage does not prove these predicates faithfully classify all raw evidence.

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

