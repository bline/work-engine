# Bootstrap Proposal-Review Procedure

Status: provisional operating procedure for proposal review before an accepted
proposal-review product contract exists. It coordinates review work but does
not accept proposals, establish the proposed artifact schema, or authorize
implementation.

## Reviewer lifetime

Each selected specialist begins in a fresh role-scoped context without builder,
proposal-former, coordinator-synthesis, or other reviewer reasoning. The
coordinator retains that reviewer identity after the initial finding artifact
instead of treating completion of one turn as retirement.

The ordinary remediation loop is:

```text
fresh isolated specialist
→ initial findings against immutable subject
→ proposal former prepares a new immutable subject or bounded delta
→ same specialist evaluates the delta and its prior findings
→ repeat while its context remains useful
→ obligation discharged, deferred, blocked, or reset with reason
```

Initial isolation establishes the fresh-perspective claim. A later pass in the
same reviewer context is a continuation of that episode, not another fresh or
independent review.

## Continuation packet

Give a retained reviewer only what is needed to judge remediation:

- the exact new subject identity and bounded delta from the prior subject;
- its own prior finding identities and unresolved limitations;
- the proposal former's concise remediation consequence;
- affected deterministic evidence; and
- any changed governing contract or premise.

Do not resend the complete initial research merely to reconstruct context the
reviewer already owns. Do not supply other reviewers' conclusions or the panel
synthesis unless resolving an explicit cross-review conflict requires that
information. If the context boundary changes, record the added information and
do not repeat the original isolation claim for that pass.

## Runtime bindings

For a retained Codex specialist, keep the spawned reviewer target and use
`followup_task` for remediation. For a Claude specialist, assign a persistent
session UUID on the initial call and use `--resume <session-id>` for later
passes. Disposable reconnaissance, placement mapping, and diagnosis sessions
must not be reused as reviewers.

When the authority-bound independent-review state surface is configured, it is
part of the ordinary Claude reviewer path. Before the initial provider call,
the coordinator publishes an episode manifest binding the exact subject,
reviewer UUID, writer generation, readers, and authority provenance. The same
initial call receives read-only evidence tools and only that episode's narrow
state transitions. The reviewer begins the episode, records its attributed
result, and reads it back before returning. This avoids a second turn that only
reconstructs and retranscribes findings.

The coordinator verifies the provider's returned session ID against the durable
binding. The reviewer does not self-certify runtime identity unless an
available tool actually exposes that observation. A launch manifest scopes the
writer but does not authenticate the MCP peer; trusted-launcher admission
remains a separate fact.

For a remediation pass, the retained reviewer receives the exact new subject
and bounded delta, records that remediation subject in its episode, evaluates
only its own applicable findings, and publishes the re-evaluation before
returning. Failed or rejected transitions remain failed attempts, not durable
review state. Preserve the structured reviewer output and report the state gap
truthfully until it is repaired or reconstructed.

Keep reviewer targets or provider session IDs available until the review
obligation reaches a terminal consequence. Session persistence is a runtime
optimization and provenance fact, not the durable owner of findings or
authority.

## Reset and recovery

Reset is a model judgment, not the default response to a fix. Reasons can
include:

- a material architecture, placement, scope, or review-premise change;
- context that is degraded or too large to remain reliable;
- contamination that invalidates the claimed perspective; or
- a renewed need for genuinely fresh independent challenge.

Record the reason, preserve still-applicable findings, and bind the replacement
to the exact new subject. Provider failure alone first triggers continuation or
recovery when possible.

If a reviewer session is lost, reconstruct from the immutable subject, that
reviewer's durable findings, limitations, and the bounded remediation delta.
Label the result as a replacement or reconstructed episode; do not claim that
the original reasoning context survived.

## Panel coordination

The coordinator may add a newly relevant specialist when changed consequences
justify it. That specialist starts fresh and does not force existing specialists
to restart. Panel synthesis occurs only after current specialist consequences
are available and preserves conflicts rather than voting them away.

The coordinator records, when observable:

- fresh-entry reviewer count;
- continuation-pass count;
- reset and reconstruction count with reasons; and
- provider tokens, cost, and elapsed time separated between initial review and
  continuation.

These measurements inform later workflow design. They do not redefine semantic
acceptance.

## Closure

Retire a reviewer only when its findings are resolved, accepted as residual
uncertainty by the proper decision owner, deferred or blocked truthfully, made
inapplicable by an authorized route change, or transferred to a replacement
episode with explicit lineage. A coordinator or persisted review artifact does
not itself exercise the proposal decision.
