# Review: H2 Pi-route slices 1–3 (`1051e660` vs `4d5b0b34`)

Reviewer: Fable (claude-fable-5-1), 2026-11-17. Read-only complex review per
[PACKET.md](PACKET.md). Method: `subject.diff` for exact subject bytes;
working-tree source for surrounding seams (no shell in this session, so no
`git show`; every hunk compared between diff and tree was identical and the
tree has no uncommitted `rust/` changes). Line numbers are working-tree lines.
No tests were run here; the commands a supervisor should run are at the end.

## Verdict

**OK with notes.** No P0 or P1. One P2 (a Host-side defense-in-depth gap for
the "no shell" property), the rest P3. The record's claims for slices 1–2 are
supportable from the code; slice 3's claims are supportable from the code
subject to the pending offline/live runs the packet already names.

## Answers to the packet's questions

### 1. Can the model affect anything beyond reading pinned snapshot bytes?

Checked paths, all closed except the P2 below:

- **Tool surface reaching the provider.** `hosted_fetch` collects tools from
  the top-level array and every `additional_tools` input item
  (`pi_coding_hosted.rs:31–45`), requires `type: function` for all, and for
  analysis runs `check_request_tools` (`pi_coding_analysis.rs:51–69`): exact
  two names and canonical digest `TOOLS_SHA256`. This happens inside
  `provider_fetch` handling, before the stream enters `Fetched`
  (`pi_coding_hosted.rs:519–539`), i.e. before any attempt is reserved or any
  upstream byte written (`hosted_step` → `hosted_attempt`, 715–725, 763+).
  `model`, `stream`, `store`, `background`, `service_tier` are also pinned
  (603–633). `tools: null` with no items yields `[]` → name mismatch → refuse.
- **Receiver tool registry.** Analysis registers only `snapshot_list` /
  `snapshot_read` (`pi-coding.mjs:198–204`); `workspace_exec` is not in
  `customTools`. The Host's `ready` check requires the advertised set to
  equal the profile's (`pi_coding.rs:67–81`, `1113–1121`).
- **Host read outside the manifest.** `serve` resolves `path` only through
  `manifest.get(path)` and re-hashes the file against the manifest before
  returning bytes (`pi_coding_analysis.rs:73–144`); `snapshot_list` iterates
  manifest keys only. Admission bounds the tree to regular files, ≤256
  members, ≤1 MiB, path ≤256 (`validate_snapshot`, 33–47). `check_job`
  requires a canonical, disjoint workspace root (`pi_docker.rs:276–290`).
- **Mount.** The inference container's expected mounts contain no
  `/workspace` for any coding profile (`expected_mounts(j,false,false)`,
  `pi_coding.rs:599–625`) and `verify` requires the mount list to match
  exactly (707–720). `coding_prepare_tool` returns before the workspace copy
  and tool container (932–936); `coding_validate_task` is skipped
  (`pi_docker.rs:1070`).
- **Shell.** No tool container; `CodingEffectGate::execute/observed` refuse
  (`analysis.rs:306–323`); `with_pi_coding_effect` refuses the Analysis kind
  (`lifecycle-store/pi_coding.rs:113–118`); the grammar refuses any
  `pi.coding_*` ingress (`pi_analysis_proof.rs:69–75`); `analysis_complete`
  requires `tool_requests` empty (`pi_coding_analysis.rs:273`). **Gap:** the
  Host's `"tool_request"` arm has no profile guard — see P2-1.

### 2. Is every claim of the record supportable?

- **Body retention and timing.** Exact body written `create_new`, fsynced
  with its directory, registered as an artifact and journaled before the
  stream becomes `Fetched` (`pi_coding_hosted.rs:519–539`); every attempt
  record gets `request_body_sha256` immediately after `reserve_attempt` and
  before the durable save that precedes gate admission, connect and dispatch
  (797–809). Failover re-dispatches the same `Fetch` carried in
  `StreamPhase::Fetched(fetch)` (`hosted_progress`, Refused arm) — same body,
  same digest. Supportable.
- **Delivery/native matching.** Host: `analysis_native_end`
  (`pi_coding_analysis.rs:227–266`) requires one content block whose parsed
  text equals the retained delivery (`result` or `{"error":…}`),
  `hostReportedSuccess` equal to `ok`, tool name equal, `isError:false`, each
  id matched once; unknown ids are admitted only as `isError:true` and
  recorded as SDK-refused so a later `snapshot_request` with that id is
  refused (177–183). `analysis_complete` requires every delivery resolved and
  matched (268–288). Supportable.
