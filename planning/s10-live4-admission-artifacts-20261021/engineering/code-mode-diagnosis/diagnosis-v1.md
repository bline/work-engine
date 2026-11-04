# Codex code-mode diagnosis — bounded read-only result

The zero top-level Responses `tools` result does **not** establish an external runtime blocker. A working local synthetic bridge has now been reported by the parent for installed Codex 0.163.0 + gpt-6-sol. However, the tested standalone `codex exec` profile exposes `apply_patch`, so it does **not** establish a credential-isolated, MCP-exclusive reader when credential-bearing files are visible to its local environment. The existing native app-server route already has the relevant stronger control: `thread/start` with `environments: []`.

## Evidence and compatibility

- Installed native CLI SHA-256 `399ff69c431c066e40de318fba45c2b07f45a76b19939d247ee4828fc70c4215`; companion host SHA-256 `8724d73ec190536233ab4628d24a071bfbcf27e5124f08fe35129f26fe48b916`. The host matches Work Engine's pinned constant at `rust/crates/lifecycle-service/src/operations/mod.rs:664`. Installed version/help and companion `--help` succeeded; these are availability observations, not execution qualification.
- Both cached gpt-6-sol and gpt-6-astra model entries contain `tool_mode: code_mode_only` and `use_responses_lite: true`; see sanitized `model-mode-projection-v1.json`. The original model catalog is hashed but not copied.
- External local Codex source checkout is clean at `039eb58a0ba6647fb8f29fdd35341f3f1b153728` (2026-08-26). Its equality to the installed October binary is **not established**. Source explanations are corroborative; retained installed-binary probes own runtime claims.
- Work Engine Tier 2 graph reads: initial generation 2026-10-21T13:57:03Z, store u72e77f184a9e544fg5; four exact source/doc paths have content_verified/metadata_match/no recorded issue. Parent project later refreshed to generation 2026-10-21T14:08:57Z, store u327c45af83be244cg2. External installed/source files are outside that project; direct targeted source reads and literal searches were used. No exhaustive repository completeness claim.
- Exact source hashes and paths are in `source-manifest-v1.json`. This agent performed no real provider/model call, product build/test/rehearsal, key read, or source edit. The parent owns synthetic forced-call execution and its raw evidence.

## Cause and host bootstrap

`core/src/client.rs:881-917` in the external source selects Responses Lite, prepends `ResponseItem::AdditionalTools { role: developer, tools }` to input, and sets top-level tools to None. The enum serializes as type `additional_tools` (`protocol/src/models.rs:962-972`). Installed runtime capture may render tool declarations in developer content instead; inspect both surfaces and the `functions.exec` description. Counting only top-level tools is invalid.

`core/src/tools/mod.rs:67-88` gives the model's tool_mode precedence over fallback feature flags. A code_mode_only model does not become a direct-tool model merely by disabling code_mode/code_mode_only flags. `code_mode_host=true` is the required bridge control; disabling it fails closed for this route.

`code-mode/src/remote_session.rs:56-110` resolves the companion and checks its existence, then lazily creates a host session. `core/src/tools/code_mode/mod.rs:204-230` initializes the session when execution first needs it. `remote_session/connection.rs:155-250` spawns the host, sends a framed ClientHello and waits for HostHello. `install-context/src/lib.rs:172-203` prefers the packaged resource and then a sibling binary. The installed sibling exists and has the required bytes. No separate Node installation or shell tool is established as necessary for the native V8 bridge.

## Synthetic supported call shape and observations

Use `features.code_mode_host=true`, the pinned companion, the admitted MCP server, and `mcp_servers.corpus.default_tools_approval_mode="approve"` for the bounded reader's calls under global approval never. Keep shell/unified_exec/view_image/image_generation/computer/browser/apps/plugins/agents/goals disabled and web_search disabled. Do not switch models to gpt-5.4 to compensate for an empty top-level tools field.

