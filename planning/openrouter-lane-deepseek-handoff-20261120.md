# Session handoff — OpenRouter/DeepSeek Pi Host lane (2026-11-20)

## What this session accomplished

Built and merged the **OpenRouter/DeepSeek Pi Host lane** — an `openai-completions`
wire family in `work-engine-manual-host` so the E1a builder battery can run a
DeepSeek V4 Flash arm through the Pi Host's credential custody and isolation.

**Merged + pushed:** `openrouter-lane-20261119` merged into
`rescue-detached-20260911` as commit `d687af6a`, pushed to origin
(`9ed92ba0..d687af6a`). Main repo `/home/bline/code/work-engine` is on rescue at
`d687af6a`.

**Merged test suite green:** 189 lib + integration = **303 passing, 0 failures**.

## State

- Branch `rescue-detached-20260911` @ `d687af6a` (main repo, pushed).
- Worktree `/home/bline/cbm-work/wt-we-openrouter-core-20261119` still on the old
  lane branch `openrouter-lane-20261119` @ `00327cee` — **can be removed**; all
  its work is merged. Shares build via `rust/.cargo/config.toml`
  (`target-dir=/home/bline/code/work-engine/rust/target`, git-excluded). Do NOT
  `git stash -u` (crate `target/` dirs are huge). `/tmp` is tiny.
- Plan: `planning/openrouter-lane-deepseek-20261119.md` (Revisions 1–10).
- Fable review: `reviews/openrouter-lane-fable-20261119/FINDINGS.md` (first, at
  31903c4b) and `FINDINGS-R2-FABLE.md` (completion check at fa0ac42c).

## What was built (the lane)

- `Wire::Completions` (`pi_stream.rs`): chat/completions SSE, terminal is literal
  `[DONE]`, usage from last usage-bearing chunk; error chunk = failure not
  completion.
- `Endpoint::OpenRouter` (`pi_coding_hosted.rs`): `openrouter.ai/api/v1/chat/completions`,
  `hosted_fetch` URL/policy validation, `completions_policy`
  (`store==false`, `max_completion_tokens` ≤ catalog bound when present,
  `reasoning.effort=="high"` required), `completions_forward_body` (Layer A→B
  rewrite).
- `pi_coding_provider.rs`: `provider "openrouter" → Wire::Completions`,
  `OPENROUTER_MODELS` allowlist, `transmitted_model()` (Layer B derivation),
  `completions_model_family`/`completions_model_in_family` (version-digit
  suffix check), budget ceilings (`OPENROUTER_CODING_BUDGET` $4 / `OPENROUTER_ANALYSIS_BUDGET` $20),
  `capability_ok`/`analysis_execution_ok`/`background_gate`.
- `pi_coding_https.rs`: OpenRouter credential custody (`OpenRouterApiKeyEnv/File`
  with **`sk-or-` prefix enforced**), `Authorization: Bearer` in `send`, screened
  from relayed bytes.
- `pi_coding.rs`: `tool_request` shell guard (scoped to **live bridges** only,
  so fixture bridges keep shell), `ready_tools`/`tools_match` threaded with
  `allow_shell`/`allow_background`, `BG_TOOLS` const.
- `receiver/pi-coding.mjs`: openrouter provider resolution offline, nonsecreating
  `setRuntimeApiKey('openrouter','nonsecret-marker')`, `thinkingLevel:'high'`,
  `workspace_exec` gating, background-plugin loading gated on `allow_background`.

## Capability authorization (operator decision, Revision 4/7)

Work Engine authorizes **shell and background at admission**: `allow_shell`,
`allow_background`, `allow_background => allow_shell`, invalid (no-shell+bg)
refused, analysis/relay admit no execution, and **`background_gate` refuses any
`allow_background=true`** until a workspace-bounded process host exists. The lane
is **foreground-only**; background is explicitly out of scope/deferred.

## Fable review status

- First review (31903c4b): CHANGES REQUIRED. All findings remediated.
- **Fable re-review (fa0ac42c): verdict OK with notes** for the foreground-only
  scope. No P0/P1. Fable captured the pinned Pi 1.5.0 client's real request
  offline and confirmed `reasoning.effort:"high"` is admitted.
- Loopback P2 note **fixed** in the merge (`(Loopback, Completions)` →
  `/chat/completions` + URL admission) so a provider-free e2e is possible.

## Remaining work (all gated on operator decision + billing authority)

1. **Live qualification** of the OpenRouter lane (real DeepSeek call through the
   Pi Host). ~$0.09 spent so far; operator monitors OpenRouter spend.
2. **E1a builder battery** on `deepseek/deepseek-v4-flash-latest:floor`, effort
   `high`. Operator: 1 run now, 3 if clean. `arm_run.py prepare` (baseline
   `1e21e71c`, packet, interface, repo-instructions at `~/cbm-work/e1a`); F3.1
   scorer `~/cbm-work/f3-battery-scorer/battery_score.py` counts-only.
3. **Live check of resolved model identity (Layer C)**: Fable noted that if
   OpenRouter returns `-latest` instead of a dated checkpoint, the Host refuses
   completion and retains the reservation. Verify the actual response during the
   first live run. Do not weaken the checkpoint rule on speculation.
4. **Background execution** stays deferred (workspace-bounded process host is
   future design; not this lane).

## Operational lessons (carry forward)

- **Fable model selection**: subagent launches have NO separate `provider` field —
  provider is the prefix of the `model` value (`provider/model-id`). The
  `reviewer` agent inherits the parent session's model if `model` is omitted.
  Always pass `model:"claude-sdk/claude-fable-5-1:xhigh"` explicitly. Two
  launches that omitted `model` silently ran DeepSeek instead.
- **Tool discipline**: serial or small batches only; never large parallel fan-outs
  of near-identical calls. For long runs (tests/builds), prefer the background
  task plugin (`bg_task_spawn`/`bg_task_watch`/`wait_for_operations`) — note it
  was unhealthy at times this session, so foreground was the fallback.
- **`git stash -u`** grabs ~80G of crate `target/` dirs; avoid it.
- This session earlier produced a 2h empty Fable run (launched with no task, then
  a provider stall at the tool-call boundary); the fix was to pass the task in the
  launch and, on a no-activity alarm on a `claude-sdk` child, inspect the child
  session for a stalled tool-call boundary and resume rather than wait.