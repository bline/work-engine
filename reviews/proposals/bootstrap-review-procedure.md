# Bootstrap Proposal-Review Procedure

Status: provisional operating procedure for proposal review before an accepted
proposal-review product contract exists. It coordinates review work but does
not accept proposals, establish the proposed artifact schema, or authorize
implementation.

## Selection input and ownership

The proposal-review coordinator owns specialist selection. The proposal former
publishes an immutable candidate subject and a bounded projection of its present
claims, consequences, authority and loading effects, uncertainty, and evidence
needs. The former may identify materially normative agent-facing content as an
observed subject property, but it does not select or omit reviewers for text it
authored.

Select perspectives from the current subject's material consequences without
reading speculative future implementation into the proposal. Record selected
perspectives and materially plausible omissions with reasons. Availability is
not applicability, an omission is not a pass, and synthesis does not acquire
selection, finding, revision, or proposal-decision authority.

Give `agent-instruction-review` one explicit disposition for every formed or
revised proposal subject. Select it when current proposal content creates or
changes materially normative agent-facing text, an instruction-loading or
precedence contract, or an exact route claimed as mandatory. Omit it with the
absent present-day instruction consequence when the proposal only anticipates a
future instruction artifact. If that distinction is uncertain, select the
specialist and let its own applicability review resolve or preserve the
uncertainty.

A selected instruction reviewer starts from the exact immutable subject and
follows `skills/agent-instruction-review/SKILL.md` plus its finding contract. It
returns `applicable` or `omitted` and owns only its advisory findings. The
coordinator records the result, the proposal former decides whether and how to
revise, and the named proposal authority retains disposition authority.

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

When the authority-bound review-state surface is configured, it is part of the
ordinary retained-reviewer path for both Claude and Codex. Before the initial
provider call, the coordinator publishes an episode manifest binding the exact
subject, provider-specific resumable runtime reference, writer generation,
readers, and authority provenance. A Claude call receives the episode through
its narrow MCP surface. A Codex reviewer receives the same semantic profile
through the repository-local authority-bound adapter and is bound to its
predeclared canonical spawned target. The reviewer begins the episode, records
its attributed result, and reads it back before returning. This avoids a second
turn that only reconstructs and retranscribes findings.

Selected reviewers also receive equivalent bounded observational access to
exact active-slice history and validated claim-lineage projections when those
records are material to the review. Claude may receive them through MCP and a
spawned Codex reviewer through repository-local readers. Neither route grants
claim applicability, arbitrary durable-key discovery, or additional authority.

The coordinator verifies the provider's returned session or spawned-target
identity against the durable binding. The reviewer does not self-certify runtime identity unless an
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

## Provider availability and fallback

A selected specialist perspective and its provider execution are separate
decisions. Provider unavailability does not let the executor omit the
perspective, change its finding contract, or report it as passed.

Treat Claude review execution as best effort when the governing configuration
does not require that exact provider or independent-review evidence. When
bounded recovery or retry for quota, infrastructure, or unavailable session
state no longer has credible value, the coordinator may execute the same
selected specialist in a fresh Codex Sol context through
`codex-adversarial-review`. Select its reasoning effort from the subject's
consequence, uncertainty, and review difficulty within configured limits rather
than fixing one effort for every subject.

The fallback starts without proposal-former, coordinator-synthesis, builder, or
prior reviewer reasoning. It receives the same immutable subject, specialist
contract, bounded evidence, and review obligation. Record the Claude attempt and
failure, fallback transition and reason, Codex model and actual reasoning effort,
fresh-context boundary, and `accepted_same_model_review` evidence class. Never
describe the result as cross-provider, cross-model, statistically independent,
or independent reasoning.

An explicit independent-review or exact-provider requirement remains binding.
Codex same-model review may provide additional advisory evidence but cannot
satisfy that requirement; preserve the obligation as unavailable or blocked, or
apply an authorized configuration amendment. Best effort keeps advisory review
from unnecessarily stopping development; it does not weaken a configured
acceptance condition.

## Reset and recovery

Reset is a model judgment, not the default response to a fix. Reasons can
include:

- a material architecture, placement, scope, or review-premise change;
- context that is degraded or too large to remain reliable;
- contamination that invalidates the claimed perspective; or
- a renewed need for genuinely fresh independent challenge.

Record the reason, preserve still-applicable findings, and bind the replacement
to the exact new subject. Provider failure alone first triggers bounded
continuation or recovery when possible, followed by the authorized fallback
above when the required evidence class permits it.

If a reviewer session is lost, reconstruct from the immutable subject, that
reviewer's durable findings, limitations, and the bounded remediation delta.
Label the result as a replacement or reconstructed episode; do not claim that
the original reasoning context survived.

## Panel coordination

The coordinator may add a newly relevant specialist when changed consequences
justify it. That specialist starts fresh and does not force existing specialists
to restart. Panel synthesis occurs only after current specialist consequences
are available and preserves conflicts rather than voting them away.

For every new immutable proposal revision, reconsider the
`agent-instruction-review` selection disposition from the revised present
content. Continue the retained instruction reviewer when the ordinary
remediation premise remains valid; select a fresh instance only when the skill's
reset conditions warrant it. A prior omission, applicability result, or finding
does not silently carry across a changed subject.

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
