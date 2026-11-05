# Local development system qualification — 2026-11-05

Status: first joined **offline** process test passed. Complete real development
acceptance remains held at the gates in the
[integration plan](../../planning/local-development-integration-and-qualification-20261105.md).
The [candidate](candidate.json) binds exact subjects and custody references.

The new Rust test is
`events_http_mcp_preserves_the_real_job_operation_across_server_restart` in
`rust/crates/work-engine-server/tests/coding_job.rs`. It uses the production
Events HTTP MCP entry, production command executable and production Rust job
owner with that owner's existing disposable fake Host. It verifies:

- missing transport authentication is refused;
- MCP initialization and exactly the four job tools are available;
- submit and an exact repeat after collection identify one operation;
- restarting the Rust server preserves that operation and its result;
- a wrong job credential produces the native owner error without a second Host;
- result acceptance remains `owner_pending`, with effect settlement `not_claimed`.

The unconditional verifier is synthetic. The Host emits a fixture receipt;
there is no provider, Docker, real coding task, deployed identity or authority
activation. This test does not classify the earlier intermittent parallel
repeat-submit failure: its repeat deliberately happens after collection.

## Reproduce the joined test

From a checkout containing this change, on the supported local Linux host with
Rust dependencies already cached and `/usr/bin/python3` available:

```bash
cd rust
integration_target="$PWD/target/local-development"
CARGO_TARGET_DIR="$integration_target" cargo build --locked --offline -p work-engine-command
CARGO_TARGET_DIR="$integration_target" COMMAND_BINARY="$integration_target/debug/work-engine-command" cargo test --locked --offline -p work-engine-server --test coding_job events_http_mcp_preserves_the_real_job_operation_across_server_restart -- --ignored --exact
```

The test is explicitly ignored without its command-binary prerequisite, as the
existing CLI process test is. `--ignored --exact` runs this case rather than
silently skipping it. Use `-- --include-ignored` to run the complete job
composition test target with the same environment.

Every test run creates fresh private fixture directories, credentials,
authorization, job state and processes. The command executable is copied into
a private single-link file before pinning, matching the existing bridge fixture
contract. No historical store or signed answer becomes fresh authority.

## Evidence and limits

The retained evidence includes the 17-test Events/CLI baseline, two accepted
AH839/Web285 browser/common-profile replays, and the new joined test's build,
focused and final composition/Clippy/format checks. Exact counts and outcomes
are recorded in the candidate and custody outputs. The AH baseline used its
older selected binaries and private source worktrees; it is not qualification
of all later shared-branch changes. Replaying that historical AH recipe still
requires its referenced worktrees and browser caches. The new joined test above
uses the current checkout and needs none of those private AH artifacts.

Two initial new-harness runs failed and are retained: first the bridge correctly
refused Cargo's hard-linked executable pin; after staging a private copy, the
test needed to decode native errors from the MCP content envelope rather than
the success-only structured envelope. Both corrections were in the test. No
production pin, error or authority rule changed. The final assertions also
check initialization and the exact advertised tool set.

The [evidence manifest](evidence-manifest.json) maps copied custody/log files to
their original private paths and digests. Absolute paths in historical records
retain their original meaning. Final source and command hashes are bound before
their runs; the broader source manifest remains in the private assignment.
These are functional test and source-review results, not a new independently
reviewed live execution profile.

The [S10 owner decision](S10-OWNER-DECISION.json) is one completed consultation,
not a resumed pilot or source grant. The source owner confirmed that seeded
task input is the minimum coding retry change and that no supported producing
binary ACK command currently exists. Preserve the old pending receipt and its
unknown reconciliation route.
