# Mutant M1: Messages model check disabled

File: `rust/crates/work-engine-review-hosted/src/check.rs`, `messages_body` (the `model` comparison, line 404 at baseline).
Baseline sha256 (before and after): `caf32cf8b73c98576aa6fc5ac3b0232dcb26787fa44991e36f8b3e721a917a15`.
Tests file sha256 at the time: `69d77577195ab648b26c22555d6c86267e843c1baaf983ff5c968547b611d222`.

Mutation (one `Edit`, unique context: the `let mut names: Vec<String> = parsed["tools"]` line follows):

```
-    if parsed["model"] != model {
+    if false && parsed["model"] != model {
```

Mutated sha256: not recorded for the run below (the hash was computed only for the earlier run of the same mutation, before the tests were split; the source edit is identical).

Command: `cargo test --manifest-path /home/bline/cbm-work/wt-org-hosted-review-20261117/rust/Cargo.toml -p work-engine-review-hosted --offline --test check`

Result: exit 101; 26 passed, 1 failed. Failing test (only one): `claude_sdk_refuses_a_request_for_another_model`, case `wrong model`:
`assertion left == right failed: wrong model was not refused  left: "checks_passed"  right: "checks_failed"` (`tests/check.rs:698`).

Restore: inverse `Edit`; `sha256sum` of `src/check.rs` = `caf32cf8b73c98576aa6fc5ac3b0232dcb26787fa44991e36f8b3e721a917a15` (equal to baseline).

Earlier run of the same mutation, before `claude_sdk_refuses_wrong_model_and_a_wrong_tool_set` was split into `..._another_model` and `..._a_wrong_tool_set` so each mutant has its own killer: killed by the combined test (mutated-file hash computed at the time but not copied into this record; restored hash `caf32cf8…`, equal to baseline).
