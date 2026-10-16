# Installed-host terminal refresh validation

`tests/task_refresh_host.rs` is an opt-in Linux integration test. It drives the
ordinary installed host protocol through `lifecycle-proof`, uses that crate's
auth-free loopback simulator, and launches the actual terminal UI binary in a
PTY. `tests/support/terminal.rs` decodes the evolving display with `vt100`.
The simulator and mutation client are test-only dependencies; the application
continues to consume read-only client contracts.

## Demonstrated journey

The test creates fresh stores, socket paths, native home and barriers under a
unique private evidence directory. It submits a controlled task, waits for its
pending question, and opens one UI process. After selecting Conversation once,
it sends no refresh, reconnect, resize or key input between stages:

1. The pending question is visible; answer and result text are absent.
2. An authorized test answer is submitted through `task.answer.v1`; automatic
   refresh displays it while result publication remains held.
3. The simulator publishes a result using exact current owner source references;
   automatic refresh displays the result in the same UI process.
4. Owner answer authority, published result, completed lifecycle delivery and
   completed native segment are verified independently through the host.
5. The UI exits successfully and restores terminal settings. The host exits and
   is reaped; exactly three simulator requests were consumed.

Each screen assertion has a deadline; raw PTY output is bounded. Child guards
clean up on assertion failure. Captures, requests, readbacks, simulator ledger,
startup configuration and `result.json` remain in the printed evidence directory
on success or failure. `result.json` binds UI binary hash and host identity.
Screen wait durations include workflow/read timing; they are not isolated
refresh latency measurements. A successful text assertion is not a visual
usability review.

## Running

Use Rust 1.92 and a compatible installed `work-engine-task-host` compiled with
`controlled-proof`, plus a trusted **S8 controlled fixture startup template**
produced by `lifecycle-proof/tests/common/mod.rs::create_fixture`. The template
must retain accessible, matching executable/source/manifest/lock/toolchain pins
and the installed native binary required by `lifecycle-codex`. This test does not
build or bless the host. The host validates its pins at startup.

The template's immutable build references are reused; its task/lifecycle stores,
sockets, native home and barriers are replaced with fresh paths. Fixture grants
receive a bounded ten-minute expiry and the provider URL is replaced by the
finite local simulator. No saved task state, credentials or real provider
endpoint is reused. The test intentionally requires the known S8 fixture
subject/authority profile, rather than accepting arbitrary production configs.

Set these absolute paths before running:

```bash
export UI_TEST_HOST=/absolute/installed/work-engine-task-host
export UI_TEST_STARTUP_TEMPLATE=/absolute/controlled-fixture/host-startup.json
export UI_TEST_EVIDENCE_DIR=/absolute/short/private/evidence
export TMPDIR=/absolute/private/tmp
export CARGO_TARGET_DIR=/absolute/ui-test-target

cargo +1.92.0 test --offline --locked \
  --manifest-path rust/crates/terminal-ui/Cargo.toml \
  --test task_refresh_host -- --ignored --nocapture
```

Create the evidence and temporary directories first, owned by the current user.
Keep the evidence path short enough for Unix socket paths. `TMPDIR` and
`CARGO_TARGET_DIR` are also required by the native launcher. On a fresh machine,
fetch locked dependencies before requesting an offline build. The regular test
suite compiles this test but skips execution; a skipped test is not a pass.

## Evidence and remaining limits

On 2026-10-16 the repository test passed against host executable SHA-256
`2703037bbcbafb7ec285f9dc9b9688a47efcb59b8dbbac3e23a68a94002c899f`, whose
pinned source manifest SHA-256 is
`4f446a2a8077f4149c3d6c97e6d7a15ce3a000242bdb75f747d5b620f5bf86e5`.
This is the retained S8 build, not a claim about a newly built current host.
The first compile encountered `/tmp` quota exhaustion; retrying with a private
`TMPDIR` passed. Local run receipts are under `/home/bline/.ui-rust-proof/`.
Earlier one-off checks are retained under `/home/bline/.we-ui-refresh-20261016/`
and `/home/bline/.we-ui-sim-20261016-r2/`; they are separate evidence.

This closes the repeatable pending/answered/result **Conversation auto-refresh**
proof. It does not establish multiple native segments, capacity refusal,
installed-host partial failure/restart, stale-reply fencing, draft preservation,
or live refresh of every view through every transition. Those remain separately
covered by controlled-peer/unit tests or unqualified as installed-host cases.
Last-read age and delivery/segment inspection still need presentation review.
It is not UI-0/UI-1 acceptance or a paid-provider production qualification.
