# Context status CLI

`work-engine-context-status` is a read-only view of one explicitly named Codex rollout. It helps an operator decide whether a logical task boundary is a sensible time to prepare a successor. It neither owns a context transition nor tells an agent to compact or hand off.

Build and run from `rust/`:

```sh
cargo run --locked -p work-engine-context-status -- --session 01a15f02-a935-7683-a849-9e919aecfe3c --json
cargo run --locked -p work-engine-context-status -- --session 01a15f02-a935-7683-a849-9e919aecfe3c --rollout /absolute/path/to/rollout-01a15f02-a935-7683-a849-9e919aecfe3c.jsonl
```

`--session` is mandatory. Without `--rollout`, the CLI searches `--sessions-dir` or `$CODEX_HOME/sessions` (falling back to `$HOME/.codex/sessions`) for filenames ending in the exact ID. It rejects zero or multiple candidates and verifies the first record's `session_meta.id` as the requested thread ID. An optional `session_meta.session_id` is validated as a separate recorded root-session lineage value; a child can correctly have a different thread ID and root session ID. It never infers an ID from the current process, working directory, or newest file. The search skips symlinks, limits directory depth to five and entries to 20,000, and reads at most a 256 MiB regular file with 8 MiB lines. A larger input fails explicitly. Output contains file location, recorded root-session lineage, timestamps, model/effort identifiers, counts and limitations; it does not print prompt bodies, tool arguments, session metadata, credentials or arbitrary event payloads.

Forked rollouts can contain copied parent metadata and telemetry. For the supported ordinal format, the first record's `forked_from_id` identifies the source and its positive `subagent_history_start_ordinal` marks the child's own history. The CLI requires contiguous zero-based record ordinals and excludes records between the header and that boundary from usage, model/effort, growth and compaction observations. Copied metadata must follow the declared fork ancestry, with no duplicate identities or conflicting recorded root session. Additional metadata in the child's own suffix still fails identity validation. Explicit thread IDs on suffix usage records and settings events must match the child. Rewritten timestamps do not establish attribution. Missing or invalid fork boundaries, malformed complete fork records and ordinal gaps fail explicitly; a partial trailing record is disclosed and ignored. A prefix-only file reports unavailable child usage rather than borrowing the parent's counters. JSON exposes `forked_from_id`, `own_history_start_ordinal` and `inherited_records_skipped`; inherited exclusions are also disclosed in text output. This uses recorded file provenance, not independently attested runtime identity. Non-fork rollouts still require exactly one matching metadata record.

To discover child thread IDs from recorded metadata, use:

```sh
cargo run --locked -p work-engine-context-status -- agents --parent-thread 01a15f02-a935-7683-a849-9e919aecfe3c --json
cargo run --locked -p work-engine-context-status -- agents --parent-thread 01a15f02-a935-7683-a849-9e919aecfe3c --recursive --sessions-dir /absolute/path/to/sessions
```

`agents` scans each `.jsonl` candidate once and reads only its first `session_meta` record, capped at 8 MiB. It uses recorded parent thread IDs, not filenames or recency, to list direct children or cycle-safe recursive descendants. It reconciles top-level and `source.subagent.thread_spawn` parent and agent-path fields; conflicting representations are skipped with a diagnostic. Results show each child's thread ID, recorded root-session ID, parent thread, agent path and rollout path. Duplicate thread IDs are checked across all scanned first records, including rows outside the requested subtree. Relevant conflicting rows remain visible, and recursive discovery stops at an ambiguous ID. Cyclic parent edges are reported without returning the requested parent as its own child. Agent paths shared by multiple historical threads are reported as ambiguous. The command never selects a candidate by recency. It caps directory traversal at five levels and 20,000 entries, results at 1,000, and retained diagnostics at 16. The report counts skipped unreadable or malformed metadata and marks its completeness as partial or ambiguous for skipped records, duplicate IDs, cycles or shared agent paths. `agents --help` and top-level `--help` show command usage; value-taking options require explicit values. First-line parsing never prints base instructions, account fields or transcript contents. Use a returned exact thread ID with the normal `--session` command to inspect that child's context.

