# H0 trusted native admission implementation and validation

Implemented 2026-10-13 UTC under the user's bounded H0 authorization. The owning
contract is [HP3 H0](rust-hp3-controlled-host-implementation-plan.md#h0-exact-episode-admission-without-a-per-command-subprocess).
This report covers the shared working-tree change, not a published checkpoint or
an independent review.

## Result and scope

`review_episode::host_admission::NativeHostAdmission` now exports:

```rust
pub fn from_trusted_descriptor(
    selection: NativeRootSelection,
    canonical_descriptor: &[u8],
) -> AppResult<Self>
```

The inherited-FD constructor retains its private, current-UID, read-only,
unlinked-file checks and its bounded read. It delegates canonical descriptor
parsing and exact selection validation to the direct constructor. Both routes
now call the store owner's `read_native_selection` before issuing admission and
compare the actual selection with the supplied value. That readback validates
the native marker, absolute root identity and actual current executable digest.
The existing request, authority, result, evidence, content, principal and
observed-revision checks in `precheck`/`admit` are unchanged.

The constructor deliberately trusts linked Rust composition to own descriptor
provenance. It does not authenticate arbitrary in-process code or make serialized
bytes a wire grant. The future H1 private startup capability remains responsible
for constructing descriptors from installed authority and exact command facts;
H0 adds no H1 consumer or startup scaffolding. It changes no store schema, marker,
ER1 read capability, campaign behavior or executable interface.

Owned changes:

- `rust/crates/review-episode/src/host_admission.rs`
- `rust/crates/review-episode/tests/trusted_native_admission.rs`
- this report

The existing public module makes a separate export edit unnecessary. No manifest,
lockfile, other shared source, Git index or branch modification was performed by this
worker. No provider or live execution was performed.

## Validation

Commands use a separate build target to avoid competing with parallel workers:

```bash
CARGO_TARGET_DIR=/home/bline/.cache/work-engine/h0-admission-target cargo test --manifest-path rust/Cargo.toml --locked --offline -p review-episode --test trusted_native_admission
CARGO_TARGET_DIR=/home/bline/.cache/work-engine/h0-admission-target cargo test --manifest-path rust/Cargo.toml --locked --offline -p review-episode --features test-faults -- --test-threads=1
rustfmt --edition 2024 --check rust/crates/review-episode/src/host_admission.rs rust/crates/review-episode/tests/trusted_native_admission.rs
```

The initial focused run passed 8/8 tests before the additional marker-integrity
case was added. The final suite includes nine tests (eight behavioral tests and
the subprocess entry point). It covers:

- Identical direct/inherited admission decisions for valid begin/result
  descriptors, altered bindings, wrong principals and stale observations.
- Changed request, authority, result, evidence and content bytes.
- Malformed, invalid UTF-8, noncanonical, oversized, extra-field and wrong-profile
  descriptor bytes.
- Continued inherited-FD refusal for writable, linked and nonprivate files.
- Supplied selection values that match descriptor bytes but differ from the
  actual root selection; root relocation; a root initialized by another actual
  executable; changed and missing marker bytes.
- Real direct begin/result writes, replay without append, descriptor observation
  refusal, reducer expected-revision CAS refusal, exact ER1 result readback and
  continued ER1 write refusal. Durable history remains exactly two entries.

The initial full `test-faults` run used default parallel test execution. Its
command suite passed 12/12 and crash suite passed 6/6, then `hp1_direct` passed
6/7: `direct_admission_rejects_mutated_scope_and_stale_revision` received
`OutcomeUnknown` on begin at line 490. The same test binary's
`direct_possible_commit_exposes_exact_context_and_reconciles_once` sets the
process-global `REVIEW_EPISODE_FAULT_CUT=after_commit_unknown` at lines 584–588.
This provides a concrete source-supported explanation for a cross-test fault
interaction; no trace captured the precise scheduling. The failure is preserved,
not waived. Serial execution isolates that mutable environment; no unrelated
test source was changed.

Final serial feature run: **47 passed, 0 failed, 2 ignored** (42 integration
cases plus 5 compile-fail documentation tests). Breakdown: command 12, crash 6,
HP1 direct 7, relative root 1, native host 3, SC2 readback 4, H0 9, doctests 5.
The two ignored HP1 history tests remain explicit diagnostic/import probes.
`rustfmt --check` and the scoped `git diff --check` passed.

Exact final source SHA-256 values (working-tree bytes; no checkpoint created):


- `rust/crates/review-episode/src/host_admission.rs`: `562e2c21ae0862db1ccc3f51483d181604dd27ba72345993b85766472b2783d2`

- `rust/crates/review-episode/tests/trusted_native_admission.rs`: `36f292388fb6a48200b3d26deda2eed6788df85a93dc551040d372a0f6a7905e`

## Evidence and limits

Tier 2 repository verification used graph project `home-bline-code-work-engine`,
initial generation `2026-10-11T22:56:21Z`, store `ude7dece53a17b29eg6`.
Exact symbol lookup returned `NativeHostAdmission`, `from_inherited_fd`,
`NativeRootSelection` and `read_native_selection`; all four rows were consumed.
The depth-one both-direction trace of `from_inherited_fd` returned all nine
reported callees and three callers (native executable, SC2 test and campaign
owner-join test). Native selection validation was read through its verified
graph snippet; material source and test helpers were also read directly.

Coverage checks reported no recorded issues with verified content for all
existing relied-on source, test and plan paths. The index refreshed during work;
these best-effort signals are not completeness proofs. The diagnostic exact-name
query for the two HP1 test functions returned zero; their exact source was read
instead of inferring absence. New H0 test code was verified directly and by
compilation/execution. A final coverage check at generation
`2026-10-13T03:10:27Z` also reported verified content and no recorded issues for
the implementation, new test and report paths.

The tests qualify H0 admission and its existing episode consumers on Unix/Linux.
They do not qualify H1, arbitrary linked-code isolation, provider independence,
read-only child enforcement or campaign outcome finalization. The shared tree
contains unrelated concurrent changes; this report does not attest a frozen
whole-repository subject.
