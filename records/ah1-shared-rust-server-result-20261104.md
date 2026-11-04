# AH1 local Rust application-server result — 2026-11-04

Status: accepted bounded private source. AH1 delivers a thin Rust process with
shared session list/read handlers, actual MCP and protected local HTTP adapters,
a Rust client, trusted startup/readiness, and orderly drain/restart.

Accepted source is `53165f4601e61d8baf57b2dcea85d383de362bd5` on
`implement/ah1-server-20261104`, tree
`f617db251f8a945c95b8cf63b5afe574120b7af7`. Its nine-path server-only delta is
based on private `c279b3e3`, which binds the accepted standalone query client
and owner pin `5095b4b7...`. No shared merge, push, installation or deployment
was performed. The shared merged `d5d51ab3...` owner is a different composition
and is not qualified by this result.

The [component contract](/home/bline/cbm-work/wt-ah1-server-20261104/rust/crates/work-engine-server/README.md)
and [exact integration handoff](/home/bline/.local/state/work-engine/ah1-server-20261104T2117/INTEGRATION-HANDOFF.md)
describe public client/configuration/wire behavior and source integration limits.
The [root acceptance](/home/bline/.local/state/work-engine/ah1-server-20261104T2117/ROOT-ACCEPTANCE.json)
binds source, binary, configuration, fixture, custody and review identities.

Actual process fixtures establish direct-owner/API/MCP/HTTP/client parity,
successful discovery, MCP-disabled HTTP/client operation, owner outage and pin
mismatch, protected local-boundary refusals, deadlines, SIGTERM/MCP-EOF helper
drain, listener closure and restart. Build, two Rust tests, formatting and
server-scoped strict Clippy passed. Five repaired-subject custody steps bind
105 input entries and 40 retained fixture files, with no recorded leftovers.
Dependency-wide Clippy's inherited query-owner warnings remain a limitation.

A separate Astra/high/default reviewer with an independently observed read-only
and network-disabled grant found two P2s: double consumption of the MCP task
handle on EOF, and incomplete discovery/owner-error evidence. Both were fixed.
The same reviewer SID/history accepted the exact repaired delta with no open
findings; this was a retained recheck, not a second fresh audit.

The original executable checkpoint was accepted at 21:57 UTC before both
45 minutes and $8, with $5.64 observed. The original $25/two-hour grant was
preserved; [the closing result](/home/bline/.local/state/work-engine/ah1-server-20261104T2117/ROOT-RESULT.json)
owns final observed cost and process/notification disposition. Cost is an
API-equivalent proxy, not measured subscription quota; reporting tail remains
distinguished from settled totals.

AH1 does not enable granted mutations, native execution, remote/multi-client
authorization, task/approval projections, signing or A4.2 readiness. It does not
qualify abrupt crash recovery, hermetic builds or production operation.
AH2/AH3 and integration against the shared merged owner remain separately
admitted work. Existing backend, CLI/UI and S10 ownership was preserved.
