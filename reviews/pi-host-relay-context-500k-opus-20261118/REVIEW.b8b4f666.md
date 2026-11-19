# Review: Pi Host relay context bound 500,000 (RL 5947), `git diff f772612f 9b63e454`

Reviewer: Opus, read-only adversarial review. Commits `24a77f7a` (records only:
serve.py r3 options and README r11/r12) and `9b63e454` (`RELAY_MAX_CONTEXT_BYTES`,
`context_bound`, test, serve.py `PI_HOST_MAX_CONTEXT`/`PI_HOST_PAD_BYTES`, record r13-r16).

## Verdict: accept with changes

The code change is correct and complete for the lifecycle relay profile. Every
relay job is admitted through `ProviderPlan::validate` with `relay == true`. No
other per-request check compares a relay request's context with 400,000.
Requests between 400,000 and 500,000 are admitted by the Host, the receiver, the
relay gate, the runner and the store. Past 500,000 the job fails closed with no
dispatch. Unit tests pass. The offline measurements in the record match the
retained run artifacts byte for byte.

There are no P1 or P2 findings. The requested changes are P3 corrections to the
record and docs. One stated rationale is wrong (P3-1) and one phrase about r16 is
inaccurate (P3-2). The record also leaves out a few limits that this change brings
closer (P3-3, P3-4). None of them blocks the live H2.

## Findings

### P3-1: The reason given for keeping analysis at 400,000 does not hold

- **Where.** `records/dev-vertical-pi-host-realization-20261118/README.md:550-554`
  says: "That bound exists because the analysis gate's `pi.provider_request` ingress,
  carrying the context twice, must fit the store's 1 MiB bound, and the relay gate
  stores no provider frame." The new doc comment
  `rust/crates/work-engine-manual-host/src/pi_coding_analysis.rs:17-20` implies
  the same contrast.
- **Evidence: the ingress is never larger than the frame.** The analysis gate
  records the container frame itself. It clones the frame, renames `type` to
  `pi.provider_request` (shorter than `coding_provider_request`) and removes
  `schema` and `host_workspace` (`rust/crates/pi-pressure-context-host/src/analysis.rs:223-230`).
  The history record is built the same way (`:255-261`). Each record is therefore
  no larger than the container line it came from.
- **Evidence: both bounds are 1 MiB.** The store refuses `raw.len() > 1_048_576`
  (`rust/crates/lifecycle-store/src/native.rs:2647`). That is the same number as
  `FRAME_BYTES = 1 << 20` (`pi_coding_analysis.rs:24`), which both profiles already
  meet at the receiver (`receiver/pi-coding.mjs:75`) and at the Host
  (`src/pi_coding.rs:1100-1107`).
- **Consequence.** At 500,000 an analysis `pi.provider_request` would be about
  1,000,480 B, which fits exactly as the relay's 1,000,502 B frame does. The store
  bound adds nothing that separates the two profiles.
- **Keeping analysis at 400,000 is still right**, for scope reasons. RL 5947 is about
  the live H2 (relay). `work-engine-review-hosted/src/subject.rs:22` mirrors 400,000
  in its `check_context_fit`.
- **Smallest fix.** Reword the README bullet and the doc comment. For example:
  "Analysis keeps 400,000 (RL 5947 covers the relay for the live H2;
  `work-engine-review-hosted` mirrors the analysis value). Its gate's ingress
  records are no larger than the container frame, so the same frame arithmetic
  would govern a later raise."

### P3-2: r16 is described as refused "before any frame", but frames followed the refusal

- **Where.** README `r16-over-cap` row (`:574`) says: "refused by the receiver's
  `contextBytes` bound before any frame or dispatch".
- **Evidence: frames emitted after the refusal.** From
  `.../serve/r16-over-cap/state/0002-rpc-0.stdout`, frame indices 141-147:
  - `turn_start`;
  - `message_start` and `message_end` (assistant, `stopReason` `aborted`,
    `errorMessage` "outside host admission unavailable");
  - `turn_end`;
  - `agent_end` (505,106 B);
  - `agent_settled`;
  - finally `{"type":"error","message":"Pi coding profile failed"}`.
- **Evidence: what was never emitted.** There is no third `coding_provider_request`,
  grant or `provider_fetch`. The ledger has requests 1-2 only. The serve record's
  attempt log has 3 intents (1/1, 1/2, 2/1). The observer has 3 receipts
  (`1-1` refused 429, `1-2`, `2-1`).
