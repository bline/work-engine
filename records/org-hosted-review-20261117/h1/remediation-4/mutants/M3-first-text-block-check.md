# Mutant M3: first-text-block type check disabled

File: `rust/crates/work-engine-review-hosted/src/check.rs`, `messages_body`, the selection of `messages[0]`'s first content block as a `text` block (line 432 at baseline).
Baseline sha256 (before and after): `caf32cf8b73c98576aa6fc5ac3b0232dcb26787fa44991e36f8b3e721a917a15`.
Tests file sha256 at the time: `69d77577195ab648b26c22555d6c86267e843c1baaf983ff5c968547b611d222`.

Mutation (one `Edit`; the line occurs once):

```
-                .filter(|b| b["type"] == "text")
+                .filter(|_| true)
```

Mutated sha256: `2e0b4fc3ffef8c97125ce3b4201d29e372868b0168183959b0fadc532a6da471`.

Command: `cargo test --manifest-path /home/bline/cbm-work/wt-org-hosted-review-20261117/rust/Cargo.toml -p work-engine-review-hosted --offline --test check`

Result: exit 101; 26 passed, 1 failed. Failing test (only one): `claude_sdk_refuses_a_task_that_is_not_the_first_text_block`, case `non-text block carrying the task bytes` (`[{"type":"document","text": <task + suffix>}]`):
`assertion left == right failed: non-text block carrying the task bytes was not refused  left: "checks_passed"  right: "checks_failed"` (`tests/check.rs:807`).

Note on what this mutant does and does not show: the other cases of that test (another text block first; image block first with the task second; empty content; null content) are still refused under this mutant, by the block-count check, the missing `text` string and the content-shape match. Only a non-text block that itself carries a `text` string equal to task + suffix distinguishes the type filter. That is the case the test pins.

Restore: inverse `Edit`; `sha256sum` of `src/check.rs` = `caf32cf8b73c98576aa6fc5ac3b0232dcb26787fa44991e36f8b3e721a917a15` (equal to baseline).
