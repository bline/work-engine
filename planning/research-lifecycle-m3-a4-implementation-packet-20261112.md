# M3 — bounded Rust current-use A4 implementation packet

Date: 2026-11-12. Revision **2**, retained F1 remediation. **Planning candidate; no implementation authorization.** Baseline: `b0cfd408a7f70ee0d21b232377e84054b0af29d4`. Root selected the proposed prerequisite split: next **M3a bounded reader/host qualification**, then **M3b projection/trust/codec**. P1–P4 remain proposed pending their owners' acceptance. No research publication, H2 execution, task admission, distribution or spending is authorized.

## Authority and observed boundaries

The [M0 approval](../records/research-lifecycle-m0-approval-20261111.md) accepts the [M0 contract](research-lifecycle-m0-contract-and-first-use-20261111.md), including LC-1–7 and scoped A4 dispositions; its earlier pending labels are historical. [M1](../records/research-lifecycle-m1-result-20261111.md) supplies structural join/reconstruction proofs. [M2 authorization](../records/research-lifecycle-m2-integration-authorization-20261112.md) and [integration result](../records/research-lifecycle-m2-integration-result-20261112.md) establish local integration of accepted `c3d82322`; Q-B1 distribution clearance remains held. No whole-suite replay is required for this prose packet.

Observed APIs under `rust/crates/research-register/src/`: `PinnedSnapshot::build` (`snapshot.rs:56`), `validate_report` and private `ValidRegister` (`validate.rs:26,82`), `validate_use_bundle` (`use_bundle.rs:396`) and private `ValidUseBundle` (`:245`). The latter exposes original manifest/record bytes, pins, source files and retained Git bytes (`:254–273`). `ValidRegister::records()` exposes typed register records; snapshot entries supply exact study/source copies. A4 must consume these owners, not validate by inference.

`anchors(valid, snapshot)` (`anchor.rs:139`) checks commit/path/tree equality, then sorts and classifies **every registered decision**, accumulating before returning. The actual CLI caller is `main.rs:71–174`; a graph match in claims-host reassessment was unrelated. `GitObjects::read` (`objects.rs:170–183`) obtains full object bytes before returning; snapshot collection (`snapshot.rs:179–237`) eagerly reads register blobs. After-read size checks cannot establish preallocation safety. M2's [private allowances](../records/research-lifecycle-m2-private-qualification-result-20261111.md) qualified an external harness, not cancellation in this synchronous API.

## Proposed interface, placement and trust

**P1, root architectural choice:** add domain modules `card.rs`, `card_wire.rs`, `card_admission.rs` and `card_host.rs` to this crate, with `src/bin/work-engine-research-card.rs` assembling a small Linux parent/private-worker executable. Keep existing register/anchor APIs and proof meanings. A separate public projection crate adds an unjustified consumer boundary today; embedding computation in M4's task host couples projection to unimplemented adoption; a new organization/runtime compiler adds no required outcome.

The future M3b worker builds one bounded snapshot, validates the register and joined bundle, admits the complete read batch, calls anchors **in process against that same snapshot**, and projects:

```rust
project_current(&PinnedSnapshot, &ValidRegister, &ValidUseBundle,
                &CurrentCardRequest, &DisclosureScope, &AdmittedUse,
                &AdmittedReadBatch) -> Result<CardBytes, CardFailure>
```

The M3b request binds exact decision ID/revision and bundle request-byte revision. No current substitution. Structural validity, scientific qualification, provenance and authenticated authorization remain separate fields/checks. A5 receives a projection, never an execution permit. M3a returns only a bounded structural batch report; synthetic qualification cannot authenticate a research use.

**P2, owner prerequisite and proposed narrow trust route:** the user/delegated research acceptance owner and applicability owner install a per-use admission map through trusted host startup, outside request/repository/tool-writable inputs. Its operator-approved interpretation binds exact saved approval/delegation event bytes/digests, authority and delegation scope, study/subject revision, assessment and qualification revisions/purpose/disposition, reviewer/exposure/capability evidence, applicability revision/relation/bounds, decision revision and request digest. The installed trust profile fixes authorized issuers and disclosure scope. Requests cannot supply the map, verifier, trust root or replacement key.

A private host verifier compares those exact bindings to retained evidence and constructs non-deserializable `AdmittedUse`; a DTO or M1 `owner`, `disposition` or `authorization_evidence` label cannot construct it. Qualification acceptance and applicability authorization need their respective owner evidence. No live map, usable production verifier or H2 new-use approval is established by today's records. Missing/protection-unestablished/invalid admission yields `AuthorityUnavailable` or `AuthorityMismatch`, no qualified-use card. Tests use explicitly synthetic installed authority. The implementation may qualify this adapter and refusal path; production use remains a separate owner installation/acceptance prerequisite. If that trust route is unacceptable, split after core/host qualification and resolve the owning admission interface before production A4; do not invent authority.

