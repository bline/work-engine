# UI projections and existing transport ownership

Status: phase0 proposal; these are semantic obligations, not new live endpoints.
The integrated [contracts revision 1](contracts-v1.md) names the proposed public
envelopes. The backend contribution supplies their source and authority basis.

| Existing component | Proven role | First writable-slice obligation |
|---|---|---|
| auth.devwits.ai sign-in | Stytch owner session; access to private pages | Identity remains separate from task/mandate approval |
| /approve + owner-answer/v1 | Renders generic request, signs exact bytes | Render every supported typed task term; hold unknown/unrenderable intent |
| POST /api/answers | Early owner/signature validation; stores a pending answer | Keep transport acknowledgment distinct from runtime admission/start |
| approval_sync.py | Verifies against pinned public keys before posting a board reply | Preserve exact proof, row digest, answer identity and decision content |
| Board request/reply | Existing owner decision record | Distinguish this answer's closure from withdrawal/supersession and effect consumption |
| Redis mailbox | Snapshot/answer/challenge transport | Never mint a task permission or become canonical task/budget state |
| Future task/result view | Not implemented in current UI | Versioned projection from the Rust owner, with revision, freshness and provenance |
| Future present/away UI | Not implemented in current UI | Changes become authoritative only after backend acceptance; show the effect on existing grants and runs |

The first vertical slice can start from a Dot-authored, typed task proposal
which becomes the canonical board request. It does not require a new generic
Start-any-task form or a second decision broker in Vercel. The owner reviews
and signs the complete task terms; Dot then requests admission under the
server-held authority. UI-originated task creation and additional controls
should use the same proposal/decision boundary when later qualified.

A result shown in the UI needs an authoritative task/revision and operation
reference, observed lifecycle state and time, structured worker outcome with
source identity, and unresolved cleanup or settlement. The existing open-request
snapshot alone cannot supply a running/completed task history after a reply
closes the request. Whether the adapter adds a projection route or versions
the snapshot is a transport choice; the Rust task owner retains the state.

Budget views distinguish limits, reservations, observed proxy usage, unknown
coverage and first-funded/close-by times. Unknown is not zero. Result or stop
views distinguish requested, acknowledged, observed and uncertain outcomes.
A signed answer stored in the mailbox is not a started task; a completed
assistant turn is not proof that all owned children/processes are closed.

Keep provider tokens, approval private keys and server-held capability secrets
out of browser DTOs and MCP model inputs. Public handles may locate authority
only through a checked trusted consumer context; caller JSON cannot create it.

Frontend owner: UI/auth session, devwits-auth main. Runtime/authority/public
contract owner: MCP/backend session in work-engine. New product runtime logic
is Rust; extending the existing React presentation does not transfer runtime
authority to it. The bookkeeping manual-host is an outer-workflow composition,
not automatically a second product dispatcher or target custody owner.
