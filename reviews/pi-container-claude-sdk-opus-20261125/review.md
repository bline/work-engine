# Review: Claude subscription (claude-sdk) lane for Python Pi containers, S0–S2

Reviewer: Opus, read-only diff review. Base `70d2e49c`; uncommitted tracked changes in
`skills/manual-orchestration` and `metrics/scripts`, plus the untracked tests and fake.
Requirements: `planning/pi-container-claude-sdk-implementation-plan-20261124.md` (S0–S2),
`planning/pi-container-claude-sdk-p7-addendum-20261125.md`, and the S0 record
`records/pi-container-claude-sdk-s0-20261124/README.md`. S3 is out of scope. I made no live or
network calls and read no credentials. I ran no Docker tests. The only file I wrote is this one.

## Review

### Correct (with evidence)

- **Credential custody on the host.** `Pi.claude_credential` (`role_host.py:1226`) opens with
  `O_NOFOLLOW` and checks `S_ISREG`, owner uid, mode `& 0o077 == 0` and size ≤ 64 KiB. It then
  checks expiry > now + 120 s and the `sk-ant-oat01-` shape. The placeholder is refused
  (`pi_container.py:255`). It never refreshes or writes, and error text never contains the token
  (covered by `LaneOptionTests.test_credential_custody_without_refresh`). The token goes from
  `provider_key` to the runner's stdin only (`role_host.py` `_spawn_container`). The spec, intent,
  ledger row, `container_env` and fingerprint contain no token. The runner's `ValueError`
  messages from `claude_oauth_token` carry no token.
- **Container credential.** `container_env` (`pi_container.py:481`) adds only the relay base URL,
  the Claude path, `PI_CLAUDE_SDK_PROMPT=claude`, the synthetic-credential switch and the retry
  and fallback switches. No `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` or `CLAUDE_CODE_OAUTH_TOKEN`
  is set. The entry writes only the public placeholder into the per-turn tmpfs `HOME` with
  `O_EXCL`, mode 0600 (`pi_container_entry.py:32`, and `run_profile` mounts `--tmpfs {home}`).