## Preserved projection semantics and proposed wire

D-6: invalid register produces **no card**, including no degraded/partial fallback; unreadable source and invalid/unavailable companion remain distinct. The requested current decision's anchor is required. A null/unestablished/missing required result yields a typed `AnchorQualificationGap` and no affected card; it never means absent. A different decision's null does not establish this decision's gap. Complete-batch read/resource failure still prevents all output. Established modified/ambiguous/absent results remain their actual classes with warnings, not scientific rejection or permission.

Current selection preserves decision-listed findings and exact current legacy applicability recorded as authorized. **Proposed extension:** explicitly admitted companion subjects also enter selection, with that distinct membership provenance. Corrections alone traverse to a fixed point, preserving both affected findings; material objections remain visible without expanding selection. Legacy status is attributed register data, never authenticated new-use permission. Decision-listed membership carries exactly “listed on the decision record; not an applicability judgment”. Preserve every competing assessment and `unknown` versus `not_assessed`; no timestamp winner, numeric confidence or support inferred from `not_established`.

Partition same-decision disclosures by revision before status. D-2/U2 pending notice is exactly: “Proposed relationship only. Not owner-authorized. Excluded from delivered evidence and applicability judgments.” Requested-revision declined notice is exactly: “Applicability relationship declined. Not authorized for the requested decision revision. This does not determine the underlying finding's validity or applicability elsewhere.” Other-revision notice is “Recorded for decision revision R (status: X). This relationship is not carried forward to the requested revision.” Substitute each row's actual revision/status for R/X, including declined rows (D-8b).

U1 cross-decision disclosure is metadata only, for relationships to selected delivered findings, within granted scope: ID/type, actual decision/revision, finding ID, relation, recorded authorization status and owner state. Preserve declined status without applying the requested-decision declined notice (D-8c). State exactly: “These records belong to other decisions. Their relationship to another decision does not establish applicability to the requested decision.” Do not deliver those decisions' finding content, traverse their relationships, count them as support/coverage deficiency, or expose siblings (U3). Deliver the selected decision's historical marker verbatim; infer no missing successor/document from reason text (D-8e). B4 remains protected by refusing historical requests; D-8a/d/g/h/i historical behavior is excluded.

**P3, proposed M3b fixture contract:** fixed schema `research_a4_current_v1`, codec `research_a4_json_v1`; no legacy byte-conformance claim. Below, `S` is Unicode-scalar text, `U` integer 0–2^64−1, `Hn` lowercase n-hex, `T[]` an array. Every named field is mandatory; only shown unions permit null; duplicates/unknown fields fail. `Pin={commit:H40,path:S,blob:H40,sha256:H64}`; `Obj={oid:H40,kind:commit|tree|blob|tag,sha256:H64,size:U}`. SHA-256/size cover payload bytes. Word alternatives below are literal strings.

```text
Envelope={schema:S,codec:S,outcome:S,request:Q|null,card:C|null,detail:S|null}
Q={decision_id:S,decision_revision:S,request_sha256:H64}
C={source:{commit:H40,register_path:S,register_tree:H40,inputs:Pin[]},
 bundle:Pin,admission:{profile_sha256:H64,receipt_pins:Pin[]},producer_sha256:H64,
 records:{revision:H64,kind:subject|evidence|assessment|qualification|request|applicability,utf8:S}[],
 assessments:H64[],qualifications:H64[],applicability:H64[],
 anchor:{report_sha256:H64,row_utf8:S,objects:Obj[]},delivered:E[],
 not_delivered:{pending:D[],declined:D[],other_revision:D[],cross_decision:D[]},
 cross_statement:S,decision_finding_statement:S,historical_utf8:S|null,
 claims:{scope:delivered_findings,row_count:U,status:empty|unavailable,
         materialized_rows:S[],exact_copies:[]|null},
 coverage:{selection:current_links_and_corrections,truncated:false,
           read_scope_sha256:H64,disclosure_scope_sha256:H64,limitations:S[]}}
E={finding:S,study:Pin,finding_utf8:S,
 membership:{kind:decision_finding|recorded_authorized_applicability|admitted_companion|correction,via:S}[],
 events:{id:S,utf8:S}[],judgments:S[]}
D={id:S,type:applicability,decision_id:S,decision_revision:S,finding:S,relation:S,
 authorization_status:pending|authorized|declined,owner_state:{owner:S,status:S},notice:S|null}
```

`records.utf8` retains exact M1 stored bytes, exposing its eight separate facets and assessment links. Legacy finding/event/judgment/materialized/historical and anchor-row JSON copies use this codec inside `utf8` strings; preserve source array order/number lexemes. Historical is null only when absent. D notices follow the exact rules above; cross rows use null and `cross_statement` is U1's exact sentence. `owner_state` copies authorization owner/status. `decision_finding_statement` is the exact qualifier above.

