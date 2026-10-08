# HP2 SC1 and CE2 validation

Completed 2026-10-08 under the user's explicit authorization to proceed with
parallel campaign SC1 and claims CE2. Both implementation/remediation workers
were configured as `gpt-6-sol/high`; a separate `gpt-6-astra/xhigh` reviewer
retained the findings through remediation. This is same-provider source review,
not a provider-independence claim.

## Result

SC1 supplies the permanent `slice-campaign` library: trusted private-root
admission, authorized phase changes, read-only verification of the frozen SC0
candidate/profile/gate input, exact selection, and durable initial-request
preparation. Its SQLite root owns revision CAS, operation receipts, reservation
identity and uncertainty readback. It does not dispatch a provider.

CE2 extends the accepted `claim-evidence` library with exact-current finding
reliance and bounded exact-revision projections. Reliance retains immutable
history; changing a finding revision removes current projected reliance.
Checked projections retain provenance, completeness and limitations, accept
1–100 unique exact selections, and reject stale or mismatched input. Versioned
DTO schemas and a generator accompany the authority-free consumer data.

Campaign reuses the claims codec and the published episode codec. It retains
ownership of selected claim documents and their subject/boundary/consumer
validation. No common codec package, temporary Node bridge or new daemon was
introduced.

## Qualified source and integration handoff

The isolated joined checkout is
`/home/bline/code/work-engine-hp2-sc1-ce2-joined`, based on published S2
`b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2` including S3. It mechanically composes
accepted CE1, SC1 candidate 1 plus its remediation, and CE2 candidate 2.
All 117 product paths match the final lane manifests; the only shared-file
composition is Cargo membership and the two domain package lock entries.
No third-party version changed. The claims JSON Schema test dependency uses the
existing workspace pin; runtime dependency identities remain recorded in the
lane manifests.

| Artifact | SHA-256 |
| --- | --- |
| Joined source aggregate | `341ae2548a84308ad838c807530652ac17cecd40928dd3a66ae58cad6701d8ba` |
| Joined source manifest | `00aaa754b32ffa83edfc6f39585be4f90b3ed68bc529598b3c96642b1e2d8178` |
| Combined product patch | `36debaa5ac41197d4f708d37695a3425f74aabcc58edfae79a388fbf539ce560` |
| Joined source archive | `ee69bafc44fefad0fa9ea9153ca7646177473996b36e91a2f29668ad0adb0ddb` |
| SC1 candidate 2 completion | `85212826db89cc105d578ae18c67abd77a2057a246d7c302aa3bbccf02c35118` |
| CE2 candidate 2 completion | `5335f976361931ebc953dbc804a2987683db91838e5e0df38a88fea1b13c54c2` |

Evidence root:
`/home/bline/.local/state/work-engine/rust-hp2-sc1-ce2-20261008/`.
`joined/handoff.json` names the archive and patch; `joined-source-manifest.json`
contains every path/hash. The aggregate hashes manifest-order UTF-8 records of
`path + NUL + lowercase SHA-256 + LF`. `joined/patch-reconstruction.json`
records independent temporary-index application to the published baseline and
byte verification of all 117 reconstructed paths. No branch or ordinary commit
was created by that check.

Shared main Cargo files remain unchanged. S4 retains their integration ownership.
The combined patch includes CE1, so an integrator must use either that complete
baseline patch or the separately recorded incremental lane patches, not apply
CE1 twice. This handoff does not include isolated HP1 host changes or S4.

## Review and final gates

One repair cycle in each lane closed seven findings:

- SC1: fresh request-handle versus reservation digest mismatch; missing
  database/root identity binding; incomplete historical profile field-type
  checks; and numeric overflow in malformed profile input.
- CE2: projection-envelope depth exceeding the inherited codec limit;
  reliance content not re-bound to its saved operation/grant digest on readback;
  and a bootstrap permission omitted from the published schema.

The final reviewer rechecked exact source before and after its probes.
`review/sc1-candidate2-review.json` has SHA-256
`1c10bf9a8ad85c819e2ceb3bc8a0afcfaece22b4d0880772b5a3b0bbd9391118`;
`review/ce2-candidate2-review.json` has SHA-256
`ee58106e868b2cda823dabe8dfb5c3beea2a0743f60dacf98640208e3bd0fe83`.
Neither retains a material finding in the reviewed scope. Original candidates,
probe outcomes and remediation receipts are preserved.

Final parent gates on the joined checkout passed:

| Gate | Result |
| --- | --- |
| Locked tests, both packages | 28 passed |
| Locked optimized tests, both packages | 28 passed |
| Both nondefault fault features | 37 test entries passed, including subprocess helpers |
| Clippy, all targets, default and fault features | Passed with warnings denied |
| Whole-workspace formatting | Passed |
| Source rehash and combined patch reconstruction | All 117 paths matched |

`parent-joined/results.json` has SHA-256
`4c8502ba5c00965cd71a03319887790cdd96e9cdd041024afdc6afff63156cc0`.
Raw logs and per-binary totals are retained beside it. CE2 also passed separate
parent gates before the join. Tests use real SQLite roots, actual-process crash
cuts, frozen legacy oracle vectors and immutable Git objects. Scratch and build
outputs stayed below `/home/bline/code/.work-engine-tmp/`.

The main staged/working status inventory was unchanged before documentation;
no main staging, reset, commit or runtime edit was issued. Its index byte hash
changed after the initial pre-status observation, so this receipt does not claim
byte-identical index preservation. Git status can refresh index metadata;
`main-preservation-audit.json` records the observation and evidence limit.

## Scope and next boundary

This qualifies two libraries together, not HP3's real cross-domain admission or
provider execution. SC1 retains the bounded SC0 source-tree gate profile; it is
not a general gate executor or checkpoint/profile producer. CE3 production-path
evidence and SC2 completion/recovery/retry/evaluation remain later slices.
Retained remediation, terminalization, saved-state disposition, operational
cutover and host responsiveness are not enabled or qualified by this result.

No ordinary commit, push, live-state migration or activation occurred. The next
implementation planning boundary is CE3 alongside the portions of SC2 that can
proceed against its accepted contracts, with their actual owner-readback join
made explicit. Shared workspace integration stays serialized with S4's owner.

Workflow: two implementation workers ran in parallel, one reviewer handled
both exact subjects, and each lane needed one review/fix iteration. No Claude
calls or parent gate failures occurred. Wall time was approximately 36 minutes.
Token and exhaustive retrieval totals were not exposed as reliable aggregate
metrics; none are inferred from source manifest counts.
