# Research lifecycle M2 — bounded R-2 qualification packet

Date: 2026-11-11. Status: **packet prepared; execution not authorized**.
This packet records investigation and proposes the next bounded objective. It
does not accept R-2, clear licensing, authorize repairs or integration, or start
an independent recheck. The user approved **$5 hard cap / $3 warning** for packet
preparation only, using API-equivalent dollars adjusted for subscription speed.

The [implementation plan](research-lifecycle-rust-implementation-plan-20261111.md)
supplies the milestone sequence. The [M0 contract](research-lifecycle-m0-contract-and-first-use-20261111.md),
[owner approval](../records/research-lifecycle-m0-approval-20261111.md) and
[M1 result](../records/research-lifecycle-m1-result-20261111.md) supply accepted
scope and subsequent progress. Earlier planning status labels do not undo those
approvals. Shared HEAD for this investigation is
`a854c90f08013df1c5e517ed5ff398258e03073f`.

## 1. Recommended next action and its dependency

Give the recovered authoring evidence below to the owner of Q-B1 for a documented
provenance/licensing disposition. **Q-B1 remains unresolved.** Functional parity,
a source header, a model's recollection and this packet do not establish legal
clearance or independent derivation. No source copying allegation is established
by the saved review either.

Once an authorized disposition permits the intended use, commission one bounded
qualification/composition objective on the preserved candidate, strengthen the
identified tests, obtain a completed independent review, and integrate only the
accepted exact result. A provenance repair or replacement changes the subject
and needs a newly scoped plan and budget; it is not covered by a narrow F-01
delta verdict.

The recommended next engineering allowance is **$12 hard cap / $8 warning**,
proposed rather than granted. Section 7 defines its scope and limits. External
legal services and replacement of the diff implementation are excluded.

## 2. Exact subjects and present standing

| Subject | Exact identity / standing |
| --- | --- |
| Original integration base | `6f3d6c716d5c499fec7a694bf43b52fb4b2d2548` |
| Original R-2 | `c67c51245e6bd9c1e6cf98391bbbb48b02d618fe`, tree `8978848812ed582447443ad67765638c4b4ca95d`; original independent **REJECT** on F-01 |
| F-01 repair | `0d021609482ad89841eb94f5c0a156e328efb7a6`, tree `38a00905fc631232a453331272b258571d04a152`, parent `c67c5124` |
| Diff source in repair | `rust/crates/research-register/src/anchor/diff.rs`, blob `da276adc5896f5ec590f7ceebd67b47fe485909b`, SHA-256 `7652481dc42094887929484f935f24c8a3329a98a5fc6e34c15ee4597e18e247`; 400 lines, 14,850 bytes |
| Shared M1 base for future composition | `a854c90f08013df1c5e517ed5ff398258e03073f`; M1 code accepted, tested and committed |
| Repair recheck | Positive executed evidence, **no completed independent verdict**; coordinator ACCEPT WITH RESERVATIONS remains separately attributed |

Both R-2 candidates remain outside the shared HEAD's ancestry. The original
R-2 delta has 38 files, all under `research-register`; the repair changes only
`src/anchor.rs` and `tests/anchors.rs` (+80/-11). `diff.rs` is unchanged by that
repair. No candidate or shared implementation was modified in this phase.

The [qualification checkpoint](../records/research-register-r2-qualification-checkpoint-20261111.md)
remains the historical record. Its source and saved decision packets distinguish
executed checks, source review, inference and incomplete work.

## 3. Provenance investigation: what is newly established

The original Claude builder transcript was recovered locally:

```text
/home/bline/.claude/projects/-home-bline-code-work-engine/
82d02fe5-1f17-4f5e-bf13-2c521c27cac6/subagents/agent-ae6cf23dd655e836a.jsonl
SHA-256 195f1d9f7df745d96bbb3f281d6f1471e8e214bae3cb6536aa2305db26949ab4
```

