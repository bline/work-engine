**CHANGES_REQUIRED — bounded C1 review. Four P2 findings; no whole-C or system acceptance.**

Subject: frozen revision8, contract `1.7-s1`, manifest SHA256 `21df51b69391110beb6952c0eaebfb3c5bf99ac0ef19ce78a225fd24412afb36`. All paths below refer to that frozen subject. Root `01a1d6e2-32ba-7701-9115-740dcb41af11` retains collection, acceptance and escalation.

**C1-R8-01 — P2: C’s exclusive validity ownership is enforced against S but not V.**

- Location: [lib.rs:597](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/src/lib.rs:597); `spec/ck.json`, rule `V-PRESENCE-1.outputs`.
- Executed counterexample: replace that V rule’s output with `SettlementDecision.valid`, using its exact declared fields `basis`, `charged`, `settlement`, and empty bindings. Leave the guard, stage and remaining declarations unchanged.
- **Actual:** the bound binary’s `canonical SUBJECT /dev/stdin` exits **0**, reporting canonical schema and structural agreement.
- **Expected:** reject a non-C constructor of C-owned settlement validity. The checker specifically prohibits S from constructing this decision, but accepts the same competing authority in V.
- Owner/invariant: PLAN §7, C-validity’s sole ownership. This is declaration validation, within D3; no raw reducer is needed to detect it.
- Origin: exact introduction **unknown**; recurrence of the broader semantic-owner enforcement failure class underlying A-05. No current financial miscalculation is asserted from this mutant.

**C1-R8-02 — P2: F’s recorded coverage need not identify its release link.**

- Location: [accounting.rs:396](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/src/accounting.rs:396); `oracle/r8-controls/VC07-conditional.bundle.json`, `$.documents[1].coverage_decisions[0].definite_link_refs`.
- Executed counterexample: change that array to `[]`, retaining the accepted coverage, retired anchor and resolved request unit.
- **Actual:** baseline and mutant both exit **0** through the bound binary’s `evidence` command.
- **Expected:** coverage retiring the anchor must identify the applicable affirmative link to its covered request.
- The checker validates listed references, then independently searches **all** result links for any endpoint matching the anchor. It does not require that witness to belong to this coverage’s reference set.
- Owner/invariant: exact F coverage and C→F witness application. This independently corroborates part of **A1-R8-01**; **C2-R8-03** establishes the related primary-validity misapplication. Deduplicate these during collection.
- Origin: exact introduction **unknown**, present in R8 supplied-record checking. This probe establishes a coherence failure, not a whole financial-gate bypass.

**C1-R8-03 — P2: Published integer domains disagree with Rust; one schema-valid input panics.**

- Locations: `ck.schema.json`, `$defs.TypeDef.properties.revision`; [accounting.schema.json:863](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/spec/accounting.schema.json:863); [accounting.rs:94](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/src/accounting.rs:94).
- Executed counterexamples:
  - Set `ck.types[0].revision=4294967296`. Python Draft 2020-12 accepts it; Rust rejects it because its representation requires `u32`.
  - Set VC07 input origin `original_position=18446744073709551616`. Python Draft 2020-12 accepts the document; Rust `evidence` exits **101**, panicking at `as_u64().unwrap()`.
- **Expected:** published domains and Rust representations agree; unsupported values receive a controlled validation error.
- Owner/invariant: PLAN §7/S1 schema agreement and the standalone checker’s input contract.
- Origin: **unknown**. The former StageId/BoundaryId defect is repaired, but broader A-09 agreement cannot be closed.

**C1-R8-04 — P2: VC08’s admission grouping lacks the namespace guarantee it consumes.**

- Locations: [VC08 provenance](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/oracle/r8-completion-B/VC08-subscription-measured-request.provenance.json), `$.initial_scope_contracts[0].reference_namespace`, `$.grant_payloads[0].guarantee`; corresponding bundle input contracts and result origins.
- Independently read raw facts: the two rows contain `subscription_proxy_admitted.rid=1` and `subscription_proxy.rid=1`.
- **Actual:** the contract declares `dollar_ticket`; its payload guarantees interpretation only of a “usable dollar rid.” Both origins nevertheless share `base:adm:S1/turn-1:rid=1`.
- **Expected:** the separate admission namespace and applicable guarantee required by PLAN §3.
- Owner/invariant: C initial-reference admission; distinct guarantees cannot substitute for one another.
- This corroborates **A2b-R8-01 for VC08**. I did not independently reverify its U06/V03 extensions or demonstrate a monetary change.
- Origin: exact introduction **unknown**, present in completion-B.

