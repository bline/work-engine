# A4 Web adapter: first-slice budget

Recorded 2026-11-04. Status: **proposal; implementation is not admitted**.
Owner: this session, `01a1a7ed-45f5-7041-9db5-a6c9aa1cd3f3`.

The proposed first slice makes the existing Web presentation replaceable without
moving approval or task authority into a UI or transport. It factors the current
presentation, mailbox access and signing dependencies, then exercises both the
current presentation and a minimal replacement against the same pinned local
fixtures. The bid is **$30 total and three hours**. A later slice connects those
boundaries to compatible AH application interfaces.

## Inputs and ownership

The architectural baseline is [revision 4 of the reviewed connector plan](connector-adapter-architecture-plan-20261104.md)
at commit `d86df598f8d140003c9ae2a803b2b7af79c759f5`, SHA-256
`026fca8e02c317fa60997d20a04c71625f006e8194aad00584a751e634765da9`.
Its A4 package explicitly permits presentation fixtures before AH is ready;
actual AH composition remains required for the overall connector result.

The observed clean devwits-auth input is commit
`119c8669c0d5bccd1acc613eeb14566a057132cc` in
`/home/bline/code/devwits-auth`. Bind its exact affected source and fixture
closure again at admission. Existing material includes:

- `src/approvals.jsx`: Stytch-authenticated HTTP access and the signing action
  are embedded in the presentation. The answer has a fixed JSON field order;
  its exact string is hashed for the WebAuthn challenge and sent unchanged.
- `src/work-dashboard.jsx`, `src/task-contract.js` and `src/task-projection.js`:
  mailbox reads, supported-profile review, exact reference checks and distinct
  outcome/accounting/custody observations. Unknown typed profiles hold approval.
- `tests/backend-interface.test.js` and its frozen Rust-produced projections:
  existing consumer evidence for Held and uncertain observations.
- `laptop/owner_answer.py` and `laptop/approval_sync.py`: independent pinned-key
  verification, exact current request digest, RP/origin checks and replay-aware
  board delivery. Mailbox receipt does not establish admission or execution.

This session owns A4 preparation and, if approved, one isolated devwits-auth
change plus its evidence. AH belongs to session
`01a1a862-034f-7750-b86b-de29e11cf327`; the Rust query/client and CLI belong to
`01a1a84f-3760-7ce0-b46a-ad5b2f520c20`. Backend integration and S10 retain their
domain/runtime work. This proposal spends none of their grants and requests no
new work from them.

The [AH implementation proposal](ah-shared-rust-server-implementation-plan-20261104.md)
currently defines AH1 as session list/read only, with protected local HTTP that
rejects browser Origin. It supplies neither task/approval endpoints nor browser
access. A4.1 can proceed without it. Subsequent Web integration can retain the
public mailbox and make the laptop sync process an AH client; direct browser
access to the laptop is not required.

## A4.1 deliverable and acceptance

Expose a narrow browser-facing boundary for existing request/projection reads,
key reads and signed-answer submission. Keep authenticated HTTP details in the
current mailbox adapter. Separate answer construction and the WebAuthn call
from presentation; neither a transport nor a replacement view may bypass the
supported-term checks or acquire admission authority. Preserve the existing
wire schemas, polling behavior, Stytch binding and response meanings.

This is a refactor of the existing React application, not selection of a new UI
framework. New independently runnable first-party adapters or executables use
Rust under [the existing language direction](../docs/rust-development.md).
Reuse existing browser/Python test machinery and immutable vectors rather than
adding a second service or duplicating a domain verifier.

The accepted result must establish:

1. Both the current view and a small test-only replacement render the same
   required terms and preserve source revisions, unavailable fields, Held and
   unknown observations. They consume the shared boundary without importing
   each other's presentation. Incomplete terms and unsupported profiles cannot
   become a generic approval action.
2. Both can use the HTTP mailbox adapter and an injected local fixture adapter.
   The latter substitutes reads and answer delivery at the application boundary,
   not merely CSS or a URL. Test isolation excludes Dot, MCP, production Redis
   and live provider access. The local adapter is test-only and cannot be
   selected by untrusted production request fields or URL parameters.
3. Under fixed answer ID, time and inputs, both views and both adapters preserve
   the exact answer string, request digest and challenge. The production signer
   retains `auth.devwits.ai`, required user verification and the existing
   assertion envelope. Account changes, expired terms and cancellation during
   an asynchronous signing prompt prevent stale submission.
4. Test-key assertions over those actual answer bytes pass the existing laptop
   verifier; changed bytes/request, closed request, wrong owner/key/origin/RP
   and expired answers fail. A bounded sync replay fixture must show that a
   repeated answer cannot create a second board decision. Fixture acceptance
   does not bypass later domain admission or establish task execution.