- **Reserved mounts.** The credential file, its parent, `~/.claude`, `~/.claude.json` and the
  bundle root are reserved at launch and on delivery. `mount_plan`'s reserved check uses realpath
  and refuses overlap in both directions (`pi_container.py:360-363`), so `~/.pi`, `~/.pi/agent`
  (which holds Pi's own `auth.json`) and `~` are also refused as ancestors of the bundle root.
- **Extension and Claude Code configuration.** Pi runs with `--no-extensions` plus only the
  pinned `-e` paths (`role_host.py:1427`). Pi 1.5.2 then loads CLI extensions only
  (`resource-loader.js:426`). The pinned extension starts Claude Code with `settingSources: []`
  (`pi-claude-subscription/src/session.ts:143`), so the workspace's `.claude/settings.json` is not
  read. A project-level `.pi/claude-sdk.json` can set only `maxSessions` and `idleMinutes`
  (`config.ts:30-39`). The extension's `childEnv` clears API-key and cloud switches.
- **Agent-dir rewrite.** `write_claude_sdk_agent_config` (`role_host.py:874`) works through an
  `O_NOFOLLOW` directory fd. It checks ownership, unlinks rather than following planted symlinks,
  and creates `settings.json` with `O_EXCL|O_NOFOLLOW`. It runs at launch and again inside the
  session lock before every turn. Delivery revalidates the bundle closure and the full content
  fingerprint (`_check_environment`).
- **Pre-dispatch policy.** The proxy checks the exact query `beta=true`, a header allowlist,
  refuses duplicate headers, requires the exact placeholder bearer and the version, client
  markers, retry count `0`, beta allowlist and the OAuth beta. On the body it checks the field
  allowlist, model pin, `stream: true`, system shape (billing header and SDK identity),
  `context_management` and tool names (`mcp__pi__*` and `DeferredToolPlaceholder`, no typed
  tools, inline tool additions included). Every claude-sdk refusal calls `halt()` before it
  replies (`pi_container.py:1456`).
- **Upstream headers are rebuilt.** Only the host bearer, version, validated betas, identity
  encoding, the length and a fixed client-metadata list are sent. The upstream path is fixed
  (`pi_container.py:1713`).
- **Output hold.** Nothing is released until the billing verdict is `allowed` or
  `allowed_warning` and the first non-ping `data:` event is a `message_start` with the exact
  model (`pi_container.py:1735-1790`). Error bodies are neither relayed nor logged; only a bounded
  `error.type` is kept. A non-SSE 200 is held until EOF and then refused.
- **Durable intent and admission.** The intent is written with `open(..., "x")`, a file fsync and
  a parent-directory fsync before `Popen` (`role_host.py:1451`, `_write_prompt`). The admission
  is fsynced under the lock before any byte goes upstream. A write failure gives
  `admission_persistence_failed` and no forward (covered by tests).
- **Discovery.** `claude_sdk_intents` and `pi_claude_sdk_intents` find intents from host-only
  files, with or without a ledger row. Delivery is refused for an unresolved, corrupt or
  unrecorded intent (`role_host.py:1334-1341`). The ledger emits one row per admission:
  `dispatch_unknown` with empty usage and an issue, never zero. A refused request produces no
  row. Rows go to pool `claude_all_models` with `usd: None`, so they are unpriced and the objective
  status becomes `partial`, an explicit gap. Nothing goes to `pi_api`. `call_id`
  (`claude-sdk:<log>:<n>`) and `seen` prevent double counting. Container Claude Code transcripts
  stay in the tmpfs home. `pi_native_join(skip=set(pi_sessions))` keeps native Pi files of these
  sessions out.
- **Existing lanes.** `container_env` now appends `key_env` at the end rather than in the middle,
  but the fingerprint uses `sort_keys=True`. Every edit to `pi_container.py` already changes
  `pi_container_sha256`. `refuse()` halts only for claude-sdk. `make_proxy` keeps `usd_limit` for
  the other providers. `run_turn` now records a proxy-construction `ValueError` and returns 64
  instead of crashing. The `_spawn` lock path is unchanged for the other lanes. I found no
  behavioural regression by reading. The parent reports the full suite green apart from the
  pre-existing `UsdPricingTests` failure; I did not rerun it.

### Findings

#### P1-1: The halt has a race. A request can be admitted and forwarded after an incomplete exchange

- **Location:** `skills/manual-orchestration/scripts/pi_container.py:1690-1700` (`finish`), with
  the waiter at `:1674-1681` and `halt` at `:1642`.
- **Evidence (code).** `finish()` writes the disposition, sets `proxy.inflight = None` and calls
  `notify_all()` under `proxy.lock`. It then releases the lock, and only afterwards calls
  `self.halt()`, which takes the lock again to set `proxy.halted`. A request waiting in
  `proxy.settled.wait_for(lambda: proxy.inflight is None or proxy.halted is not None)` can take
  the lock in that gap. It then sees `inflight is None` and `halted is None`, and is admitted and
  forwarded. The code's own comment (`:1674`) says this waiter is the normal case: "Claude Code
  sends its next request as soon as it reads message_stop, while this proxy may still be writing
  the previous disposition". The race is reachable whenever a disposition is not `completed` after
  `message_stop` was already released. Examples are `usage_not_established`, or `stream_error`
  from a reset after the final event.
- **Evidence (run).** I used the test harness and the fake upstream. I replaced `proxy.lock` with a
  wrapper that delegates to the real lock and only sleeps 0.5 s before `halt()` re-acquires it, on
  the thread that wrote the failed disposition. Request A used mode `{"hang":1,"terminal":false}`.
  Request B was sent 0.2 s after A was dispatched. Result: the upstream received **2** requests.
  Proxy rows: `admitted 1`, `request 1 terminal_missing`, `admitted 2`, `request 2 completed`.
  B got a 200 and `halts == ['claude_sdk_turn_halted: terminal_missing']`. The wrapper changes only
  timing, not lock semantics, so this interleaving is legal for the shipped code.
- **Contract contradicted.** S0 README:76 says "No later request in that turn is forwarded".
  `pi-host.md:460` says "every later request in it is refused". The plan requires no replay or
  continuation through an uncertain exchange. Accounting stays truthful because B has its own
  durable admission.
- **Smallest fix.** In `finish()`, set `proxy.halted` inside the same locked block, before
  `notify_all()`, whenever `completed` is false (including a failed disposition write). Call
  `on_halt` after releasing the lock. Add a test with a waiting second request.

#### P1-2: Duplicate JSON keys let a request bypass the model pin, tool and field checks. The proxy validates one parse and forwards the raw bytes

- **Location:** `pi_container.py:1879` (`body = json.loads(raw)`), `:1887` and `:1713` (the raw
  bytes are forwarded). Compare the `anthropic` lane at `:1512`, which re-serializes
  `json.dumps(body)`.
- **Evidence (run).** I built `raw = '{"model": "claude-sonnet-9-9", "tools": [{"type":
  "web_search_20250305", "name": "web_search"}], ' + <valid S0 body without its leading '{'>`.
  `check_claude_sdk_request(json.loads(raw))` returned `None`, because Python keeps the last
  duplicate. The proxy answered 200, and the fake upstream received one request whose
  `body_sha256` matched these exact bytes, including the unvalidated model and hosted tool. Python
  also accepts `NaN` and `Infinity` (`json.loads('{"x": NaN}')` gives `{'x': nan}`), which are not
  JSON.
- **Why it matters.** Any process in the container can reach the relay with the public
  placeholder; the plan calls these checks a misconfiguration check, not proof of caller
  identity. Whether the upstream parser keeps the first or the last key, or rejects duplicates,
  is unknown. If it is not last-wins, the model pin, hosted-tool refusal and field allowlist do
  not bind what is dispatched. Hosted `web_fetch` or `web_search` would also give the networkless
  container a provider-side network path. The plan requires "Strictly parse and validate request
  bytes, then forward the same accepted body".
- **Smallest fix.** For claude-sdk, parse with `object_pairs_hook` that rejects duplicate keys at
  any depth, and `parse_constant` that rejects `NaN` and `Infinity`. Refuse with
  `body_not_strict_json` (this halts). Add a test case.

#### P2-3: `deliver` crashes with `KeyError: 'turn'` while an unrecorded turn's container is running

- **Location:** `role_host.py:1672` returns `{"state": "running", "kind":
  "pi_container_turn_unrecorded", ...}` with no `"turn"`. `role_host.py:1739` (`deliver`) and
  `:1327` (`_spawn`) format `st['turn']`.
- **Evidence (run).** In the `TurnCustodyTests` harness, the launcher dies after the intent and
  `_container_state` is patched to `True`. `status` gives `state=running, turn=None`.
  `host.deliver(...)` raised `KeyError 'turn'`, and `_spawn(event="deliver")` raised
  `KeyError 'turn'`. This fails closed (nothing starts), but the caller gets a traceback instead
  of `refused_busy`.
- **Smallest fix.** Include `"turn": orphans[0]["turn"]` in that status dict, or use
  `st.get('turn')` in both messages. Add a test assertion.

#### P2-4: The proxy log's directory entry is not made durable, so after an OS crash a dispatched turn can read as `missing`, which the contract treats as known no-dispatch

- **Location:** `pi_container.py:1437-1442`. `start()` fsyncs the log's directory only for
  `SubscriptionProxyBudget`; claude-sdk has no budget. The `host/` directory chain created in
  `container_environment` (`d.mkdir(parents=True)`) is not fsynced either.
- **Evidence (reading).** The admission row is fsynced through the file descriptor only. POSIX
  does not make a new file's directory entry durable without a directory fsync, which is why the
  Codex lane does one. The contract says `missing` means "the proxy never started, so nothing was
  sent" (S0 README:110, `pi-host.md:470`). The plan says "Make file and directory durability
  explicit on creation". This needs a host crash, not process death, so it is P2.
- **Smallest fix.** In `start()`, fsync the log directory also when `self.provider ==
  "claude-sdk"`. At launch, fsync `host/` and its parent once after creation.

#### P2-5: Workspace project settings can override the pinned Pi retry setting

- **Location:** `role_host.py:888` resets only `models.json`, `claude-sdk.json` and
  `settings.json`. `Pi.BOOL_FLAGS` accepts `--approve`/`-a` for this lane, and
  `claude_sdk_options` does not refuse it.
- **Evidence (reading Pi 1.5.2).** Pi keeps project-trust decisions in
  `<agentDir>/trust.json` (`core/trust-manager.js:180`), and the container can write the agent
  dir. `.pi/settings.json` is a trust-requiring resource (`trust-manager.js:8-11`). With no CLI
  override, `resolveProjectTrusted` returns `trustStore.get(cwd)` (`core/project-trust.js:~38`).
  Pi then deep-merges the workspace `.pi/settings.json` over the host-written global settings
  (`core/settings-manager.js:220`). So one turn can write `trust.json` plus
  `<cwd>/.pi/settings.json` with `{"retry":{"enabled":true}}`, and the next turn runs with Pi
  retry re-enabled. `--approve` gives the same result directly. The fingerprint still records
  `pi_settings` as pinned. The effect on egress is bounded: the proxy's halt makes any resend after
  an error get `turn_halted`, apart from the P1-1 race. The plan, however, requires that earlier
  workspace writes or turn-state edits cannot shadow the lane's bootstrap.
- **Smallest fix.** Add `trust.json` to the reset list, refuse `--approve`/`-a` for claude-sdk,
  and consider pinning `--no-approve` in `pi_argv` if project-local resources are not needed on
  this lane.

#### P2-6: URL or file-id image and document sources are not refused, so the provider fetches for a networkless container

- **Location:** `check_claude_sdk_request` (`pi_container.py:761-818`) validates message roles
  only, not content block sources. The existing `anthropic_unpriceable` (`:844`) detects
  `image`/`document` sources whose type is not `base64`, `text` or `content`. It is used only when
  a dollar budget exists (`:1484`).
- **Evidence (reading).** A body with
  `{"type":"image","source":{"type":"url","url":"https://…?q=<data>"}}` passes every claude-sdk
  check. The provider would fetch that URL. S0 observed no such sources. The plan allows only the
  observed wire and refuses hosted forms before dispatch.
- **Smallest fix.** In `check_claude_sdk_request`, refuse when
  `anthropic_unpriceable(body.get("system")) or anthropic_unpriceable(body.get("messages"))` is
  truthy, as `source_refused:<type>`.

#### P2-7: Docs and contract text differ from the code on refusals and on what `complete` means

- `records/pi-container-claude-sdk-s0-20261124/README.md:100` says "`refused` rows are refusals
  before any admission. Nothing was dispatched." But `refuse()` also logs after dispatch:
  `upstream unreachable` (`:1719`), `upstream_status_N` (`:1732`), and billing or model refusals
  at the end of the exchange (`:1814`). `status` lists them all under `egress.refused`.
- README:110 says "Only `complete` and `missing` mean the outcome is known". `complete` only means
  that every admission has a disposition. It covers `dispatched_no_response`, where the code
  comment at `:1716` says "The request may have reached the provider: dispatched, outcome unknown",
  and also `stream_error`, `terminal_missing` and `usage_not_established`. Delivery is then allowed.
  The data does keep these apart: per-request `disposition`, `usage_complete: false`, and a ledger
  `usage_status: not_established` with an issue. Only the turn-level label and the text overstate
  what is known.
- `pi-host.md:460` says "Any refusal … halts the turn". The body-read timeout path
  (`pi_container.py:1860`, `return proxy.log({... "body not received in time"})`) does not halt.
  Nothing was dispatched there, so only the text is wrong. P1-1 is the substantive exception to
  this sentence.
- **Smallest fix.** Correct the three sentences. Optionally, name the turn-level state
  `settled`, or add `outcome_known: false` when any disposition is in the dispatched-unknown set.

#### P2-8: `retire` of an unrecorded turn checks the container but not the runner

- **Location:** `role_host.py:1786-1792`. The intent records no runner identity; it cannot,
  because it is written before `Popen`. `retire` and `status` use only `_container_state`.
- **Evidence (reading).** If the launcher dies after `Popen` and after writing the key, the
  detached runner can still be starting its proxy and `docker run` while `retire` reports
  `unrecorded_turns_collected` and appends `retire`. The container then starts afterwards. The
  evidence remains discoverable (`status` shows the orphan as `running`), so this is a lifecycle
  gap, not an evidence gap. The window is short.
- **Smallest fix.** Have the runner note its pid and start time in the host-only `runner_record`
  first. Have `retire` and `status` treat an intent as live unless the runner record shows
  `container_removed`, or the noted runner is gone.

### Commands run (all read-only with respect to the repository, except writing this file)

- `git status --short`, `git log --oneline -3`, `git diff HEAD --stat|-- <files>`: these scoped
  the change set.
- `cd skills/manual-orchestration/tests && timeout 300 python3 -B -m unittest
  test_pi_claude_sdk.RequestCheckTests test_pi_claude_sdk.ProxyTests
  test_pi_claude_sdk.LaneOptionTests test_pi_claude_sdk.TurnCustodyTests`: **29 tests OK**
  (18.6 s).
- `cd metrics/scripts && python3 -B -m unittest tests.test_usage_ledger_claude_sdk`: **8 tests OK**.
- Three inline `python3 -B -` probes in `skills/manual-orchestration/tests`. They import the
  test harness, use the fake upstream and a temporary directory, and were cleaned up. They
  reproduce P1-1 (forced interleaving, 2 upstream requests), P1-2 (duplicate keys forwarded) and
  P2-3 (`KeyError 'turn'`). The outputs are quoted in the findings.
- I read the installed `pi-claude-subscription` 0.2.4 sources and the Pi 1.5.2 `dist` modules
  (resource loader, settings and trust) for P2-5 and the extension-isolation checks. No
  credential file was read.
- Not run: Docker tests (`RealClaudeSdkContainerTests`), the full regression suite, and any live
  or network call.

### Merge verdict

**OK with notes. No P0.** The two P1s, the halt race and the strict-JSON differential, sit directly
on the lane's fail-closed egress claim. Each is a few lines to fix. Fix them, with tests, before
S3 or any live credential use. Until then, do not accept the S0 README and `pi-host.md` statements
that "no later request is forwarded". The P2s can follow, or be fixed in the same pass, since each
is small.

---

## Recheck (remediation, read-only)

Same rules as the original review: no repository edits, Docker, network or credentials. I read
the current working tree with `git diff HEAD` and the new tests. The original review above is
kept as written; this section records the recheck.

### Review

#### Correct (fix verified)

- **P1-1 halt race: closed.** `finish()` (`pi_container.py:1713-1727`) now sets
  `proxy.halted = reason` inside the locked block, in the `finally`, before
  `proxy.inflight = None` and `notify_all()`. This also covers a failed disposition write
  (`disposition_persistence_failed`). `on_halt` runs after the lock is released and only for the
  first halt. Any waiter woken by `notify_all()`, and any later admission check under the lock,
  now sees `halted`.
  - **Verified by running:** I reran my original forced-interleaving probe (a lock wrapper that
    delays the finishing thread's next acquire by 0.5 s; A in mode
    `{"hang":1,"terminal":false}`, then B). The upstream now gets **1** request. B gets **429**.
    The rows are `admitted 1`, `request 1 terminal_missing`,
    `refused turn_halted: terminal_missing`.
  - The new test `test_queued_request_after_incomplete_exchange_is_never_forwarded` records
    `p.halted` at `notify_all` time and asserts `["terminal_missing"]`. With the old ordering it
    would record `[None]`, so it detects the defect deterministically.
- **P1-2 strict JSON: closed.** `strict_json()` (`pi_container.py:822-834`) rejects duplicate keys
  at every depth (`object_pairs_hook`, which runs after escape decoding, so `"mo\u0064el"` also
  counts as a duplicate). It also rejects `NaN`, `Infinity` and `-Infinity` (`parse_constant`).
  It is used only for claude-sdk (`:1910`). A failure goes to `refuse(400, "body is not JSON")`,
  which logs and halts.
  - **Verified by running:** the original duplicate-model plus hosted-tool body now gets **400**,
    the upstream gets **0** requests, and
    `halts == ['claude_sdk_turn_halted: body is not JSON']`.
  - Out-of-range numbers such as `1e400` parse as `inf`, which is a float, so
    `max_tokens_refused` still applies.
- **P2-3 missing `turn`: closed.** The unrecorded-turn status includes `"turn"`
  (`role_host.py:1687`).
  - **Verified by running (original probe):** `status` gives `state=running, turn=1`, and both
    `deliver` and `_spawn(event="deliver")` raise `HostError refused_busy`. Before the fix both
    raised `KeyError`.
- **P2-4 durability: closed.** `start()` fsyncs the log directory for claude-sdk too
  (`pi_container.py`, `isinstance(..., SubscriptionProxyBudget) or self.provider ==
  "claude-sdk"`). `container_environment` fsyncs `host/`, `pi/<sid>`, `pi/` and the state
  directory for this lane (`role_host.py:1073-1079`). The intent keeps its file and parent fsync.
- **P2-5 workspace settings override: closed.**
  - `trust.json` is added to the per-turn reset (`role_host.py:891`).
  - `--approve`/`-a` is refused in `claude_sdk_options` (`:924`).
  - `pi_argv` pins `--no-approve` (`:1441`). In Pi 1.5.2 that sets `projectTrustOverride=false`
    (`cli/args.js:232`), so `resolveProjectTrusted` returns false before reading the trust store.
    The `trust.json` reset is defence in depth.
  - Delivery reuses the launch's validated `host_argv` from the host-only ledger.
  - The test asserts the pinned order `--no-extensions --no-approve -e <extension>`.
- **P2-6 provider-fetched sources: closed.** `check_claude_sdk_request` now refuses
  `anthropic_unpriceable(system) or anthropic_unpriceable(messages)` as
  `source_refused:<image|document>` (`pi_container.py:~802-805`). The check recurses into
  nested content such as `tool_result`. Inline `base64` still passes (new test
  `test_provider_fetched_sources_refused`).
- **P2-7 contract and docs: closed, apart from one wording nit below.**
  - `claude_sdk_turn_summary` returns `outcome_known`: true only for `missing`, or for `complete`
    with every disposition `completed` (test
    `test_outcome_known_only_when_every_exchange_completed`).
  - The body-read timeout now calls `halt()` (`pi_container.py:1886-1889`).
  - `pi-host.md` now says what `complete` means, adds `outcome_known`, says that
    `egress.refused` mixes refusals before and after dispatch, and says the halt is recorded
    before waiters wake. With the timeout fixed, "Any refusal … halts the turn" is now true for
    every `refuse()` call site and for the timeout path.
- **P2-8 runner liveness for unrecorded turns: closed, with a narrow residual window (below).**
  - The runner writes `runner_started {pid, pid_start}` (`pi_container.py:2149`). Its
    `process_start_time` has the same `/proc/<pid>/stat` field-19 format as
    `Pi._start_time`, so pid reuse is detected.
  - `_orphan_runner` and `_orphan_live` (`role_host.py:1781-1795`) treat an intent as live while
    its container runs, or while the recorded runner is alive and no `container_removed` has
    been written.
  - `status` reports `running`. Delivery and plain `retire` give `refused_busy`.
  - `retire --force` sends SIGTERM to the runner's group: the runner leads its own session, and
    its SIGTERM handler stops the container and removes it in its `finally`. It then waits on
    `_group_alive` and collects any leftovers.
  - The new test covers status, refused delivery, refused plain retire, and the return to
    `unrecorded_turn` after `container_removed`.
  - The `--force` kill path has no unit test. A unit test cannot signal its own process group,
    so the path is verified by reading only.
- **No regressions from the fixes in the other lanes:**
  - `strict_json` and the source refusal apply only to claude-sdk.
  - The halt changes are inside `forward_claude_sdk`.
  - `runner_started` is now the first runner record for every lane. Nothing consumes the first
    record by position (searched `usage_ledger.py` and the Pi container, Anthropic and Codex
    tests).
  - The directory fsyncs and the trust reset are gated on the lane.

#### Findings

- **P2-9 (new, docs only): the README tells readers to match `refused` rows by `n`, but they
  carry none.** `records/pi-container-claude-sdk-s0-20261124/README.md:101-104` says "A `refused`
  row without a matching admission is a refusal before dispatch … the disposition row with the
  same `n` is the authority". `refuse()` (`pi_container.py:1477`) logs `method`, `path`,
  `http_status` and `reason`, but no `n`, so a refused row can never be matched to an admission.
  The authority statement is right; the matching rule cannot be applied. `pi-host.md` already
  has it right ("the per-request dispositions say which were dispatched"). Nothing in the code
  depends on this: the summary and the ledger count only admissions and dispositions.
  **Smallest fix:** reword to "Dispatch is read only from admissions and their dispositions.
  `refused` rows are not keyed to requests: they occur before dispatch and also after some
  dispatched exchanges." Alternatively, add `n` to refused rows emitted inside
  `forward_claude_sdk`.
- **Residual note on P2-8 (report-only, not a new defect).** `runner_started` is written after
  the runner reads the spec and `sys.stdin.readline()` (`pi_container.py:2138-2149`). So between
  `Popen` and that first record (interpreter start-up, imports, the key handoff; typically well
  under a second) an unrecorded intent still reads as not live. A `retire` in that window can
  collect nothing and append `retire` before the runner starts its container. The evidence stays
  discoverable: the orphan then reads as `running`, and delivery stays refused because the intent
  is unrecorded. The window is now far narrower than at the original review. Closing it fully
  would need, for example, the runner checking for a host-only retire marker before `docker run`.
  I don't consider that necessary before S3.

### Commands run (recheck)

- `git status --short`; `git diff HEAD --stat`; scoped reads of `git diff HEAD` for
  `pi_container.py`, `role_host.py`, the S0 README and `pi-host.md`; reads of the new tests.
- `cd skills/manual-orchestration/tests && timeout 400 python3 -B -m unittest
  test_pi_claude_sdk.RequestCheckTests test_pi_claude_sdk.ProxyTests
  test_pi_claude_sdk.LaneOptionTests test_pi_claude_sdk.TurnCustodyTests`: **34 tests OK**
  (25.1 s).
- `cd metrics/scripts && timeout 300 python3 -B -m unittest tests.test_usage_ledger_claude_sdk`:
  **8 tests OK**.
- One inline `python3 -B -` script that reruns my three original probes against the fixed code,
  with temporary directories removed afterwards:
  - P1-1: 1 upstream request, B gets 429.
  - P1-2: 400, no upstream request, halted.
  - P2-3: `refused_busy` from both deliver paths.
- Not run: Docker tests, the full regression suite, and anything live, networked or involving
  credentials.

### Updated merge verdict

**OK with notes.** Both P1s and all six original P2s are closed: P1-1, P1-2 and P2-3 were
verified by running, and the others by reading plus new tests that pass. The only open item is
P2-9, a one-sentence README wording fix, plus the report-only residual window on P2-8. Neither
blocks merge or the offline S1/S2 acceptance. S3 still needs its own approval, as the plan says.