The recorded interval is 2026-11-11T18:21:09.087Z–18:42:40.959Z. It contains
59 tool invocations: 47 Bash, one Read, six Write, four Edit and one handback.
Line 93 records the initial write of `diff.rs`; its event SHA-256 is
`2df078d95095c7d212c4028e2f36745eb36993e9aa9bf353cbfe9ae24b5f5ff8`.
The initial source SHA-256 is
`f8527732513f5c156eee4b4f5952573b775e672237f2f55dbeda6930228e0968`.
The `slide` function between `fn slide(` and the Stage 8 comment is byte-identical
in that write and the committed candidate: SHA-256
`5c7534064258ac9879dd21f49547729f30dfad3eae5dcca585f3f79e2d69ce10`.

This is attributable authoring evidence for the control flow that concerned the
reviewer. The visible commands show repository/reference inspection, source
generation and differential testing. The handback at line 232 asserts no Git or
xdiff source was opened or copied and attributes the compatibility choices to
general knowledge. That statement remains the author's assertion. The trace
does not attest the model's prior exposure or establish a complete independent
derivation history. A cited paper was not supplied as a frozen derivation input
by this investigation.

The earlier reviewer read the Rust but had neither upstream source nor these
development records. Its O3 finding identifies a distinctive combination of
tail trimming, occurrence filtering, search conventions and opposing-group
sliding; it explicitly reports **no demonstrated source-expression match**.
The recovered trace improves the evidence available to the Q-B1 owner; it does
not resolve the reservation. This pass makes no license-compatibility judgment
and did not compare or import upstream implementation source.

Available dispositions are to establish a sufficient, documented basis for the
current implementation's intended use, or to require a separately scoped
replacement with its own provenance and functional qualification. Merely
renaming or refactoring the disputed control flow would not establish origin.
An owner authorization to evaluate privately is distinct from a documented
distribution disposition and cannot itself establish license compatibility.

## 4. Evidence inventory and corrections to broad summaries

The preservation manifest, 20,564,705-byte candidate bundle and all twelve listed
evidence artifacts match their recorded hashes. The nested package manifests
also verified: **164 original-review entries, nine repair-packet entries and
eleven recheck-package entries**, all matching. These checks authenticate
recorded bytes; they do not rerun or upgrade the recorded conclusions.

| Evidence under `/home/bline/cbm-work/` | SHA-256 |
| --- | --- |
| `research-phase-closure-20261111/r2-preservation/MANIFEST.md` | `f300c3e7e8761617d1289aeff4ee1b2bd699e78561edbeb1ddb2e28ef2e3a69a` |
| Preservation `r2-candidates-20261111.bundle` | `1a1f3257333905cf7c1c25b51c5b7fac359721c7ba0cee35add8f11864457a76` |
| `r2-independent-review-20261111/coord/reviewer-output-frozen/report.md` | `5a1c06c087448bfedb61ac925319ea4fd5941e60bc08a4c9f5467228dedfedfd` |
| `r2-f01-repair-20261111/recheck-packet/MANIFEST.sha256` | `6a714107d87f74f591a23092987e28289fa98433b02d4aa3b73be5080db9c6df` |
| `r2-f01-recheck-20261111/coord/reviewer-output-frozen/report.md` | `7b5b87147de9b98b09606ae63ac7d2494f1e5c44ac74e9b641059e4e59ab92a8` |
| `r2-f01-recheck-20261111/decision-packet.md` | `9ae10405f88ef6e8b5c2726f23ba717faa5791ed91859eb6fd788327fde71a1c` |

F-01 discrimination was executed: the new missing-revision regression fails on
the original and passes on the repair. Missing maps to `cannot_read`; newly
ambiguous remains `invariant_failure`. The recheck executed checks 1, 2, 5 and 6,
source-reviewed 3 and 7, and did not finish independent check 8. Its $2 API-route
cap could not reserve the next request despite only about $0.52 spent. This is
not a reviewer acceptance or evidence that the remaining pass costs $0.52.