The latest input and cache counts come from `event_msg.token_count.info.last_token_usage`. A zero-input reset snapshot is unavailable even when its `total_tokens` is positive; a valid smaller post-reset request remains usable. `total_token_usage` is cumulative accounting and is never used as current context. A valid positive `info.model_context_window` on the same usage observation supplies the denominator. If the latest usage has no safe positive window, the fraction and headroom are unavailable even when an earlier observation had one. The fraction (`input_tokens / model_context_window`) and saturated headroom are **proxies**: the input counter is a request observation, not a live context allocation measurement. File modification age is a freshness signal, not proof the session is still active. JSON records source line and event timestamp; if a later malformed, partial or invalid usage record exists, `latest_request_status` is `historical_observation`; current fraction, headroom, growth and economics are unavailable until a valid request follows. Missing counters remain unavailable. Cache counts greater than input and integers outside `u64` are rejected. A trailing partial line is ignored and disclosed. The report retains at most 16 diagnostic examples and includes per-category counts plus an omitted count so damaged rollouts cannot flood output. Compaction markers, model changes, window changes and decreasing input observations start a new recent-sample epoch; recent growth is the difference between endpoints of at most eight distinct usage snapshots in that epoch, never a sum of repeated snapshots. Emission timestamps do not establish new request identity: repeated last-usage and cumulative-counter snapshots retain the first observation time. Missing cumulative fields are unknown, not evidence of a new request; known values are retained across missing fields, and a conflicting known value can distinguish one. Cumulative counters never supply the context numerator. A missing denominator splits recent growth epochs, and the last valid window is retained only to detect a later observed window change; it cannot supply the current proxy. A model transition without a newly recorded effort leaves effort unknown. These resets are conservative and do not prove the provider's internal context state.

Optional economics require **all** of `--fresh-input-tokens`, `--preparation-tokens`, `--launch-tokens`, `--rehydration-tokens`, and `--reconstruction-tokens`. Each value must be an estimate in the same declared weighted-input-token units. `--fresh-cache-fraction` defaults to zero and `--cached-weight` defaults to 0.1; both are explicit scenario assumptions, not provider prices. `--remaining-calls N` projects N calls under fixed per-call input/cache assumptions. Continuing uses the measured latest input/cache; the fresh case uses the supplied fresh input estimate and fixed overhead. Break-even is the first integer call count where `N × fresh weighted input + all overhead` is strictly less than `N × continuing weighted input`. It is unavailable when there is no measured continuing request, an overhead overflow, or no positive per-call saving. This says nothing about dollars, plan quota, provider billing, output tokens, or a universal context threshold.

At a logical task boundary, an operator can inspect this report and, if a successor is warranted, write a durable handoff with its exact path and SHA-256. The worker reports `handoff_ready` and the artifact identity to its parent. The parent verifies that artifact and creates the successor; the successor rechecks it and explicitly takes ownership. This CLI supplies evidence for that judgment but grants no authority or automatic handoff.

## Manual handoff control observations