**Prior finding dispositions**

| Prior | C1 disposition |
|---|---|
| A-03 | The previously excluded PiApi/not-applicable/not-reported tuple now has `X-BRANCH-005-anchor-3`, yielding unresolved `AtLeast0`. The A3 finite record agrees. General raw applicability remains unestablished. |
| A-04 | Cited external-name defects are repaired: `pool`, flattened `kind/nano/basis`, valuation decimal strings and `derivations`, obligation fields, and qualified U08 reason. All **186 current attempt wire records** passed both validators. This is bounded wire agreement, not arbitrary constructor or financial conformance. |
| A-05 | S-specific repair supported: all S decision rows forward C’s reference; tested constructor/reference mutants reject. Broader exclusive ownership remains open through C1-R8-01. |
| A-06 | Registered dependency repair supported. All ten protected-node `impact` probes reached the required consumers, including R, Python, independent assertions, schema/contract views and completion consumers. All 19 acknowledgements remain unestablished. Digest-mutation and stale-ack enforcement were inspected in source and recorded tests; I did not freshly replay their write-requiring suites. |
| A-09 | Canonical specification passes standard Draft 2020-12 and Rust; both reject StageId/BoundaryId swaps. Broader agreement remains open through C1-R8-03. |

A/C2 retain financial truth, complete consumer census and authorship-provenance obligations. I do not close A-07/A-08. Their relevant source still skips non-string response identities and computes comparison applicability from `nums.len()==1`. Consequently, [README.md:7](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/oracle/README.md:7) overstates those supported axes. Its broader disclaimers—and R8-COMPLETION’s finite-only qualification—correctly leave arbitrary conformance, executable confluence, live authority and S2a unestablished.

**Coverage, execution and limitations**

Read required policies, adoption and architecture evidence; previous findings and relevant guard history; applicable doctrine/rationale; all three schemas and interface; canonical section outlines and relevant C/S/V/F/X, projection and dependency declarations; relevant library/decision code, complete accounting/main consumers, structural tests and Python dependency test; current A1/A2a/A2b/C2 reports and postgate documentation.

Executed read-only Python/hash inspections and the pinned binary’s `check --generated`, `canonical`, `wire`, `evidence` and `impact` commands. Mutants existed only in memory and reached Rust through `/dev/stdin`. Additional controls rejected old wire tags/nesting/splits, illegal obligation amount sources, missing closed `by`, unqualified U08 reasons, dangling input proofs and result flags inserted into input proofs.

Verified:

- **941** frozen entries, **1,285** capsule-bound files, six dispatch projections and the supplied archive hash.
- Binary SHA256 `66ec52f8d51bf591a29b529948fd5e9957929234701ba9867216c1c845aeefd2` and validator SHA256 `ab5f78d47d6105791a8ca14cf0c0799015aca67dd7935a48d18756735417429a`.
- All six join-2 and five regression custody-record/stream hashes. Joined records report 6/6; regression output reports 35 Rust and 13 Python tests.
- Of 479 unique joined inputs, **474 match frozen/evidence equivalents**, the **three disclosed metadata files differ**, and the external control binding and binary match.

Regression records contain no input-digest inventory; their historical passes are not independently reconstructed exact-source executions. I did not rebuild, rerun sabotage, replay every historical revision, exhaustively resolve external payload references, or audit all financial/attribution expectations. No byte-identical whole-subject execution claim is made. B predecessor accounting continuity remains unestablished.

Graph checks returned `outside_project/unavailable`; conclusions use exact frozen source, without graph-completeness claims.

**Process and identity**

Acknowledged root’s binding: SID `01a1d890-d020-74a0-a7d5-ed4a2902dd2f`, rollout `/home/bline/.codex/sessions/2026/11/13/rollout-2026-11-13T22-38-51-01a1d890-d020-74a0-a7d5-ed4a2902dd2f.jsonl`. Root records **Astra/high/default**, read-only/never; the pinned context checker independently reported Astra/high, latest observed input **145,611**, with no compaction. Backend fulfillment, billing and actual proxy spend remain unknown.

No files, commits, settings, delegates or provider launches were created; all launched commands terminated. No context reset occurred. The existing **DESIGN_REVIEW_REQUIRED** guard remains with root; this report authorizes no repairs.