- **Grammar binding of tool results to Host deliveries.**
  `pi_analysis_proof.rs`: each executed call needs exactly one
  `pi.analysis_read` with its id (27–50), bound by invocation, tool name,
  arguments, artifact digest, delivered==result, `snapshot_result` type;
  native content must equal `expected_content` and details must equal
  `{"hostReportedSuccess": ok}` (160–176); the read must sit strictly between
  that call's `tool_execution_start` and `_end` in the ingress order (172);
  refusals must have no read (177–180); reads and executed calls are in
  bijection (191); final message must be a clean `stop` with non-empty text
  ≤64 KiB; `agent_end`/`agent_settled`/`pi.session_history` checked as in v4.
  The negative cases in `tests/pi_analysis.rs` cover forged result, missing,
  duplicate, late and mislabelled deliveries, a refusal with a delivery,
  tampered bytes, a coding intent, a non-stop final and an altered request
  input. Supportable.
- **Settlement.** `settle_pi_coding` Analysis branch
  (`lifecycle-store/pi_coding.rs:368–394`): domain turn, `first_sequence==1`,
  `validate` → `commit_native_settlement` with the final text; no anchor, no
  checkpoint, no effect entry. `analysis-qualification` requires
  `component_passed` **and** `CompletedSettled` before acknowledging
  (`analysis-qualification.rs:178–186`). Supportable.
- **Slices 1–2 claims.** `message_update` suppression is live-bridge only
  (`pi-coding.mjs:212`); the v4 grammar's `assistant()` tolerates absent
  updates (`pi_coding_proof.rs:56–70`); the live protocol digest string
  changed (`lifecycle-store/pi_coding.rs:26–30`) and no other literal pin of
  the old string exists outside stale worktrees. Incremental read:
  `coding_advance_from`/`coding_unprocessed` share `coding_out_offset`
  (`pi_coding.rs:1004–1011`, `1648–1653`); the offset is advanced only after
  all lines in the tail applied, and an error aborts the loop
  (`pi_docker.rs:742–745`), so no frame is applied twice. Unit test
  `incremental_output_applies_each_complete_frame_once` covers partial line,
  single application, duplicate refusal and agreement with the whole-file
  path.

### 3. Relay coalescing vs. the settlement rule

- **Non-blocking / no loss.** `next_buffered` (`pi_stream.rs:189–218`) only
  calls `next` when the size line, the bytes it will take and (for a full
  chunk) the 2-byte terminator are already in `BufReader`'s buffer; a
  zero-size chunk returns `None` (207), so the chunk terminator and EOF are
  consumed only by the blocking `next` → `Ok(None)` and `upstream_end_ms`
  semantics are unchanged. A result found while coalescing is `held` and
  returned at the next request (473, 485–488); `last` is computed on the
  result actually sent, so neither an error nor the end is lost. Coalesced
  size ≤ `CHUNK` (480), so a relayed frame stays ≤ ~22 KiB base64 < the
  receiver's 128 KiB input bound. Unit tests cover both coalescing and the
  partially buffered chunk.
- **Screening / held-back bytes.** `advance` screens the whole `held`
  buffer and releases all but the suffix the screen holds back; a larger
  chunk only widens the screen window. A secret split across HTTP chunks that
  are now coalesced is detected earlier (before forwarding) rather than in the
  drain; both outcomes are `Interrupted("hosted_access_exclusion")`.
- **Terminal-last / tail rule.** `terminal_is_last(forwarded, unforwarded)`
  judges the concatenation (358), and `usage` takes the first
  `response.completed`; moving post-terminal bytes from `unforwarded` into
  `forwarded` cannot change the verdict. `terminal_last` additionally
  requires `held_dropped == 0`, no `drain_error`, and drained or framed end
  (`pi_coding_provider.rs:159–169`) — unchanged.
- **Drain.** Cancel after a coalesced terminal chunk → `Draining` →
  `reader.data` yields the held `Ok(None)` or reads the terminator →
  `Cancelled{upstream_end:true}` → Completed under K-a, as before.
- **Truncation / interruption.** One behavioural difference: for a chunk
  whose terminator is malformed, `next` previously dropped that chunk's data
  and returned the error; now the data is forwarded and the error arrives one
  request later. Disposition is unchanged because `settle_attempt` makes any
  `interrupted` relay Uncertain regardless of `terminal_last`
  (`pi_coding_provider.rs:255–281`).
- **Loop timing.** `busy` is set only when container output was applied or
  stream frames were written (`pi_docker.rs:648, 667, 722, 826`); idle ticks
  still sleep 20 ms. No hot loop.

### 4. Bounds end to end