Workspace-check history needs a precise qualification. The original closure
reports a successful root-run check on `c67c5124`; its `root/gate-check.log`
ends with a finished dev profile and has SHA-256
`c2327b08879bc2b28e2476ce5c5b7d26dc247eb913658a7079b4f9822df4f22a`.
The independent original review timed out during workspace checking; the repair
review did not establish completion. None qualifies the repaired candidate
combined with M1. A fresh combined-workspace check is therefore required.

## 5. Remaining checks and their consequences

| Item | Required result for the proposed bounded objective |
| --- | --- |
| Q-B1 | Attributable disposition with its scope and evidence before integration/distribution; otherwise preserve the unresolved reservation |
| Required null anchor | Preserve M0's approved first-use refusal: no affected card and a typed gap; `null`/`not_established` never becomes `absent` or confirmed support |
| F-01 / check 8 | Completed independent judgment on the exact repaired/composed subject, with the original rejection, repair and all new findings retained |
| Missing packed revision | A discriminating fixture validates successfully, removes only the required revision's packed availability, obtains `cannot_read` for that revision and no partial report, and has a valid restored control |
| Post-validation ambiguity | Demonstrate the actual lookup path if feasible with a bounded fixture; preserve unit evidence and explicitly report any unmet end-to-end condition |
| Resource limits FO3-01 | Inventory and bound the entire invoked batch, preserve timeout/unavailable outcomes, and establish the enclosing execution controls for the intended environment |
| Full-history requirement Q-B5 | Supply every pinned historical object; run history tests rather than silently skipping. A limited CI/archive environment must report unavailable qualification |
| Composition | Preserve M1's module and proof boundary; run crate gates, oracle/history checks and the combined workspace check on the exact final subject |

The packed-object fixture must keep target/tree/blob inputs readable; deleting
an arbitrary pack can mask the revision-specific failure. A precomputed pair
of colliding revision prefixes could exercise ambiguity: expose only the first
commit for validation, then the second for anchor lookup. This is a proposed
test strategy, not executed evidence or a requirement to brute-force collisions
inside CI. Freeze the fixture and cap preparation work; report a feasibility
failure rather than expanding the objective silently.

The selected current register contains **eleven decisions**. R-2's
`anchors(&ValidRegister, &PinnedSnapshot)` iterates every registered decision;
it is not a selected-H2-only computation. At the named shared HEAD the six
distinct target blobs total 267,844 bytes; the largest is 100,621 bytes / 2,337
lines. The H2 target is blob `cc9327188f64fefb1d98ac652c98404dc7d79fc2`.
These are Git object inventories, not successful validation/anchor execution or
a worst-case performance bound. Revision-side blobs and full batch costs also
need admission inspection.

For the first bounded qualification, propose a preflight of at most sixteen
decisions, 1 MiB and 10,000 lines per source/target blob, and 8 MiB aggregate
input bytes, with a 60-second/1-GiB/8-MiB-output enclosing anchor invocation.
These are proposed resource allowances, not calibrated scientific thresholds;
verify both fit and behavior under refusal. Allow longer separate gates for
compilation and the complete oracle/history suite. External test limits do not
create a timeout inside the synchronous Rust API. M3 must bind the actual
in-process admission/enclosing-host controls or retain an explicit unmet
resource condition; this packet does not invent cancellation support.

Rename following remains outside this objective. M0 settled the refusal policy
for the selected use, not the broader rename/partial-card contract. Actual A4
enforcement belongs to M3 and must be tested there.

## 6. Concrete future implementation and review packet

Use an isolated worktree based on refreshed shared HEAD, initially the M1
commit named above. The integration lead owns the whole composition. Preserve
the original candidates and evidence. The R-2/M1 overlap is `src/lib.rs`:
retain M1's `use_bundle` module and add the accepted anchor exports deliberately.
`src/main.rs` and the Cargo files still match R-2's original base at the
investigation HEAD. Recheck this at execution; no compatibility claim is made
for a future changed checkout.

