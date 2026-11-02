# Phase 0 owner and reuse disposition

Status: proposed placement and bounded current-source inventory, 2026-11-02.
Source identities and graph limitations are in [source evidence](source-evidence.json).
Neither a merged candidate nor a phase 0 proposal activates runtime behavior.

| Semantic fact | Current owner / evidence | Disposition for the first slice |
|---|---|---|
| Owner identity | devwits-auth Stytch sign-in; existing mcp-auth verifier | Reuse identity checks; identity cannot approve work |
| Human task decision | Board request/reply and exact owner-answer/v1 proof; devwits-auth docs/owner-answer-v1.md | Preserve authority and bytes; explicitly qualify typed review and post-reply consumption |
| Browser review/signing | devwits-auth `src/approvals.jsx` | UI owner extends the existing React view, shows every executable term, holds unsupported terms |
| Answer transport | devwits-auth `api/answers.js`, `api/_lib.js`, `laptop/approval_sync.py`, `laptop/owner_answer.py`; Redis mailbox | Preserve transport and trusted pinned-key verification; do not add a task ledger here |
| Protected task/revision/admission/outbox | Proposed `rust/crates/session-task-admission/` library, backend-owned | One bridge-facing admission owner; new SQLite answer/admission/outbox state; canonical budget reservation composes by stable identity |
| Native target/session custody | `rust/crates/session-bridge`, particularly `src/wire.rs` and `src/store.rs` | Keep one native custody owner; extend a disabled opt-in disposable profile only after qualification |
| Objective accounting and outer engineering workflow | Existing canonical scope-budget/usage-ledger and `rust/crates/work-engine-manual-host` composition | Reuse supported accounting/custody mechanics; do not make manual-host a second product dispatcher or new decision authority |
| MCP/HTTP identity and action gate | `skills/session-events/scripts/session_events.py`, HTTP-OAUTH.md, mcp-auth, bridge RPC | Thin composition over the Rust owner; preserve current read-only default and old RPC meaning |
| Task/result/budget UI history | Future versioned projection from the task owner | Open-request snapshots are insufficient after answers; project durable state with freshness, provenance and explicit unknowns |
| Present/Away revision and mandates | Future Rust authority boundary, backend-owned | First slice fixes present mode; reserve explicit mode/mandate contracts for later qualified implementation |

The frozen `fleet-v2-prototype` export is a source reuse candidate for immutable
proposal snapshots, scoped idempotency, outbox and stale-revision fixtures. Its
UI, simulated operating engine and disabled production auth are not a current
authority or execution owner. Do not migrate it wholesale or spend phase 1
rebuilding an independently qualified bookkeeping core.

The new library is not a generic task engine or separate service. Its atomic
answer/intent-to-admission/outbox transaction is a property to implement and
qualify, not a claim about today's bridge store. A stable operation key joins
the admission outbox to the bridge; there is no cross-store atomicity or
exactly-once native-execution claim. A missing native acknowledgment fences
replay. The authority store, budget state, key policy and runtime configuration
must be outside the worker's qualified write boundary. File mode 0700 alone
does not isolate another process running under the same user account.

## Qualification matrix

| Path | Established now | Remaining qualification |
|---|---|---|
| Codex, present | Scoped diagnostic read-only bridge profile and deployed approval transport | Typed executable review, durable decision admission, confined write profile, actual Dot task/result/cost/closure flow |
| Codex, away | No executable mandate qualification claimed | Explicit delegated authority, aggregate budget/time/concurrency, revocation and transition/restart behavior |
| Claude, present | Transcript observation | Native launch/direction/interrupt/closure parity and task admission |
| Claude, away | No executable mandate qualification claimed | Claude parity plus the same qualified delegated authority and transition tests |

Thirteen published tools do not establish this matrix. The current staging
read-only gate, catalog listings, OAuth login and a diagnostic native rehearsal
are narrower subjects than a real Dot coding task. The separately owned Dot
plugin installation was observed; authenticated real Dot read remains
unestablished in the peer's phase 0 result. Its browser handoff stays with that
peer and its separate allowance.

## Bookkeeping dependency disposition

Candidate `a0cda34e` is now an ancestor of shared HEAD `63bbd7ea`, superseding
the original plan's uncommitted-only premise. The merged fresh-task host was
live verified through pinned bundle `8b8621bd`; see the
[merge/live result](../../records/deterministic-bookkeeping-host-merge-live-result-20261102.md).
The installed manual-orchestration skill remains `13394695`; source merge and
a pinned live test do not globally activate it. Retained selected-turn settings,
broader native fault recovery, Claude and optional structured grants are not
established by that qualification. Reuse only the supported mechanics and
preserve the existing owners' accounting and release boundaries.

## Outcome ownership through completion

| Completion outcome | Owner and acceptance |
|---|---|
| Phase 0 contracts/reuse | UI root integrates; backend source owner concurs; exact-byte fixture checks and explicit limitations |
| First present Codex slice | Backend integration conductor, UI presentation owner, separate reviewer; exact approved task to real Dot native result and truthful closure |
| Codex direction/recovery | Backend; task-bound follow-up, interrupt, reconnection and no blind retry after lost ACK |
| Claude parity | Backend provider adapter; same correlated launch/control/result/settlement tests |
| Away mandates | Backend authority owner plus UI review; both hosts, revocation, bounds, restart and mode ordering |
| Integrated operator experience | UI consumes Rust projections; reload/history, pending questions and unknown state remain truthful |
| Joined rollout | Named release owner and independent reviewer; four host/mode paths plus transition/outage tests, accounting, rollback and exact activation decision |

UI root is `01a19b6d-db5b-7a50-b4d1-7f69e34bb20e`; backend root is
`01a195d4-b7a7-77f2-ba65-277669c0ec97`. Both remain directly accountable to
the user. Source-owner concurrence is not independent implementation review.