5. Focused adapter/signer tests, existing JavaScript suites, production build,
   relevant Python verifier/sync tests and browser approval/dashboard flow
   checks pass. Reuse meaningful existing coverage and extend the boundary
   cases above. Record exact commands, fixture/source identities and limitations.
6. A separate Astra high reviewer examines the frozen affected source and the
   browser/verifier evidence. Blocking findings are repaired and rechecked
   within this bid. Deliver a reviewable source checkpoint and collected handoff;
   any user-visible commit contains only the approved owned change.

Browser tests may inject a test authenticator and generate offline assertions
for the actual verifier. Such evidence establishes the adapter/signing boundary,
not a real production-origin passkey ceremony, deployed Stytch behavior or live
task admission. A4.1 is bounded partial A4 work; it does not complete A4/A5
composed-server conformance.

## Budget and checkpoints

| Allocation | Planned maximum, API-equivalent dollars |
| --- | ---: |
| Implementation and focused validation | 10 |
| Separate Astra high review | 5 |
| Repairs and reviewer recheck | 5 |
| Root preparation, source binding, collection and acceptance | 7 |
| Closure and reporting reserve | 3 |
| **A4.1 total** | **30** |

These are planning allocations inside one ceiling, not five independent grants.
Count this session's attributable A4 preparation from the budget request through
closure, plus every admitted builder/reviewer turn and relevant validation.
Earlier completed architecture reviews and other sessions' work stay separate.
At admission, collect preparation usage already incurred and deduct it from the
same $30; do not reset cost to zero. Use observed service-tier-adjusted
API-equivalent accounting and disclose unsettled or unavailable usage. This is
not a claim about actual subscription billing or quota.

Proposed organization after approval: one bounded builder, normally
`gpt-6.1-terra`/medium/Fast, with this root retaining contract and integration
decisions; one separate top-level `gpt-6-astra`/high/standard read-only reviewer,
retained through a bounded repair. Observe applied settings and the review
grant. No roles have been launched. Use an isolated worktree; the builder is not
alone in the repositories and must preserve other actors' changes.

The first executable checkpoint is the earlier of **45 minutes after admission
or $8 aggregate**, including preparation. It must show both presentations
reading one pinned request through the injected boundary, plus a signed-answer
byte vector accepted by the actual verifier. This tests both extraction and the
security-sensitive seam before spending the remainder. If unestablished, stop
new dispatch, collect work and report the concrete blocker and remaining bid;
do not restart the clock or silently reduce final acceptance.

Target productive completion by two hours or $22 aggregate. Protect the last
hour and $8 for review, repairs, collection and closure; begin review earlier
when an immutable candidate is available. The hard overall limit is **three
hours from approval and $30 including preparation**, whichever arrives first.
An unresolved blocker or exhausted allowance produces a truthful partial
handoff rather than an unsupported completion claim or automatic extension.

## Following slice and integration limit

**A4.2: actual AH integration and replacement qualification** is provisionally
**$20–30 / two hours**, to be re-estimated against accepted immutable AH and
domain contracts. It wires the Web/sync adapter to the actual supported owner
queries and decision path, exercises current and replacement presentations and
an alternative local transport with MCP disabled, and checks owner outage,
restart, stale/denied input and unchanged verifier behavior. AH1 session reads
alone do not satisfy its task/approval prerequisites. If needed interfaces are
missing, report that dependency to their owner rather than inventing another
backend or importing an unfinished private candidate.

The tentative A4 envelope is therefore **$50–60 / five hours**, excluding AH
implementation, A2 multi-client/remote policy, A3 delivery migration, A5 overall
release conformance, Pi-specific review support, live provider runs, deployment,
production Redis changes and credential/pin changes. Only A4.1 is offered for
approval now. No production state is migrated by either estimate.

## Evidence and continuation

Repository evidence is Tier 2, task-directed, with direct-source checks of the
known paths above. The work-engine graph generation observed for this proposal
was `2026-11-04T21:14:16Z`; coverage checked both plans and every relied-on
devwits-auth implementation path. The latter are outside that graph project,
so source inspection supplies their evidence. This is not an exhaustive
cross-repository audit or new runtime qualification.

The same session continued after automatic compaction, retaining the reviewed
architecture, ownership binding and pending budget decision. Durable recovery,
coverage and proposal references are under
`/home/bline/.local/state/work-engine/a4-web-adapter-budget-20261104/`.
Implementation remains pending explicit approval of this bounded bid.