| Quantity | Receiver | Host | Store/other | Consistent |
| --- | --- | --- | --- | --- |
| Container→Host frame | `config.frameBytes` = 1 MiB (`pi-coding.mjs:25,54`) | `FRAME_BYTES` 1 MiB (`pi_coding.rs:1014–1020`) | ingress ≤ 1 048 576 B (`native.rs:2647`); recorded value is a subset of the frame | yes |
| Host→container frame | 128 KiB (`pi-coding.mjs:105`) | largest: `snapshot_result` ≤ 100 000 B (`pi_coding_analysis.rs:26`), prompt ≤ 32 KiB, relay chunk ≤ ~22 KiB | — | yes |
| Request body | `MAX_BODY` 600 000 (`:26, 253–255`) | 600 000 (`pi_coding_hosted.rs:606–610`) | observer 700 000 | yes |
| Context | `config.maxContextBytes` (`:229`) | plan ≤ 400 000 (`pi_coding_provider.rs:449–455`); frame carries it twice → ≤ ~800 KB < 1 MiB | — | yes |
| Requests | `config.maxRequests` (`:27, 229`) | plan ≤ 32 (`pi_coding_provider.rs:444`); gate `1..=64` (`analysis.rs:76`) | grammar `MAX_ROUNDS` 64; events ≤ 4096 (`lifecycle-store/pi_coding.rs:314`; worst case ≈ 32·6 + 64·5 ≈ 520) | yes (Host governs; see P3-b) |
| Reads | — | `max_tool_calls` ≤ 64 (`pi_coding.rs:402–408`), page 24 576, args ≤ 1024 | packet `max_snapshot_reads` must equal (`analysis-qualification.rs:98`) | yes |
| Job output | — | 64 MiB loop + `read_from` (`pi_docker.rs:649–653, 1465–1475`) | — | yes |
| Prompt | — | gate 32 768 on the composed prompt (`analysis.rs:26, 74–75`) | store 32 768 for the analysis profile (`native.rs:2494–2499`) | yes |
| Answer | — | none before settlement | grammar 64 KiB (`pi_analysis_proof.rs:14`) | fail-closed, late (P3-d) |

Nothing fails open. Two fail-closed-but-late cases are noted (P3-d, P3-e).

### 5. Coding profile regressions from the shared changes

- `Profile::validate` now requires `toolchain` and `acceptance` for every
  non-analysis schema (`pi_coding.rs:423–424`) and keeps the 4-request /
  24 000-byte / `SrcLibRs` pins via `is_continuity` (420–421, 433–434); the
  `complete` arm keeps `len != 4` for continuity (1610); the receipt pass
  condition is unchanged when `analysis` is false (`pi_docker.rs:1310–1317`);
  `verify_live_job` now names the coding profile explicitly (`job.rs:210`).
- `is_hosted` covering two schemas affects `bind_hosted_transport`,
  `hosted_profile`, hosted mounts, `bind_coding_effect_gate`,
  `ProviderPlan::validate`; each still admits the coding profile as before.
- `declared_tools` is stricter for coding than the previous `tools`-only
  check (P3-j: not yet covered by a coding run on `1051e660`).
- Loop timing and incremental read apply to coding; covered by the s12 set on
  `da0a54f8` (the subject only adds the analysis branches to those paths).

## Findings

### P2-1 — Host `tool_request` arm is reachable under the analysis profile

`pi_coding.rs:1278–1301`. The arm checks expiry, id, duplicate/count and the
command bound, journals the intent as `None` (1296–1298) and then calls
`coding_inspect_tool()` (1301). There is no `analysis_profile()` guard, and
the count check uses `max_tool_calls`, which is up to 64 for analysis
(402–408). The refusal therefore comes from `docker inspect` of a never-created
`-tool` container failing ("tool inspect unavailable", 911–920), not from
policy. The pinned receiver cannot emit `tool_request` in analysis mode (no
`workspace_exec` registered), so this is not reachable by the model today, but
the "no shell" guarantee should not depend on a Docker failure mode.
**Smallest fix:** at the top of the arm, mirror the `claims_request` guard:
`if self.analysis_profile() { return refuse("workspace tool outside a coding profile"); }`.

### P3 notes

- **a.** `coding_hosted_grant` labels every stream binding with
  `HOSTED_BRIDGE_PROFILE` (`pi_coding_hosted.rs:496`), including analysis
  streams. Identity-only (compared by equality), but the retained
  `relay.json`/stream evidence will name the wrong profile. Use the job's
  schema.
- **b.** Request-count constants disagree: Host `MAX_REQUESTS` 32
  (`pi_coding_analysis.rs:12`), gate `1..=64` (`analysis.rs:76`), grammar
  `MAX_ROUNDS` 64 (`pi_analysis_proof.rs:15`). The Host bound governs because
  the plan is checked at admission; align the gate/grammar to 32 or document
  that they are looser by design.
