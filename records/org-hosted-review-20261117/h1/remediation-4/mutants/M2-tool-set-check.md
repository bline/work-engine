# Mutant M2: Messages tool-set check disabled

File: `rust/crates/work-engine-review-hosted/src/check.rs`, `messages_body` (the sorted-names comparison, line 414 at baseline).
Baseline sha256 (before and after): `caf32cf8b73c98576aa6fc5ac3b0232dcb26787fa44991e36f8b3e721a917a15`.
Tests file sha256 at the time: `69d77577195ab648b26c22555d6c86267e843c1baaf983ff5c968547b611d222`.

Mutation (one `Edit`; `if names != tools {` occurs once in the file, the Responses path uses `tool_names(parsed) != tools`):

```
-    if names != tools {
+    if false && names != tools {
```

Mutated sha256: `e00ab246dbaf2d4b817d54ef0a23e6209ccfa2d72d260c69f35da1b18e1621ae`.

Command: `cargo test --manifest-path /home/bline/cbm-work/wt-org-hosted-review-20261117/rust/Cargo.toml -p work-engine-review-hosted --offline --test check`

Result: exit 101; 26 passed, 1 failed. Failing test (only one): `claude_sdk_refuses_a_wrong_tool_set`, first case `extra tool`:
`assertion left == right failed: extra tool was not refused  left: "checks_passed"  right: "checks_failed"` (`tests/check.rs:698`). The wrong-model test still passes under this mutant, so M1 and M2 are killed by different tests.

Restore: inverse `Edit`; `sha256sum` of `src/check.rs` = `caf32cf8b73c98576aa6fc5ac3b0232dcb26787fa44991e36f8b3e721a917a15` (equal to baseline).

An earlier run of the same mutation, against the combined test `claude_sdk_refuses_wrong_model_and_a_wrong_tool_set` (since split), was also killed (same mutated hash `e00ab246…` not recorded then; the file is the same edit), and restored to `caf32cf8…`.