`control --session ID --rollout /absolute/FILE --trigger-tokens N
--band-floor-tokens N` emits a compact JSON assessment for the
[chatboard control subscription](codex-chatboard.md#session-control-subscription).
`--host claude` uses Claude's reader and `--agent ID` binds a Claude child.
Both session and agent identity must match the transcript. Levels are explicit
caller-selected hypotheses, with `0 < trigger < band floor`; they do not change
the underlying telemetry or imply a provider limit.

The output has `schema: manual-context-control-v1`, `topic`, optional `event`,
and an opaque `next_control_cursor`. Passing the previous cursor as `--cursor`
suppresses unchanged levels while retaining epoch/model/window/quality changes.
A normal initial reading is quiet. Errors reading the bound source produce an
`unavailable` event; invalid arguments or incompatible cursors fail the command.
The cursor binds host, session, agent, source path and policy. Nothing is written
to the board, no session is started or reset, and no workflow ownership moves.

`--lead-calls N` (default 3, 0 disables forecast alerts) compares headroom to
recent positive growth across at least three same-epoch observations after the
last record gap. Current gaps and stale file modification suppress forecasts. The estimate is
requests to a selected empirical band, not time to a guaranteed compaction.
The original full report remains available for investigation.

Control observations distinguish historical gaps from current uncertainty. A gap
at or after the last valid usage yields `unavailable`; later valid readings
restore pressure alerts. Forecasting uses only the same-epoch samples after the
last gap. Historical gap diagnostics remain visible, and the existing report
commands label retained readings `historical_observation` while a gap at or
after that reading leaves current pressure unknown. This includes a Codex
snapshot with valid input counters but no valid window on that same record:
the counters remain visible, while current proxies and economics are unavailable.
After valid telemetry resumes they report
`latest_valid_observation_with_record_gaps`; growth uses only the valid suffix.
Identity mismatches yield an `unavailable` event with `unavailable_detail`; they
require binding repair, not an assumption that waiting will resolve them.

## Claude Code sessions

The `claude` and `claude-agents` subcommands give the same read-only view for Claude Code transcripts. Existing Codex report output is unchanged; each subcommand has its own `--help`.

```sh
cargo run --locked -p work-engine-context-status -- claude --transcript ~/.claude/projects/<project>/<session>/subagents/agent-<id>.jsonl --json
cargo run --locked -p work-engine-context-status -- claude --session <session-id> [--agent <agent-id>] [--projects-dir DIR]
cargo run --locked -p work-engine-context-status -- claude-agents --session <session-id> [--with-usage] [--json]
```

`--session` resolves `<project>/<session>.jsonl`, or with `--agent` `<project>/<session>/subagents/agent-<id>.jsonl`, under `--projects-dir` (default `$HOME/.claude/projects`), and rejects zero or multiple matches. Every record's `sessionId` and `agentId` must agree with the requested identity, or with the file name for `--transcript`; a conflict fails the report.

**Measured.** Each API call is one `message.id`; the several assistant records Claude Code writes for one response count once. Its input and cache counters come from the first record, and its output is the largest `output_tokens` among its records, since an early record can hold a streaming partial. A repeat whose input or cache counters differ keeps the first record and is diagnosed. A call's context is `input + cache_creation + cache_read`. The report shows the latest call with its line and timestamp, all distinct observed models and the latest one, the latest recorded `effort` (cleared by a model change that records none), recent growth as the endpoint difference of at most eight calls in the current epoch, and lineage. A compaction (`compactMetadata` or `subtype: "compact_boundary"`, with trigger, `preTokens`, `postTokens` and line) or a model change starts a new epoch and clears the latest call until a new one appears. Cumulative accounting covers every distinct call across epochs: uncached (`input + cache_creation`), cached (`cache_read`), output and the baseline's weighted unit (`uncached + cached/10 + 5 x output`). Records with model `<synthetic>` are not API calls and are excluded from accounting with a diagnostic; an own-history synthetic record leaves current pressure unavailable until a valid call follows. All three input counters must be nonnegative integers with a positive sum representable as u64. Cache-only input remains valid. An older valid call is labeled `historical_observation` after an invalid or synthetic record, including in agent listings; it does not supply current pressure or growth. A fork transcript starts with a `fork-context-ref` record followed by a copy of the parent's spawning call (assistant record(s) with a null `parentUuid`). The reference becomes a `fork` lineage event with `parentSessionId` and the recorded `contextLength`, whose unit is not established. The copied call(s) are reported under `inherited` and excluded from `cumulative`, the latest call, models and effort, so a fork with no call of its own reports its latest call as unavailable and summing parent and fork totals does not count the copied call twice. File caps, freshness and diagnostics match the Codex path; a trailing partial line is disclosed and ignored.

**Not measured.** Transcripts record no context window, so there is no fraction, headroom or window and the report says so in its limitations. A non-synthetic assistant call without usable input/cache counters (missing, invalid, zero-sum or overflowing) is counted as `calls_without_usage`, excluded from measured distinct-call totals, and unavailable, never zero. A call without `output_tokens` is excluded from output and weighted totals and counted. The weighted unit is a baseline accounting unit, not a price or quota measurement.

`claude-agents` lists `agent-<id>.jsonl` and `agent-<id>.meta.json` in the session's `subagents/` directory, with agent ID, type, the meta `description` (the Agent tool call's short description input, shown bounded to 256 characters), requested model and effort, `isFork`, spawn depth, tool-use ID, `finished` and transcript path. `--with-usage` adds each agent's latest call, epoch, compaction count, cumulative accounting and, for a fork, its inherited call count. The listing is `partial` when an agent's meta is `missing` or `unreadable`, its transcript is missing (`missing_transcript`) or unreadable (`unreadable_transcript`), or an agent-named entry is a symlink or not a regular file (`skipped_agent_entries`). An agent with only a historical reading is `historical_observation`. An agent whose transcript is readable but has no valid usage is `absent_counters`; its row is complete, so the listing stays complete. No failing agent removes the others. Both subcommands skip symlinked project directories and require real `<session>` and `subagents` directories. Neither prints message content, tool inputs other than that description, or prompt bodies.

`metrics/scripts/claude_transcript_usage.py` deduplicates on `(message.id, timestamp)`. Repeated records of one response carry different timestamps, so that script counts each record: on one 2026-10-21 reviewer transcript it reported 78 calls and 1,690,583 weighted tokens where `message.id` deduplication gives 42 calls and 867,108.5. Because it counts every record, it also sums partial and final outputs of one call, and it counts a fork's copied parent call.
