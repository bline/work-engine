# Corpus schema `pi-accounting-corpus/1`

One JSON file per case: `cases/<case_id>.json`, with canonical key order. The executable definition is
`validator/validate_corpus.py`; this page describes it.

**Money:** nano-USD integer strings. **Times:** RFC 3339 UTC. **Evidence references:** `<alias>/turn-<n>:<line>`.
**Attempt keys:** `res:<alias>/turn-<n>:rid=<r>`, `adm:<alias>/turn-<n>:rid=<r>`, `row:<alias>/turn-<n>:line=<l>`.

```text
{
  schema: "pi-accounting-corpus/1", contract: "pi-accounting-contract/1.4-s1",      // revision 5 (P1.33); 1.3-s1 P1.32, 1.2-s1 P1.30, 1.1-s1 P1.29, 1.0-s1 P1.28
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
                 replacement: {eligible, reason: UsageBound|NotUsageSettlement|UsageIncomplete|UsageUnpriceable|NoReservation},   // 2.1 (R5-01)
                 evidence: [refs], settlement: {decision, basis?, reasons, reserved?, charged?},
                 obligation: {state, amount_source: settlement_amount|reservation_confirmed|reservation_bound|valuation_or_unresolved},   // 6 (R5-08)
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
