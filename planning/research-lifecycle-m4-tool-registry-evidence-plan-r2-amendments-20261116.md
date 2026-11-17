# M4 tool registry evidence plan: revision 2 amendments

2026-11-16. Root decisions by research-lifecycle root (Pi session
`01a1e56d-7ceb-71fc-9c46-c9fcd9618234`) on the separate Fable xhigh plan review
(`/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/002-plan-review/REVIEW.md`,
verdict `accept_with_required_changes`, five P2 and six P3 findings). The base plan is
[revision 1](research-lifecycle-m4-tool-registry-evidence-plan-20261116.md), SHA-256
`d75e97da5712e0280accefb582e3725f1b860530d4e0c55c19d16452f2ebdece`. Where this document
and revision 1 differ, this document governs. Together they are the accepted S1 plan.
Review was same-provider (Claude builder and Claude reviewer).

## Root decisions on open questions

- **Refusal of use is the claim.** M4 requires evidence or enforcement before protected use.
  This unit delivers evidence and refusal of use. Native pre-execution prevention stays open
  (revision 1, section 8).
- **Expected tool set** as revision 1 defines it, derived from sent bytes (see A4). If the real
  build registers anything else under the selected configuration, the builder stops and reports.
- **Unknown-name dispatch** is consistent with the claim only with a distinct `registry_miss`
  outcome kind, a matching snapshot citation and no handler run (A9).
- **Builder model:** `claude-sdk/claude-sonnet-5-5` at `high` through Pi (Codex quota is exhausted;
  model-selection Claude row for a fully specified packet). Budget table in revision 1 stands.

## Required changes

**A1. Incremental WebSocket tool sends (review 1).** `turn.rs:1675–1679` sends an empty `tools`
when the window uses incremental tools. Strict mode already makes generation requests
self-contained (008). In strict mode, also disable incremental tools so every strict generation
request carries the full `tools` value, following the same pattern 008 used for
`previous_response_id` and WebSocket reuse. If that cannot be done within the owned paths, a
strict send whose `tools` is empty while the snapshot has specs makes the turn ineligible with
reason `incremental_tools_unsupported`; record that and continue rather than widening scope.
The accepted 008 strict-input positive must still pass.

**A2. Join key between snapshot and request bytes (review 2).** Each snapshot has a
`router_snapshot_id`. The strict transport metadata row for every send built from a
`StepContext` cites that `router_snapshot_id`. Extend the exact field set checked in
`compiled_input.rs` accordingly. Pairing by position or order is refused.

**A3. Names and equality (review 3).** Snapshot rows record, per tool: `(namespace, name)` as the
dispatch rows already do; the rendered nested code-mode name from `code_mode_name_for_tool_name`;
exposure; and a SHA-256 digest over the serde-serialized `ToolSpec`. Each snapshot also carries
`model_visible_specs`, serialized by `create_tools_json_for_responses_api` (the function the
request path uses). Request `tools` equality is equality of values parsed with
`lifecycle_wire::parse_strict_json`, with array order significant. Byte equality is not required.

**A4. Expected-set carrier and owned paths (review 4).** Add `rust/crates/lifecycle-codex/src/transport.rs`
to the owned Work Engine paths. The service builds an `ExpectedToolSet` from the exact
`thread/start` `dynamicTools` bytes it sent plus `launch.code_mode_host`, and hands it to
`NativeHarness::acquire_native_input`, which passes it to the consumer. The native snapshot and
dispatch-row publication is gated by the existing `CODEX_STRICT_TRANSPORT_INPUT=1`; no new
environment switch.

**A5. Post-terminal dispatch (review 5).** Drop the consumer-side claim for dispatch after the
turn's terminal. Instead, the controlled fixture re-digests `strict-tool-registry.jsonl` after the
native process has been waited, and fails on any change since acquisition. The consumer limit
(it reads the file once at `turn/completed`) is stated in the result. Native gating through a live
`StepContext` (`delegate.rs:188–195`) is noted as structural evidence only.

## Clarifications

**A6.** Invariant wording: the native diff changes no rule for which tools are admitted. In strict
mode only, a publication failure may refuse the step or the dispatch.

**A7.** Snapshot records `tool_mode` (from `effective_tool_mode`). The consumer derives the
expected mode from `ExpectedToolSet`, never from the snapshot.

**A8.** Startup prewarm, compaction and review captures also produce snapshot rows. The consumer
scopes by `thread_id` and `codex_turn_id`, as `compiled_input.rs` does for transport rows. A prewarm
snapshot publication failure in strict mode fails thread start; record it if observed.

**A9.** The consumer refuses: two snapshots for one step id; two dispatch rows for one
`tool_call_id`; a dispatch row before its cited snapshot in file order; a rejection row without a
distinct `registry_miss` kind (`registry.rs:666` `record_failed` serves both registry miss and
handler failure, so the producer must distinguish them).

**A10.** Cross-check: every `tool_call_started` in the ordinary trace (as `causal.rs` extracts it)
must have a strict dispatch row. One way only, because ordinary writes are best effort.

**A11.** The consumer chain's store check is `lifecycle-store/src/native_causal.rs`, not
`lifecycle-codex`.
