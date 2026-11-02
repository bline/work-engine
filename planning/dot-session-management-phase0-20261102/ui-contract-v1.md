# UI/auth boundary — phase 0, revision 1

Status: proposed consumer contract for source-owner review, not an implemented
execution API. Scope: devwits-auth main fffd8f0, exact owner-answer/v1 bytes,
and the future typed task/mandate review surfaces. No production code changed.

## Existing authority and transport

The board is the existing decision source. Redis and the UI are transports and
projections. Signing in identifies the owner; it does not authorize a task.
The browser signs the exact UTF-8 answer_json challenge. That answer binds the
full append-only request row through request_digest; verifiers must not
re-serialize either signed bytes or the authoritative request bytes. The pinned
SPKI/credential registry is the consumer trust source, not registration alone.
The final acting consumer rechecks proof, owner/RP/origin, expiry, current task
meaning, key trust and one-use admission. A mailbox verified flag is only data.

Reference: devwits-auth docs/owner-answer-v1.md, api/answers.js, api/_lib.js
checkAnswer, laptop/owner_answer.py verify_answer, and approval_sync.py pull.
Auth is not indexed in the current graph; direct bounded source reads were
used after check_index_coverage reported project unavailable.

## Required review projection

The current generic approval page shows ask, body and request metadata, not a
validated execution terms form. A signature over hidden data is insufficient
to claim the owner reviewed executable terms. A typed task review must display
objective and revision; operation; provider/model or bounded profile; workspace
and source basis; allowed effects/paths/network; all-actor budget units/amount
and reserve; time/expiry; steering/delegation limits; and reserved decisions.
Display values from the same validated object whose original row bytes the
signature binds. Reject or hold unsupported versions and unrenderable terms.
An agent-supplied prose summary cannot replace authoritative structured terms.

The current answer-v1 envelope can carry this binding without changing its
fields: include the versioned intent in the request row. A new intent schema is
a product contract to qualify; the fixture name below is explicitly a proposal,
not an accepted runtime wire version. Task revisions are owner-issued state
identities; no second generic canonicalization is silently introduced.

## Approval consumption after a board reply

The transport verifies against open requests before posting the signed reply,
and an approve/reject reply closes that request. At later action time, the
consumer must distinguish answered-by-this-exact-proof from withdrawn,
superseded, conflicting or previously consumed state. Do not blindly require
the row to remain open after its own answer closed it. verify_answer accepts
the caller-supplied original row; its caller owns current lifecycle eligibility.
Bind answer_id to one admission/operation durably and atomically. A repeated
transport delivery returns that operation; a lost native ACK never permits a
second launch merely because no final result is visible.

Do not discard a signed comment. Conditional or ambiguous owner intent must be
reconciled with typed bounds before effects. The first qualified profile can
be limited to unconditional, empty-comment typed approvals, holding other
approved comments for owner/model interpretation rather than pretending to
understand arbitrary conditions. This is a proposed narrow qualification, not
a universal ban on comments. Reject/defer/ask do not authorize a start.

## Public view and action boundary

The UI consumes versioned public task, decision, operation/result and mode
views. It never chooses an executable/socket/config file, forges admission,
interprets a transport identity token as a human grant, or reads a private
capability store. The backend owns task revisions, admission/grant resolution,
budget reservations, native custody and observed settlement. A UI-selected
profile/workspace is a validated permitted identifier, not arbitrary shell/RPC.

Task approval may explicitly authorize launch and subsequent in-scope steering;
it must say so. Current exact-message diagnostic grants cannot silently become
a broader task permission. Away adds explicit delegated task choices and bounds;
mode alone grants nothing. Mode/revoke transitions show the effect on queued
and in-flight work and must not display a cancellation request as termination.

## Acceptance data

ui-binding-vectors.json uses the existing request_json/request_digest reference
codec and exact answer bytes. It has no valid passkey proof, real owner key,
capability or execution authority. Expected outcomes in acceptance-cases.json
are semantic obligations for future tests, conditional on separately valid
cryptographic and policy evidence. No fixture is itself an allowed action.
Native invocation under all phase0 artifacts is zero.