Outer keys sort lexicographically by Unicode scalar value. Encode compact UTF-8 plus one LF, no normalization/BOM. Escape only quote, backslash and U+0000–001F: use `\b\f\n\r\t` for those five controls, otherwise lowercase `\u00xx`; slash, non-ASCII and U+2028/U+2029 stay literal. Reject unpaired surrogates/nonfinite numbers. Outer counts use decimal U without leading zeros; embedded legacy numbers retain exact valid JSON spelling, including `-0`/exponents.

Order **each** array: all Pin arrays by `(commit,path,blob,sha256)`; objects by `(kind,oid)`; records and three revision arrays by revision; delivered by finding; membership by `(kind,via)`; events and each D group by ID; judgments and materialized rows by source file/row order; limitations by exact text. Embedded copied arrays retain source order. Empty arrays remain arrays. No runtime timestamp/absolute host path/PID; card SHA-256 remains external.

Claims scope is delivered findings only: zero matching materialized rows means `empty`, row_count 0, rows `[]`, exact_copies `[]`; one or more means `unavailable`, actual count/rows and exact_copies null because M3 never consults native claims. This says nothing about claims elsewhere.

Only outcome `card` carries C and null detail. Non-card outcomes carry null card and detail equal to the outcome code; Q is null only for an unparseable request. Only these codes are permitted: missing decision→`not_registered`; U02/U03→`revision_unknown`; known prior revision→`historical_unsupported`; invalid register/bundle→`invalid_register`/`invalid_bundle`; unreadable inputs→`unreadable`; absent/mismatched authority→`authority_unavailable`/`authority_mismatch`; required null→`anchor_gap`; insufficient read/disclosure scope→`scope_unavailable`/`coverage_unavailable`; exceeded envelope→`resource_exceeded`; unsupported platform/missing enforcement→`controls_unavailable`; unknown codec→`unsupported_version`; malformed request→`bad_request`; cancellation before delivery→`cancelled`, uncertain completed delivery→`delivery_unknown`. Further diagnostics remain outside these deterministic bytes.

Complete proposed expected-byte vectors and their digests are bound by [wire-vectors-v2/manifest.json](/home/bline/cbm-work/research-lifecycle-m3-planning-20261112/planner/wire-vectors-v2/manifest.json): success/empty, success/unavailable and revision-unknown refusal. Manifest SHA-256: `c7698bd3a71db01752ed02b25e0b61ac24d6180510d05f0ef5a8ac66de59aa4f`. Vectors include escaping, legacy exponent/negative-zero spelling and judgment IDs opposing source order. These are synthetic codec examples, not valid register/authority evidence; M3b must additionally derive semantic goldens.

Retain original M1 bytes, verified anchor dependency objects, admission evidence, producer identity and card bytes for reconstruction. Missing copies make reconstruction unavailable, never trigger a newer fetch. No truncation or omitted material objection; unavailable mandatory disclosure refuses. `claim-evidence::RepositoryReader::resolve` requires separate repository/decision-scoped startup (`repository.rs:20–51,80–106`), so it supplies no implicit research-copy permission.

## Actual resource route and engineering boundary

**P4, owner-controlled initial profile, not inherited policy:** propose at most 16 registered decisions; 1 MiB/10,000 lines per text blob; 8 MiB aggregate distinct read-object bytes; 60 seconds each CPU and wall; 1 GiB worker memory; 8 MiB combined framed result/reconstruction/diagnostic output. These are starting qualification bounds, not worst-case guarantees or scientific thresholds. Smaller hostile test profiles demonstrate boundaries.

The Rust parent bounds control input, pins executable/configuration, installs worker controls **before object-store opening**, and owns wait/kill/drain. Proposed supported route requires Linux delegated cgroup-v2 memory containment plus process CPU limits and an independent wall deadline; unsupported/unavailable enforcement refuses startup. CPU enforcement granularity and address-space versus resident-memory limits must be reported accurately; M2 proved an address-space allowance, not this proposed memory profile.

Add a budgeted `GitObjects` construction/read path in `objects.rs`, shared by snapshot, validation, bundle reads and anchors. Trusted read scope must cover the mandatory whole-register/anchor dependency closure; check each access or return `ScopeUnavailable`, never silently broaden reads. Reject declared oversized objects before payload allocation using qualified bounded metadata/stream reads, check actual verified bytes/lines, and charge every newly discovered object/read dependency. Register/companion preflight precedes their retained-copy allocation; anchor admission inventories **all** decisions and both revision/target blobs, not only the requested one. Bound metadata and ancestry traversal too. OS containment covers decoder/allocator work and unforeseen expansion; post-read checks alone cannot satisfy preallocation admission. If pinned gix cannot support the required read boundary, stop and propose the exact reader change or a separate prerequisite slice, rather than claiming this gate passed.

