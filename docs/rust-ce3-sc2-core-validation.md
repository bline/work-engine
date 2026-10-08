# CE3 and SC2 core validation

Completed 2026-10-08 against shared commit
`41992a22d86b041d2db357d16345c477804af63a`. The user explicitly authorized CE3 and
SC2 core implementation in parallel using `gpt-6-sol`. Two Sol/high workers owned
the separate crates. This result is uncommitted; no provider execution, live-root
migration, activation or publication was performed.

## Implemented scope

CE3 adds production-path observation custody, deterministic establishment,
registered production grants, exact checked readback and independently durable
stage recovery to `claim-evidence`. Both consumption boundaries and all three
claim outcomes remain distinct. New private roots use schema 2; historical roots
are not silently upgraded. Controlled immutable evidence owners qualify the
local domain behavior, not actual runtime attestation.

SC2 core adds schema-2 private campaign roots, synchronized owner/lease fencing,
durable preparation and dispatch, one-use dispatch permits, exact recovery and
private outcome, retry and evaluation transitions. Dispatch and retry state is
scoped by obligation and operation. Owner-dependent completion entry points
remain closed pending the real claims/episode join. Retry predicates are tested
locally; they do not supply HP3's actual execution custody.

The accepted scope is described in the [CE3 plan](rust-claims-ce3-implementation-plan.md),
[SC2 plan](rust-campaign-sc2-implementation-plan.md) and
[integration plan](rust-ce3-sc2-integration-plan.md).

## Exact subjects and verification

Final candidate 3 has 26 changed product/test/schema paths: 17 claims paths and
9 campaign paths. All final file hashes match the worker manifests and the
parent's combined gate subject. The shared Cargo lock hash also matches the gate
receipt. The branch head and complete staging-area entry listing are unchanged
from this implementation's start; other sessions' working-tree work is preserved.

| Evidence | SHA-256 |
| --- | --- |
| CE3 candidate-3 completion receipt | `618e511503523f4c6f8dd2bc06f3dec2eced4859ff6693ac453fd755b2669ae7` |
| SC2 candidate-3 source manifest | `e94d24f95f55ddbb976453e8b271f431cf893e0f2fce7501202a7502820188d4` |
| SC2 candidate-3 completion receipt | `427a389e7b13af5a92c5535211d78ef9dcb01e2167f53564529975586f1aad5f` |
| Retained source review | `d5b44bc383899dc2804f7656c89ca3e53304d5057d9da714170b51331ec9a897` |

Evidence is retained under
`/home/bline/.local/state/work-engine/rust-ce3-sc2-implementation-20261008/`.
The parent `final-receipt.json` binds the manifests, review, final combined logs,
source aggregate and this validation document. Earlier candidates and review
history remain separate.

The final parent run passed:

- Both packages' locked all-feature tests, serial test harness: **73 executable
  test cases and 5 compile-fail doctests**, no failures. Totals include fault-test
  helpers; this is not a count of independent end-to-end scenarios.
- Both packages' all-target/all-feature Clippy with warnings denied.
- Both packages' formatting checks, with no source changes during the gates.

Each worker also passed optimized tests on its final source. CE3 schema generation
and conformance were qualified in candidate 2; candidate 3 changed no schema
sources, and the inheritance is explicit in its receipt. Actual-process fault
tests cover local observation/establishment stages and campaign preparation/
dispatch. They do not qualify the deferred cross-owner workflow.

## Review and remediation

One separate read-only reviewer retained findings across three candidates. Ten
P2 findings were raised and closed; no findings remain open in the bounded source
review. This makes no provider/model-independence claim.

The fixes cover complete size accounting and preallocation checks, custody
reconciliation, profile-owned artifact requirements, timestamp compatibility,
duplicate-preserving artifact comparison, persisted campaign semantic bindings,
read-only unsupported-schema refusal, pure retry predicates, per-obligation
attempt handling and removal of unintended reader-only count limits.

The first remediation closed seven findings but introduced three write/read
inconsistencies; candidate 3 closed those with regressions. A missing campaign
admission file was added to the final nine-path manifest before acceptance.
SC2's early R1/R2 failures were observed in tool output without retained raw logs;
its initial R3 red run was masked by a concurrent dependency compile error. These
limitations are recorded rather than reconstructed as stronger evidence.

## Remaining work and limits

The scoped read-only episode API, concrete CE3/campaign/episode readback join,
downstream cross-owner fault cuts and HP3 execution/transport custody remain
pending. So do provider-backed positive retry, full host responsiveness,
terminalization/acceptance, saved-state disposition and Node retirement.

SC2's 129-entry selection and 129-evaluation-state tests pass. A separate
129-attempt stress run was stopped after progressing through 40 slots in more
than two minutes under historical validation. Its log is retained; this work
does not qualify that history size or claim a throughput improvement. Measure
and address supported-history performance before the host qualification gate.

Workflow: two implementation workers, one retained read-only reviewer, two
remediation rounds and final parent combined validation. No Claude calls or
provider calls were made by this implementation effort. Requested implementation
model was `gpt-6-sol/high`; effective model identity and complete token totals
were not independently exposed.
