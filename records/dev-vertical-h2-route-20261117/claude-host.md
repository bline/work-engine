# Claude Host core: qualification record

Standing: qualification record for the [Claude Host core plan](../../planning/dev-vertical-claude-host-20261117.md).
Not owner acceptance. The frame vocabulary towards `lifecycle-service` is not
chosen (research-lifecycle owner, board 5849). Run files:
`~/.local/state/work-engine/dev-vertical-h2-route-20261117/claude-host/`
(request bodies stay there: they contain the account email).

Crate `rust/crates/work-engine-claude-host` (new). Claude Code 2.1.326 at
`~/.local/bin/claude`.

## Unit and integration tests

`cargo test -p work-engine-claude-host`: 9 unit tests (tools check exact by
name and value, digest order-independence, namespace mapping, SSE tool_use
assembly and incompleteness, snapshot read bounds, HTTP head and chunked
framing and bounds) and 2 proxy integration tests against the in-process
fixture: pinned requests forwarded with the exact body retained and digested
before dispatch, response retained, credential absent from every file; extra
tool, changed schema, wrong model, non-API path and oversized body refused with
**zero** upstream hits; request-count bound refuses the second request. Clippy
clean.

## Offline runs (real Claude Code, fixture upstream, no inference)

| Run | Fixture script | Result |
| --- | --- | --- |
| `off1` | tool_use `mcp__we__snapshot_read`, then text | eligible, `checked_completed`; 2 requests forwarded; 1 call joined (`toolu_fixture_1`, `request-1`, `msg_fixture_1`, `workspace`/`snapshot_read`); Claude Code's real request matched the pinned tools array exactly |
| `off2` | tool_use for `mcp__we__shell` (not pinned) | ineligible: "tool_use toolu_fixture_1 in request 1 was not relayed" |

No `sk-ant-` or bearer value in either run directory.

## Live run (subscription)

`live-1`: model `claude-haiku-5-5`, H2 treatment snapshot (12 members,
158 961 bytes), non-H2 qualification task (read the first page of each member,
publish a one-sentence-per-member summary with `result_publish`, answer
"done"). Result: eligible, `checked_completed`. Three requests (7 425,
68 382 and 73 707 bytes), 12 parallel `snapshot_read` calls from request 1 and
one `result_publish` from request 2, all joined to their `tool_use` and
response ids; final text "done". Stream rate-limit event `allowed`,
`isUsingOverage: false`; Claude Code reported $0.0078 API-equivalent. User agent
`claude-cli/2.1.326 (external, sdk-cli)`. Tools digest
`171f72bd…c18d45`. No credential in any run file; the account email appears
only in the retained request bodies (as expected; retention rule open with the
operator and the research-lifecycle owner).

## Not covered yet

The lifecycle-facing frames and tool server (after the owner's ruling); the
per-turn capture is in the Host's own shape, not yet the store's
`NativeInputRecord`; no isolation around Claude Code (R9′ open); Claude Code
version pinning is optional in the job (`claude_sha256`); traffic Claude Code
sends outside the base URL is not observed.