A synthetic SSE `response.output_item.done` item must have type custom_tool_call, namespace functions, name exec, a call_id, and raw JavaScript input; it is not a JSON function argument. The complete three-frame fixture is `forced-sse-fixture-v1.json`. The script lists `ALL_TOOLS` and invokes `tools.mcp__corpus__list_artifacts({})` and `tools.mcp__corpus__read_artifact(...)`. The next request should contain a matching custom_tool_call_output; MCP audit and exact returned bytes independently bind the nested call.

Parent-reported installed-binary outcome on 2026-10-21: gpt-6-sol returned two local requests, successful list/read MCP calls, and enabled nested names:

`apply_patch`, `clock__curr_time`, `list_mcp_resource_templates`, `list_mcp_resources`, `mcp__corpus__list_artifacts`, `mcp__corpus__read_artifact`, `read_mcp_resource`.

This parent report refutes the missing-code-mode-host hypothesis for that exact successful profile. This agent has not independently replayed the result, tested Astra's bridge, observed actual inference, or established the final assessment route's full effective capability grant.

## apply_patch boundary and architecture decision

`core/src/tools/spec_plan.rs:1112-1117` registers apply_patch when a local environment exists and model metadata supplies an apply_patch tool type. The cached requested models supply freeform. Shell and unified_exec feature flags do not control that branch. Installed `codex features list` labels apply_patch_freeform removed. Targeted config-schema/config source searches found no supported apply_patch off switch; do not treat a removed feature or an invented config key as enforcement.

Read-only sandbox plus approval never should reject writes (`core/src/safety.rs:26-118`), and outer read-only mounts add filesystem enforcement. That is not MCP-exclusive reading: `tools/handlers/apply_patch.rs:410-435` verifies patch arguments before `execute_verified_patch` reaches `prepare_apply_patch` at line 566. `apply-patch/src/invocation.rs:214-270` reads Delete/Update targets during verification. This is an evidenced pre-authorization file-read path, **not** proof that this installed binary exfiltrated any credential. A denied Add File probe establishes only that tested write denial. It cannot prove absence of reads against visible auth files or proc entries.

The existing Work Engine native route sends `thread/start` with `environments: []` and bounded dynamicTools (`operations/mod.rs:91-133`). Its task profile enables code_mode_host (`:754-760`, `:830-858`), validates effective config before turn entry (`:310-326`), and checks companion hash before spawning (`:535-547`). With no environment the cited registration branch omits apply_patch. Its actual tools are namespace workspace.snapshot_read and task.question_open/result_publish, delivered via app-server item/tool/call to the actor; this is distinct from an MCP corpus server. The selected native product route is fixed to gpt-6-sol and should not be silently changed to Astra.

For a separate assessment, the concrete existing-interface candidate is a standalone **app-server** process using the bounded corpus MCP configuration plus code_mode_host=true; initialize experimental API, config/read, then thread/start with requested model, approvalPolicy never, sandbox read-only, `environments: []`, and no dynamicTools (or an empty list), then turn/start. It can retain MCP while withholding local filesystem tools. Qualify that exact candidate with a local forced SSE probe: capture effective config, complete direct/nested registry, successful exact corpus reads, absence of apply_patch/exec_command, and reject a forced unregistered patch call. Do not reuse a product session or consumed live run.

For standalone **codex exec**, installed help has no no-environment option; inspected `exec/src/lib.rs:1175-1201` constructs ThreadStartParams without setting environments, and the inspected exec CLI/config fields provide no established way to set an empty environment list. That is a bounded unsupported-route conclusion, not a claim about every possible future version or hidden API. If the assessment must remain `codex exec`, credential isolation requires a separately qualified transport/process boundary that never exposes credentials through its readable environment; denying writes alone is insufficient.

## Result

The tool-mode defect was a probe interpretation error plus a disabled-host variant, not established provider failure. The substantive remaining architecture issue is the standalone CLI local-environment capability surface. Native app-server `environments: []` is an existing concrete control worth qualifying for the separate assessment. No paid/live action is authorized or performed by this diagnosis.