The worker stages complete output; the parent consumes no card until exit, framing, size, pins and outcome checks pass. Timeout, cancellation, OOM, cap-reached/truncated frame, crash or unavailable input discards staged card bytes. Cancel before delivery prevents publication; a lost reply after a completed delivery leaves delivery unknown, never proof of absence or permission to retry an effect. There are no research/task mutation effects. M4 owns actual delivery/adoption receipts and execution settlement.

**Selected proposed M3a scope:** budgeted reader in `objects.rs`, `bounded_read.rs`, `read_host.rs`, `src/bin/work-engine-research-read.rs`, `lib.rs`, necessary Cargo/lock declarations, `tests/{bounded_read,read_host}.rs` and `docs/bounded-read-profile-v1.md`. Adjust snapshot/bundle/anchor plumbing only as enforcement requires. Its synthetic vertical control builds snapshot→register validation→M1 joined bundle→same-worker in-process anchors. Exit proves controls precede repository open, bounded allocation/dependency discovery, and complete-result-or-refusal under actual CPU/timeout/memory/output/cancellation/unsupported-platform failures. Isolate every negative against a successful control. Preserve M1/R2 proof/read/ambiguity/oracle boundaries. **M3b** later owns full card selection, trust admission, this codec, goldens and reconstruction; M4 owns adoption/execution.

## Acceptance and next allowance

The saved [A4 specification §9](/home/bline/cbm-work/a4-spec-finalization-20261110/a4-v1-specification-v1.md) inventories 17 frozen cases at `8dbe4275`. M3b re-derives C01–C11/X01, particularly C01/C10 cross-decision, C02 successor absence, C04/C07/X01 other-revision, C09 correction and C11 objection. P01/P02 refuse historical requests; U01 is unregistered, U02/U03 both unknown-revision refusals. Preserve old JSON as historical regression evidence. M3a uses `tests/use_bundle.rs`'s synthetic pattern; later H2 fixtures use frozen decision/applicability/event/judgment rows and `codex-compaction-points.json`/`handoff-threshold-replay-m22.json`, retaining remaining manifest inputs needed for register validity.

| Acceptance subject | Valid control and required negative consequence |
| --- | --- |
| M3a composition | Same-pin synthetic snapshot/validation/bundle/anchor batch succeeds; mismatched pins or missing dependencies produce no successful batch. M3b additionally checks admitted use. |
| M3b authority | Installed synthetic exact approvals succeed; missing/forged label, request verifier/map, wrong issuer/delegation/purpose/scope fails. No live-authority claim. |
| Register/anchors | M3a preserves invalid/unreadable, packed-object loss and post-validation ambiguity failures. M3b adds required-null/missing-anchor no-card gap. |
| M3b disclosure | Current succeeds; P01/P02 historical refusal, U01 unregistered, U02/U03 same unknown-revision refusal; exact notices, U1/U3 and D-8b/c/e/f hold. |
| M3b evidence | Correction/objection closure and competing/unknown facets remain visible; inaccessible mandatory material refuses, cross-decision content never leaks. |
| M3b bytes | Exact goldens/reconstruction; missing copy, numeric loss, unknown codec or pin substitution cannot pass. |
| M3a actual host | Full 11-decision/near-boundary controls succeed; actual CPU/wall/memory/output failure, preallocation overrun, late discovery, cancellation or unsupported platform yields no partial successful batch. |
| Both slices | Relevant M1/R2 regression gates pass on changed bytes; no network, register mutation, task grant/adoption or H2 run. |

Propose a **separately approved M3a $10 proxy cap/$7 warning** for one Sol 6.1/medium builder, root qualification and separate fresh read-only Astra/xhigh/default review, retaining ordinary remediation. M1 cost about $6.61 and M2 $7.19; these observations support a bounded allowance, not a completion forecast for new reader/host work. Reassess at warning; stop at cap, unavailable enforcement, unsafe allocation or larger architectural replacement. P2/P3 remain deferred to M3b; do not widen into trust, projection, H2 or M4. No automatic spending/dispatch grant follows.

Evidence: repo-search Verify, graph `home-bline-code-work-engine`, lookup generation `2026-11-12T07:16:13Z`, store `u3cd366b8475f1bc9g2`; final 20-path coverage generation `2026-11-12T07:40:21Z`. Relevant narrowed query pages completed. Source content verified, with unresolved calls checked against the cited exact methods; no complete call-graph claim. Coverage/source identities and host-local authority/specification hashes are frozen in `/home/bline/cbm-work/research-lifecycle-m3-planning-20261112/planner/`. Tests and runtime readiness are future acceptance work. M4 still owns LC-4 preparation/invalidation/adoption and the selected task consumer; its interface consumes the immutable card/basis and independently established grants.