- **Smallest fix.** Write "before any provider request frame, grant or dispatch".
  Also cite the aborted message's `errorMessage` as the evidence. The receiver's
  admission test at `pi-coding.mjs:308` has seven disjuncts. Only `contextBytes`
  applies here: id 3 is within 32, it is the same model, it is far from the
  deadline, and a turn is active. The `agent_end` size (505,106 B) agrees with
  "about 505,000".

### P3-3: The answer margin is understated, and the record does not say this change creates it

- **Where.** README `:582-585` says: "near the bound a final answer above about
  19 KB makes the receiver's history frame exceed 1 MiB".
- **Measurement (r14).**
  - The `coding_history` line is 995,036 B at last `contextBytes` 494,902.
  - The final assistant message is 562 B (45 B of text), and the history frame
    carries the text twice.
  - So H ≈ 2C + 5,232 + 2·(extra answer bytes).
  - With the receiver and Host line bound (≤ 1,048,576 including the newline;
    `pi_docker.rs:1481-1483` keeps the newline in the Host's line), the extra
    plain-ASCII answer allowed is about 21,671 B at C = 500,000.
  - It is about 31,671 B at 490,000, 41,671 B at 480,000 and 65,671 B at 456,000.
  - JSON escapes, and thinking blocks in the final message, use the same margin
    twice.
- **What changed.** Above C ≈ 456,000 the history frame, not the protocol's
  64 KiB answer bound, decides first. The answer bound is
  `native-realization/src/lib.rs:32`, enforced at `pi_host_serve.rs:622-628`.
  At the old 400,000 cap, every answer within 64 KiB fit (about 120 KB of margin).
  So this diff makes a ≤ 64 KiB answer fail as "unknown coding frame" instead of
  the named "final answer exceeds its bound".
- **Why only P3.** It fails closed. The offline H2 treatment context is about 347k
  (r11).
- **Smallest fix.** In the record, replace "about 19 KB" with "about 21 KB of plain
  text at 500,000 (less with escapes or thinking)". Add the crossover: above about
  456,000 of context, the 64 KiB protocol answer bound is no longer reachable.

### P3-4: Per-turn and per-job limits that grow with context, not mentioned in the record

README `:580` says "No other bound refuses a context below 500,000 first". That is
true per request. The record does not mention two cumulative limits that the
higher cap reaches sooner. Both are enforced only after spending, and both fail
closed.

- **8 MiB of retained request bodies per turn.**
  - This bound is checked only after the turn: as a capture reason
    (`pi_host_serve.rs:1020`), in the runner (`lifecycle-service/src/operations/pi_host.rs:480`,
    which returns `Rejected`) and in the store (`lifecycle-store/src/native_input.rs:113`).
  - Before dispatch, `provider_attempt` checks only the send count
    (`pi_host_serve.rs:1147`).
  - Near the bound a body is about 492-498 KB (r14: 492,859). So 16 near-bound
    requests fit, and a 17th makes the turn ineligible after it has been paid for.
    At the 400,000 cap the figure was about 21.
  - With the H2's `max_requests` 32, this is reachable if the live model keeps
    requesting after it has read everything.
- **64 MiB of container stdout per job.**
  - The bound is `OUTPUT_BYTES` (`pi_coding_analysis.rs:26`), enforced as
    "process output exceeded bound" at `pi_docker.rs:671-682`.
  - A near-bound request costs about 1,000,500 B (request frame) plus about
    664 KB (`provider_fetch`).
  - A single turn of 32 near-bound requests, plus tool events, `agent_end` and
    history, comes to about 57 MB. That fits under 67.1 MB.
  - A multi-turn relay thread near the bound (up to 16 turns; each adds about
    1.5 MB for `agent_end` plus history) reaches about 79 MB, over the bound.
    At 400,000 the same thread was about 63 MB.
- **Smallest fix.** Add one sentence to the record naming these two limits and the
  per-turn and per-thread request counts at which they bite. No code change is
  needed for the live H2.

### P3-5: Line citations in another crate went stale because of this diff

- **Where.** `rust/crates/work-engine-review-hosted/src/subject.rs`:
  - `:16` cites `pi_coding_analysis.rs:23-24` for `SNAPSHOT_BYTES`/`SNAPSHOT_FILES`.
    These are now at `:28-29`.
  - `:322` cites `:22` for `PAGE_BYTES`, now at `:27`.
- Both citations were correct at `f772612f`. The five new lines (17-21) moved the
  constants. `:16` → `pi_coding_analysis.rs:16` (`MAX_CONTEXT_BYTES`) is still
  correct.
- **Smallest fix.** Update the two citations, or cite the symbols. This is a note
  for the review-hosted owner: the change's claim limits its writes to the manual
  host and the records.

### P3-6: Small serve.py points (24a77f7a, records only)

- Runs from the r3 manifest still request snapshot id `snapshot:h2-treatment-r2`
  (`serve.py:60`, `:121`). The r13-r16 reads show it.
- The default treatment observer's final text changed from "treatment members" to
  "snapshot members" (`serve.py:129`). A rerun of `r10` with the committed script
  is therefore not byte-identical to `r10`. The default shape is otherwise
  unchanged.
- The pin checks are `assert`s (`serve.py:88`, `:105`, `:109`), so they are disabled
  under `python3 -O` or `PYTHONOPTIMIZE`. This follows the file's existing
  convention.
- Optional: label the snapshot by manifest, and make the pin checks explicit
  `sys.exit` refusals.

### P3-7: Admission coverage is only at the helper level

- `context_bound_is_per_profile` (`pi_coding_provider.rs:1387-1401`) tests the
  helper and the constant relationships. No unit test goes through
  `ProviderPlan::validate`: relay admitted at 500,000, refused at 500,001;
  analysis refused at 400,001.
- r13/r14 (admitted at 500,000) and r15 (refused at 500,001) cover the relay end to
  end offline.
- Optional: if the admission fixture at `:1300-1380` can take
  `LIFECYCLE_PROFILE` and `claude-sdk` cheaply, add the three cases.

## Answers to the questions, and what I checked and found sound

### 1. Is relay-only scoping correct and complete? Yes

**Admission.**
- `Host::admit` → `check_job` → `profile.validate(job)` (`pi_docker.rs:235`, `:272-273`)
  → `plan.validate(job)` (`pi_coding.rs:461`).
- A relay schema without a provider fails the schema test at `pi_coding.rs:404-421`,
  so every relay job reaches `ProviderPlan::validate`.
- There, `relay = is_relay(schema)` (`pi_coding_provider.rs:540-543`), which
  implies `analysis` (`pi_coding.rs:56-61`).
- The bound `1024..=context_bound(analysis, relay)` (`:566-567`) gives 500,000 for
  relay, 400,000 for analysis and 32,768 otherwise (`:642-650`).
  `context_bound(false, true)` is unreachable.
- `coding_prepare_provider` validates again (`:817`).

**Every other comparison follows the plan value or does not exist.**
- **Receiver.**
  - `config.maxContextBytes` is the plan value written into `job.json`
    (`pi_coding_provider.rs:839`) and checked per request at `pi-coding.mjs:307-308`.
  - `MAX_BODY` is `config.maxBodyBytes` = 600,000 (`pi-coding.mjs:30`;
    `pi_coding_provider.rs:844`).
  - `FRAME_BYTES` is `config.frameBytes` = 1 MiB (`pi-coding.mjs:29`; `:845`).
- **Host frame reading.** `coding_advance_lines` uses `analysis::FRAME_BYTES` for
  every analysis-family profile, relay included (`pi_coding.rs:1100-1107`).
- **Per-request Host check.** `coding_provider_request` is mapped to
  `provider_request` with `contextBytes` (`pi_coding.rs:1560-1571`) and compared
  with `plan.max_context_bytes` (`pi_coding_provider.rs:887-891`).
- **Relay gate.** `Controller::frame` / `provider` compares no context and stores
  no provider frame (`pi_host_serve.rs:556-567`, `:1086-1103`).
- **Runner.** `operations/pi_host.rs` checks the job by digest. Its 1 MiB is the
  frame bound on Host JSON-RPC output (`:26`, `:615`), which never carries the
  context. `thread/read` is held at 1,000,000 by the Host (`pi_host_serve.rs:670`).
- **Store.** The raw ingress bound of 1 MiB (`native.rs:2647`) applies only to
  runner-recorded Host frames. Native input: bodies total at most 8 MiB, metadata
  at most 128 KiB (`native_input.rs:107-115`). The metadata carries digests only
  (`pi_host_serve.rs:1028`; `pi_host.rs:530-539`).
- **Other hits.** These are other profiles, not the relay:
  - `pi_coding.rs:466` (continuity, 24,000);
  - `work-engine-server/src/job.rs:219` (24,000);
  - `work-engine-review-hosted/src/subject.rs:22` (analysis consumer);
  - `lifecycle-service/tests/pi_host_join.rs:147` (offline default 400,000,
    untouched as instructed).
- **Protocol.** `PI_HOST_PROTOCOL.md` states no context bound, so no protocol or
  `PROTOCOL_DIGEST` change is needed.
- **Live arm.** Nothing in the repository sets the live arm's
  `max_context_bytes`. The arm file's `JobParams` must carry 500,000
  (`pi_host_join.rs:128-136`).

### 2. Is keeping analysis at 400,000 justified?

Yes as scope, no as stated (P3-1). The only other coupling is
`work-engine-review-hosted`'s own copy at `subject.rs:22`. It is consistent
because analysis is unchanged.

### 3. Frame arithmetic, measured from the r13 and r14 container stdout

Largest line per frame type, with the 1,048,576 B bound:

| Frame | r13 (C 490,314) | r14 (C 494,902) | At C = 500,000 |
| --- | --- | --- | --- |
| `coding_history` (2× final context + 2× answer) | 985,860 | 995,036 | ≈ 1,005,232 (≈ 43 KB margin) |
| `coding_provider_request` (`messages` + `agentMessages`) | 981,130 | 990,306 | ≈ 1,000,502 (48,073 margin) |
| `provider_fetch` (base64 of body + headers) | 652,552 | 658,673 | ≤ ≈ 801.5 KB even at `MAX_BODY` 600,000 |
| `native_event` `agent_end` (1× the turn's new messages) | 491,048 | 495,636 | ≈ C (r16: 505,106) |
| `native_event` `turn_end` (one tool batch) | 271,044 | 271,044 | not context-bound |

- **Request frame.** The record's "about 1,000,500 (48 KB below 1 MiB)" is right.
  The frame is 2C + 502 in r14.
- **Answer margin.** "Above about 19 KB fails closed" is conservative. See P3-3.
- **`provider_fetch`.** This frame does not bind first. The 600,000 body bound
  binds first, in the forwarder (`pi-coding.mjs:132`: 413, counted as
  `forwarderRefused`) and in the Host (`pi_coding_hosted.rs:726-735`). Near the cap
  the body is just under the context (r14: 492,859 vs 494,902).
- **`agent_end`.** This is a third frame that grows with context. The record does
  not mention it, but it carries the context once and is not binding.
- **Relayed results above `FRAME_BYTES - 1024`.** The Host refuses them
  (`pi_coding_analysis.rs:378-379`); the job ends and `Controller::finish` fails
  the turn (`pi_host_serve.rs:296-320`).
  - The receiver's input-line bound of 131,072 (`pi-coding.mjs:159`) is stricter.
    Any `snapshot_result` over 128 KiB already fails closed there ("host protocol
    rejected", `:176`).
  - Neither bound depends on the context cap. H2 pages are 32 KiB (largest
    `tool_execution_end` 44,921 B).
- **Line bounds agree.** Receiver and Host apply the same bound to lines including
  the newline (`pi-coding.mjs:73-75`; `pi_docker.rs:1481-1483` with
  `pi_coding.rs:1106`).

### 4. Does r16 show the receiver's bound refusing first? Yes

- The aborted `message_end` carries "outside host admission unavailable"
  (`pi-coding.mjs:308`, `failure()` at `:298-303`).
- `liveBridgeFailed` makes the prompt loop throw (`:454`). The outer catch emits
  the generic `error` frame (`:479`).
- The Host refuses that frame as "unknown coding frame" (`pi_coding.rs:1706`).
  `Controller::finish` then fails the turn with "the Host job ended during the
  turn: unknown coding frame".
- The capture is ineligible ("Pi's history for the turn is absent"), there is no
  third grant or attempt (above), and `containers_after` is `[]`.
- The record's description of the failure reason is accurate: truthful, but
  generic, and older than this change. Its "before any frame" wording is not
  (P3-2).
- r16's `failures` list also has later follow-on entries ("process
  deadline/output/input failure", "native relayed completion mismatch"), and the
  run took 140.6 s against 87.3 s for r14. All are fail-closed.

### 5. serve.py

- **Measurement-only, defaults unchanged.** `PI_HOST_PAD_BYTES` defaults to 0 (no
  member). `PI_HOST_MAX_CONTEXT` defaults to `400_000`. Neither affects the default
  path except the final-text note in P3-6.
- **Padding.** Plain ASCII without escapes. The member totals in the record add up:
  270,633 + 140,000, + 144,586 and + 155,352.
- **A4 pin.** It refuses wrong bytes: the sha256 and the byte count must both match
  the manifest entry, otherwise the script aborts before `prepare()`
  (`serve.py:83-91`). It is assert-based (P3-6). An r3 manifest without
  `PI_HOST_A4_MEMBER` raises `KeyError`, which also refuses.
- **No draft A4 bytes are committed.** The range touches only four files.
  `git grep` at `9b63e454` finds only the digest prefix, the size and the
  manifest key.
- **Manifest drift.** The S2 r3 manifest at its default path has changed since the
  runs: it is now `d6a0bf71…`, while the record names `9fcaa325…`. The A4 entry is
  unchanged (144,537 B, `bdc8c2ef…`). Each `summary.json` records every member's
  sha256, so the snapshot is still reproducible.

### 6. Other safety points for the live H2

- **Budget.** The per-request reservation is the catalog worst case (pi-ai
  `anthropic.json`, `claude-haiku-5-5`): a context window of 1,000,000 tokens at the
  highest input rate plus 128,000 output tokens (`pi_coding_provider.rs:496-502`,
  `:770-795`). It does not depend on `max_context_bytes`, and 500,000 B is well
  under 250k tokens. The budget gate is at `:895-906`. `MESSAGES_ANALYSIS_BUDGET`
  43,000,000 ≥ 1,320,001 × 32 (`:26`, `:559`).
- **Container memory.** `memory.peak`: 278 MiB (r13), 273 MiB (r14), 272 MiB (r16)
  of 512 MiB, with no `oom_kill`. r11 at a smaller context peaked at 328 MiB, so
  context size is not the main driver offline. Live thinking adds some.
- **Host memory.** The Host reads only the new stdout tail
  (`pi_coding.rs:1090-1097`).
- **Cumulative limits.** See P3-4.
- **Context window and thinking.** `contextBytes` is
  `JSON.stringify(context)` (`pi-coding.mjs:307`), so live thinking blocks count,
  as the record says.

### Record numbers verified against the artifacts

The `summary.json` sha256 values match the record's prefixes:

| Run | sha256 prefix |
| --- | --- |
| r11 | `6b1cbef7` |
| r12 | `013ea538` |
| r13 | `676412a4` |
| r14 | `876cfd0e` |
| r15 | `ec74e2cb` |
| r16 | `32c86519` |

- The table values match the summaries: `contextBytes`, request and history frames,
  bodies, `thread/read`, `memory.peak`, eligibility and containers left.
- r15's job had 500,001 and was refused at `thread/start`. r13, r14 and r16 jobs
  had 500,000.
- The receiver in every run's `inputs/pi-coding.mjs` is byte-identical to the
  repository receiver (`9cbff00f…`).

## Commands run (all read-only for the repository)

- `git log`, `git diff f772612f 9b63e454` (stat, name-status, per path), `git show`
  of base files, and `git grep` at `9b63e454`.
- The board read (`codex_chatboard.py read --active-claims`). The author's claim
  covers this branch; no conflict.
- `CARGO_TARGET_DIR=/home/bline/.cache/opus-ctx500k-target cargo test -p work-engine-manual-host --lib context_bound`:
  1 passed.
- `CARGO_TARGET_DIR=/home/bline/.cache/opus-ctx500k-target cargo test -p work-engine-manual-host --lib`:
  109 passed, 0 failed, 6 ignored. One warning, an unused import `PiOwner` at
  `src/pi_native.rs:6`, which is outside the diff and predates it.
- `python3 -I` read-only parsing of the r11-r16 `summary.json` files, of the r13,
  r14 and r16 `state/0002-rpc-0.stdout` frame sizes and kinds, and of the serve
  records. `sha256sum` of the summaries, the receiver copies and the S2 r3 manifest.

No Docker, no serve.py, no Q tests, no inference, and no use of
`WORK_ENGINE_HOSTED_CREDENTIALS`. No repository file was modified except this
review file.