- **c.** `AnalysisSession::new` does not validate its `HostedProviderPolicy`
  (the coding gate does, `coding.rs:379`); the Host validates the same table
  via `HostedRoutes::validate`, so there is no admitted gap, only a weaker
  gate-local invariant.
- **d.** The 64 KiB answer bound exists only in the grammar
  (`pi_analysis_proof.rs:14, 194–196`). A much longer final answer passes the
  Host (the `message_end` frame carries it once, ≤1 MiB) and fails either at
  the receiver's history frame (conversation twice) or at settlement, after
  all requests were paid. Fail-closed; consider a Host-side check on the final
  assistant `message_end` text length so the job fails before the history
  round trip, or pin `max_output_tokens` in the body policy.
- **e.** `MAX_RESULT_BYTES` 100 000 (`pi_coding_analysis.rs:26, 194–197`)
  refuses the whole job rather than returning `ok:false` to the model. A
  24 576-byte page of control-character-heavy text (JSON escaping up to 6×)
  or a maximal `snapshot_list` (256 members × 256-byte paths ≈ 92 KB) can
  approach or exceed it. For `snapshot_read`, return an error result instead;
  for `snapshot_list`, the admission bound could account for the encoded size.
- **f.** `snapshot_list` reports `bytes` from live `fs::metadata`
  (`pi_coding_analysis.rs:89–94`) rather than a pinned size; `snapshot_read`
  still verifies the digest, so the inconsistency is not evidence-bearing.
- **g.** `TOOLS_SHA256` (`pi_coding_analysis.rs:30`) is a self-derived pin
  ("taken from the first offline run") with no in-repo derivation; the unit
  test only shows a hand-rolled spec mismatches. It proves stability of the
  pinned receiver+SDK surface, and the name/type checks carry correctness.
  The record should state the derivation inputs (receiver sha256, SDK index
  sha256, the retained `coding-hosted-1.body.json` it was taken from) so the
  pin is reproducible. Canonicalization relies on `serde_json` sorted keys
  (no `preserve_order` found in workspace manifests); deterministic either way
  for a given binary.
- **h.** `read_from` applies the 64 MiB bound to every profile's tail
  (`pi_docker.rs:1465–1475`); harmless because the loop checks the profile's
  `output_limit` first (658–661).
- **i.** `next_buffered`'s "never blocks" holds except when the two buffered
  terminator bytes are not CRLF: `line()`'s `read_until` then reads on into
  the socket. `next` blocked identically in that malformed case and the
  socket read timeout bounds it; no regression.
- **j.** The `declared_tools` tightening for the coding profile (refuse when
  `input` is present but not an array or an `additional_tools` item lacks an
  array `tools`; all declared tools must be `type: function`) is not yet
  covered by a coding run on `1051e660` (s12 ran on `da0a54f8`). The analysis
  offline run's digest match evidences the SDK's `additional_tools` shape, so
  the risk is low; close it with the pending coding regression set.

## Correct (evidence)

- `coding_bridge_dispatch` routes every bridge command, including
  `snapshot_result`, through `gate.dispatch` (`pi_docker.rs:686–699`,
  `pi_coding.rs:840–865`), so the analysis gate's "reply equals the recorded
  delivery, sent once under the current turn grant" branch
  (`analysis.rs:420–431`) is live, not dead code.
- The observer's second response pairs a real `snapshot_read` with a
  `workspace_exec` call (`pi_hosted_analysis_observer.rs:72–80`), so the
  offline `ok` scenario exercises the SDK-refusal path, the two-call ordering
  the grammar assumes, and the primary-429 failover with the same body.
- `settle_pi_coding`'s Analysis branch runs before any anchor/checkpoint
  logic, and `with_pi_coding_reset` has no Analysis arm, so an analysis
  session can never be recovered into a coding successor.
- The receiver's `zstd` decompression bound follows `MAX_BODY`
  (`pi-coding.mjs:253`), so the Host never decompresses.

## Tests the supervisor should run

Use a `CARGO_TARGET_DIR` outside `rust/crates/pi-pressure-context-host/target`.

- `cargo test -p lifecycle-store --test pi_analysis`
- `cargo test -p work-engine-manual-host -- coalescing_tests pi_coding_analysis declared_tools_include_additional_tools_items incremental_output_applies_each_complete_frame_once`
- `cargo test -p pi-pressure-context-host` (build of `analysis-qualification`)
- The pending slice-3 offline analysis set and the coding regression set on
  `1051e660` named in the record; the coding set closes P3-j.
