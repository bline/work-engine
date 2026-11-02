# First implementation bid: present-mode Codex

Status: proposal, not admitted by the phase 0 approval. Scope and acceptance
include [contracts revision 1](contracts-v1.md), including the explicit proposed
owner-answer consumption-rule refinement and the empty-comment qualification.

## Result to establish

Dot proposes one typed task; the owner reviews its complete effects/budget/time
terms and signs the existing answer format. A protected Rust consumer verifies
that exact current decision, reserves canonical objective capacity and admits
one Codex task in a disposable repository. The exact session/turn returns a
structured result. The existing UI shows its task, admission, artifacts, usage
coverage and custody state. A repeated decision cannot launch another task.

## Proposed allowance

**$30 speed-adjusted API-equivalent / 3 hours across all actors.** Productive
boundary is $21 / 2 hours 15 minutes; protected review/closure allowance is
$9 / 45 minutes. Both start at first funded setup/readiness and survive restart.

| Included responsibility | Planning allocation |
|---|---:|
| Backend coordination/integration | $4 |
| One bounded backend builder | $9 |
| UI owner: typed review and result projection | $6 |
| Independent Astra review of execution/authority delta | $4 |
| Joined disposable live pilot, held until exact packet approval | $2 |
| Reconciliation, remediation, rollback and closure | $5 |
| **Total** | **$30** |

Allocations guide scope, not separate additive grants. Review and closure remain
protected if productive work grows. Choose exact model/effort/tier with current
selection guidance and capacity evidence at admission; phase 0 launches nobody.
Use one phase ledger/reservation lineage for all actors, including these retained
roots, and do not spend the independent OAuth/bookkeeping allowances twice.

## Source ownership and placement

Backend root owns Rust task/admission state, bridge adapter, MCP/public result
composition and deployment preparation in work-engine. UI root owns the existing
React approval/result presentation in devwits-auth. Each remains directly
responsible to the user. Backend is the proposed joined integration conductor;
the UI owner accepts the presentation and signed-byte boundary. An independent
reviewer accepts only its stated review subject, not deployment authority.

The protected task/admission owner is the proposed
`rust/crates/session-task-admission/` library, with one durable SQLite
answer/admission/outbox store. Existing session-bridge retains native-target custody. Accounting
composes the canonical budget/ledger mechanisms; manual-host does not also
dispatch the product target. No change to foreign bookkeeping/skill-release
files is owned by this bid without coordination.

UI source scope is principally `src/approvals.jsx` and its existing styling,
with only the transport projection changes needed for task/result history.
`api/answers.js`, `api/_lib.js` and laptop verification keep their transport
roles. Any necessary consumer documentation refinement is explicit and reviewed;
the v1 signing bytes and public key trust policy remain the compatibility base.
Do not add a second task authority to Redis or Vercel.

## Qualification and activation

Required checks include typed display completeness, unknown versions, pinned
proof and current lifecycle checks, stale/revoked/expired/cross-task decisions,
all nonempty comments held, atomic answer uniqueness, conflicting IDs, canonical
budget reservation and stale meter behavior, lost ACK/restart recovery,
first-funded clock preservation, missing/invalid worker results and honest
native/process closure. Test existing read-only behavior as a regression.
Qualify the actual native filesystem grant, including resolved-path escape and
protection of authority, budget, key-policy and configuration state from the
worker. Same-user file permissions alone do not establish that protection.

Implementation and independent review precede a concrete immutable release,
profile, endpoint, trust-policy and pilot packet. The public write gate remains
disabled. The live $2 allocation is held until the owner approves that exact
activation packet and signs the exact task. This boundary comes from the
approved completion plan's separate implementation/activation contract; it is
not an extra permission request for ordinary source work.

Before that pilot, the separately owned actual Dot OAuth read, trusted approval
key, qualified isolated workspace effects and complete spend/custody evidence
must be established. Otherwise deliver tested disabled code with the specific
unestablished condition. Do not call a local adapter test a completed real Dot
vertical slice. Rebid if the required profile cannot fit the allowance.

Excluded: Claude control, Away mandates, general follow-up steering, concurrent
tasks, production repositories, deployment automation, a new public write route
and global skill activation. Those remain later completion-plan slices.