The default builder proposal is Sol 6.1 / medium, with ownership limited to
R-2's crate delta, discriminating qualification tests and composition edits.
Preserve accepted M1 semantics. Changes to `diff.rs`, the error contract or a
new dependency require refreshing the review scope and budget before treating
the old algorithm evidence as applicable. No A4 implementation, register
publication, task-host change or live H2 run is included.

Capture exact source inputs and command/process custody. In the isolated Rust
workspace, use a dedicated external `CARGO_TARGET_DIR` and run:

```text
cargo test -p research-register --locked --offline
cargo fmt -p research-register --check
cargo clippy -p research-register --all-targets --locked --offline -- -D warnings
cargo check --workspace --all-targets --locked --offline
```

Keep the complete diff oracle and history evidence visible. Retain earlier
4,736-pair and 385-row results within their actual scope: the historical sample
contains only unchanged/moved/modified; it is not all-class or full-envelope
parity. Rerun the available differential against the final subject where it
protects changed behavior. Preserve meaningful original-versus-repaired failure
discrimination. The optional external-register helper is distinct from a
required history/oracle skip.

Freeze the resulting commit/tree, full main-relative patch, input manifest and
gate digest before launching a **fresh read-only Astra / xhigh / default**
review. Give it the old reports, exact M0 scope, candidate and test evidence,
including the provenance reservation. Require a completed scoped verdict and
finding dispositions; a coordinator cannot supply the missing reviewer verdict.
Observe actual reviewer identity, exposure and capability configuration. Retain
that reviewer for bounded remediation, recording each new subject accurately.

After an accepted exact subject and the required Q-B1 disposition, the
integration lead checks current HEAD, index and affected bytes, transfers only
the reviewed delta, verifies M1 preservation and reruns appropriate composition
checks. Commit/publication and any push retain their separately established
authority. No automatic A4 or live execution follows integration.

## 7. Proposed next budget and stop boundary

Request **$12 hard cap / $8 warning**, in the same speed-adjusted API-equivalent
proxy units, with a new objective record and account-capacity check. A planning
allocation is $3 builder/test work, $4 separate review/remediation, $3 supervision
and integration, and $2 contingency inside the single cap. These allocations
are controls, not measured forecasts or independent spending grants.

This covers the bounded tests, composition, gates, completed review, ordinary
remediation, qualified local integration and result record described above.
It excludes legal services, a new diff implementation or dependency migration,
rename support, broad resource-framework work, A4, live execution and push.
If Q-B1 requires replacement, stop that route and present a concrete replacement
packet rather than consuming this allowance as if the subject were unchanged.

The prior Pi proxy's request-reservation failure informs future route selection;
it does not transfer a dollar allowance between subscription and API pools. The
proposed review route is the separately bound standard-tier Codex host. A move
to Pi/API needs explicit route, pricing and worst-case reservation capacity.

At the hard cap, admit no new work, collect owned results/processes and report
the actual incomplete gates. An unresolved mandatory condition is not waived
by spending the allowance. The $8 warning prompts reassessment; it is not a
second hard cap. This execution proposal remains unapproved.

## 8. Packet evidence and limits

Investigation artifacts live under
`/home/bline/cbm-work/research-lifecycle-m2-packet-20261111/`: exact approval,
candidate identity, preservation/package verification, authoring trace summary,
source copies, current batch inventory, overlap inventory and budget/continuation.
They are host-local references, not a portable evidence distribution. Do not
publish the native source or raw session transcript as part of this packet.

Codebase Memory Tier 2 project `home-bline-code-work-engine`, generation
`2026-11-12T01:52:22Z`, indexes the shared tree. Exact-path checks found historical
anchor paths missing from that tree, so this investigation used immutable Git
source. Current `lib.rs` was content-verified; other inspected code paths have
unresolved calls. Byte inventories and exact diffs do not claim complete graph
coverage. No runtime experiment, build, independent recheck, repair, integration
or scientific qualification was performed here.

Packet exit means a reviewable bounded plan and truthful evidence inventory
have been delivered. It does not mean M2's engineering acceptance gate has passed.
