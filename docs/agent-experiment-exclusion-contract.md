# Agent experiment exclusion contract

Status: **implementation authorized by the user, 2026-10-23**. The first
[filesystem adapter](agent-experiment-control.md) implements managed artifact
and mailbox delivery, membership and handoff state. Its capability table names
the host surfaces it does not control. This contract installs no skill, enrolls
no session, and changes no current campaign or R2 restriction.

## 1. Purpose and authority

An experiment can preserve its declared exposure and participation conditions
while unrelated agents, skills and repository work continue to evolve. Its
isolation claim remains inspectable across dispatches, interruptions and
handoffs. A global release freeze is unnecessary when the protected experiment
can actually retain its required environment.

An **exclusion** names an actor or population, an information or effect boundary,
the operations excluded, and its lifetime. It is not a blanket instruction to
ignore other agents. Excluding an actor from a treatment, from dispatch, from
unblinded evaluation, and from modifying shared resources are different rules.
Excluding a result from an analysis is a measurement decision, not membership.

The experiment owner defines and authorizes the experiment specification within
the user's authority. The specification names who can admit participants, amend
conditions, adjudicate incidents, score results, and close protected phases.
These roles may coincide unless the experiment's independence claim forbids it.
A storage write, board claim, session identifier or successful check grants none
of these authorities. Existing workflow and human approval boundaries remain.

[DESIGN.md](../DESIGN.md) governs this contract. Experiment-specific rules
belong to their accepted specification; this contract does not select a model,
statistical method, universal blinding requirement or automatic void rule.

## 2. What is bound

The following distinctions must survive context loss; their physical encoding
is an implementation choice.

| Identity or record | Meaning |
| --- | --- |
| Experiment and specification revision | Exact question, conditions, permissions and decision authorities in force |
| Experimental unit and assignment | What is assigned to a treatment: a task, session, team, branch or another declared unit; not necessarily one agent |
| Arm and phase | Treatment membership and the phase in which a rule applies; a single-arm experiment is valid |
| Agent branch | A logical line of responsibility across descendants and successors; independent of a Git branch |
| Participant binding | Host identity, session/thread and child identity where needed, role, unit/arm/phase, branch and effective environment |
| Environment revision | Exact instruction, code, tool and configuration sources required by the experiment, plus declared variable or unpinnable factors |
| Ownership generation | Current right to perform the named controlled operations; distinguishes a successor from a stale predecessor |
| Exposure and action evidence | What was supplied, observed or done, by whom, with provenance and stated observation limits |
| Intervention and disposition | A change or incident, affected units and the owner's decision about continued work and measurement |

The specification is immutable by revision. Amendments create new revisions
with explicit applicability; they do not rewrite which conditions governed
earlier work. Allocation rules and their evidence are owned by the experiment,
not inferred from a session name or directory layout.

Membership is explicit before protected activity. A parent relationship alone
neither admits a child nor proves its environment. Descendants inherit the
branch's constraints; a different role or environment requires an admission
allowed by the specification. A service or human outside the agent tree that
can affect the measured work must have a declared interaction boundary too.

## 3. Scope of the exclusion policy

For each protected boundary the specification identifies **who may perform
which operation on which information or resource, during which phase, under
what conditions**, and what evidence establishes compliance. Rules may cover
classes of artifacts or actors; they need not enumerate every future message.

Relevant boundaries include:

- **Instruction exposure:** skill listings as well as bodies, system and role
  instructions, repository instructions, prompts, tool schemas and injected
  updates. If a host-controlled surface cannot be pinned, declare the limit and
  whether the experiment can proceed with that uncertainty.
- **Context and memory:** forked conversations, compaction summaries, handoffs,
  retrieval indexes, persistent memories and earlier participation. A fresh
  session identifier is not evidence of an unexposed context.
- **Communication and observation:** board and direct messages, control events,
  tool results, notifications, dashboards, transcripts and access to arm labels
  or scoring material. Topic names and filenames can themselves disclose labels.
- **Actions and outputs:** dispatch, mutation, publication, integration,
  acceptance and access to another unit's artifacts. An unscored helper may
  still influence a scored unit through its output.
- **Shared dependencies:** mutable code/data, caches, build locations, rate
  limits, quota and other resources that can couple arms without sharing text.
  Declare material sharing and its permitted effects; the contract does not
  promise to eliminate provider or machine-level interference it cannot observe.

Every protected surface needs a policy; an unspecified surface is unresolved,
not implicitly unrestricted. Within a declared surface, access requires a
matching permission. Unrelated work outside that scope remains unaffected.
Conflicting applicable experiment policies require disposition before the
affected operation; an agent cannot select whichever rule is less restrictive.

## 4. Different experimental uses

These are examples of applicability, not a closed role taxonomy. Permissions
follow the required comparison and information flow, not the role title.

| Use | What the specification must distinguish |
| --- | --- |
| Worker instruction or model comparison | Each unit's baseline and treatment; whether a common dispatcher may adapt packets using observations from both arms |
| Coordinator or whole-team comparison | The coordinator and relevant descendants form part of the unit; cross-arm planning or shared retained helpers can mix treatments |
| Reviewer/scorer comparison | Candidate artifacts and permitted rubric are inputs; arm labels, author reasoning, earlier verdicts or reference answers may be withheld according to the claim |
| Blind scoring outside the arms | An evaluator may inspect outputs from several arms without becoming their participant; mapping and unblinding have separate permissions and timing |
| Scripted counterpart or environment | The response policy is bound; identical policy need not produce identical replies when agents take different actions |
| Shared tool, helper or observer | Allowed inputs, outputs, retained state and feedback routes are declared; receiving all arms' logs does not authorize sending advice back |
| Longitudinal or crossover experiment | Reuse and carryover are intentional design variables, with phase/order rules; changing an arm label does not erase prior exposure |
| Handoff/recovery experiment | Replacement and inherited material are themselves treatment dimensions; do not force a fresh-context route if that would change the experiment |
| Live dogfooding or single-arm observation | Allowed operational corrections and external interactions are recorded; isolation alone supplies no control group or causal attribution |
| Concurrent or nested experiments | All applicable constraints and interactions are declared; nesting does not automatically establish compatibility or statistical independence |

A retained agent spanning arms is not a neutral relay merely because it has the
same instructions. Its exposure and adaptive decisions are part of the design.
The owner can permit them, use separated contexts, or narrow the resulting
claim. No route is automatically sufficient for an independence claim.

## 5. Admission, actual exposure and continuing operations

Before launch supplies protected material, the launcher resolves the exact
specification and environment revision and checks that the intended role and
unit are permitted. Pinning requires materializing the named bytes or equivalent
immutable sources; a symlink to a changing global skill directory is not a pin.
The experiment's code baseline may evolve through its own authorized work;
updates from outside that lineage require the declared import permission.

When a host assigns identity only after starting a session, preparation and
identity attachment are distinct. The host must preserve the binding through
startup and withhold further protected activity until identity is verified.
If startup already exposes forbidden material, later enrollment cannot repair
that exposure. The adapter reports when it cannot satisfy this boundary.

Evidence distinguishes the **intended environment**, **material supplied by the
host**, and **material observable in the participant's transcript or context**.
A manifest proves intended inputs; a file digest proves bytes; neither alone
proves delivery. The specification defines which evidence is required, including
whether it must be observed independently of the participant. Unknown exposure
cannot count as verified nonexposure or be silently scored as a control.

Each governed launch, delivery, read or effect resolves the applicable current
binding and authority. Implementations must prevent an obsolete authorization
check from admitting an operation after a conflicting ownership or policy
transition; checking at listener startup alone is insufficient. The admission
point, operations already in flight and their disposition must be observable.
A transition cannot revoke information already delivered or undo an effect.

Invalid, missing or unsupported bindings stop the affected managed operation
with an explicit reason. They never select the global environment or board as
a fallback. This is not a global halt or permission to abandon unrelated work.

## 6. Communication and outside interventions

The boundary is what reaches an actor's effective input, not just the messages
its application elects to handle. Filtering forbidden content after including
it in a tool response is too late. Broadcasts, direct messages, control telemetry
and queued/replayed messages follow the same applicable exposure policy.

Messages and imported artifacts retain source, intended audience, relevant
binding revision and delivery disposition. Separate publication, admission,
delivery, acknowledgment and the required consequence; none proves the next.
A blocked payload is not echoed into the recipient's rejection diagnostic.

Cross-arm or outside information can be admitted when the specification allows
it. Transformations such as removing labels or preparing a blinded artifact
record their source and resulting artifact identity. Record who can inspect
the mapping; do not give a blinded actor the full policy manifest if that
manifest itself reveals what is withheld. A neutral binding handle can suffice
for its tools, with a role-scoped instruction projection.

Where correctness depends on agent compliance, the agent receives its applicable
constraints, decision authority, reason and concrete failure consequence before
the governed behavior. The projection must preserve those meanings without
revealing withheld information. A hidden manifest or an unexplained "experiment
mode" flag cannot substitute for that instruction exposure.

Operator corrections and necessary operational stops remain possible. Record
their authority, timing, content exposure, affected units and whether they fit
the registered intervention policy. Human authority can change the work; it
cannot retroactively make an intervention absent. Urgent stopping must not wait
for measurement bookkeeping; record the event as soon as the stop permits.
The measurement owner decides whether the result remains eligible, needs a
separate cohort or loses a claim under the experiment's rules.

## 7. Handoffs, forks and recovery

The branch identity, applicable specification/environment, prior exposure,
pending obligations and admitted continuation sources survive a handoff.
Preserving a branch does not mean copying all of the predecessor's conversation.
What is inherited follows the experiment's declared treatment and the owning
workflow's continuation requirements.

A handoff binds the predecessor, intended successor, exact continuation
artifacts, unresolved processes/children and the operations being transferred.
The successor's admission must be established before it takes those operations.
An atomic ownership transition advances the generation; a stale predecessor
cannot continue those managed operations. Remaining capabilities, such as
answering a provenance question, are explicit. Preparation and activation are
distinguishable so a failed launch leaves recoverable pending state.

The continuation preserves message selectors and replay positions together,
including admitted but undelivered work. Session-bound control telemetry is
rebound to the successor's actual identity; predecessor counters and cursors
cannot become the successor's telemetry baseline. Do not replay queued content
into a different exposure scope without evaluating that scope.

Live process and child ownership follows the host's actual capabilities. Where
it cannot transfer, the workflow must collect or quiesce it before claiming the
transfer complete. Experiment membership does not make another session's child
handles usable. Existing root-transfer authority remains unchanged.

Compaction, restart, relabeling, removal from a branch and re-enrollment do not
erase exposure history. A handoff can itself carry contamination, including a
summary written by an actor exposed to both arms. Provenance checks establish
sources, not semantic freedom from influence; the experiment must state any
stronger evidence or isolation it requires.

## 8. Filesystem state and the enforcement limit

No daemon is required. Host commands consume durable state from the filesystem.
The existing [durable-state capability](../skills/durable-state/SKILL.md) supplies
atomic publication, integrity checks and retained revisions; it owns storage
mechanics, not experiment meaning. The [adapter guide](agent-experiment-control.md)
defines the implemented key layout and CLI. Views or exported JSON must not
become a second writable authority.

Immutable specifications and evidence references are separate from changing
membership and ownership. Related transitions must be atomic at their semantic
boundary; separately writing an "old owner inactive" file and a "new owner
active" file cannot be treated as an atomic transfer. Concurrent writers use
revision checks or equivalent serialization. A rejected stale update preserves
the accepted state and reports the conflict.

Filesystem notifications are wake hints. After replacement, missed events or
restart, clients reread authoritative state. No actor infers release of an
exclusion from silence, a missing watcher or an expired advisory claim. The
absence of a daemon also means time alone does not execute cleanup or close an
experiment; a named actor must reconcile any time-based condition.

The implementation declares coverage for each protected surface:

- **Enforced at a managed boundary:** the named adapter can reject the operation
  before protected exposure or effect, including stale ownership.
- **Cooperative and observed:** participants agree to the rule and evidence is
  collected, but the host does not prevent bypass.
- **Uncontrolled or unobserved:** the limit and its consequence for the required
  experiment claim are explicit.

These classifications can differ within one experiment. A shared filesystem,
private Git ref, worktree, session ID or topic filter is not a security boundary.
Agents with unrestricted access may bypass the CLI, read other arms' files, or
edit its state. Claims requiring prevention of that conduct need stronger host
enforcement or must remain unestablished. Independent review/scoring contracts
are not satisfied merely by placing actors in different directories.

## 9. Change, incident and end conditions

Prospective changes name their authorizing owner, new revision, affected units
and effective boundary. A change to frozen instructions, a permitted feedback
route or an observation instrument can change the experiment even when product
code is untouched. Preserve both the old and new conditions and their evidence.

On a violation or uncertain exposure, preserve the incident and prevent further
affected managed operations until the specification's incident rule or its owner
disposes them. Do not erase a trial, silently retry it as fresh, exclude an
unfavorable result, or downgrade the required claim on the participant's own
authority. Record replacements and exclusions so attrition remains visible.
An owner's decision can permit work to continue without certifying the original
comparison. Nothing here forces the same incident policy on every experiment.

Each protected phase has an explicit end predicate and an owner who records
whether it holds, with evidence. Distinguish **work execution ended** from
**evaluation or blinding ended**. Builders may be released while evaluators
still require blinded artifacts. A final reviewer outside the measured execution
need not extend that execution phase unless the specification says it does.

Closing a phase stops new admissions under it and identifies which bindings and
exclusions are released, retained or superseded. The closure decision accounts
for pending launches/transfers, admitted participants and operations in flight;
each is completed, cancelled or explicitly reassigned under permitted conditions.
An earlier admission cannot silently activate work after its phase has closed.
Releasing an agent for ordinary work is explicit; records stay attributable to
their earlier conditions.
Cleanup, publication and removal of retained evidence require their own existing
authority. Ending an experiment does not itself authorize any of them.

## 10. Observable acceptance cases for an implementation

These cases check the contract boundary, not experimental effectiveness.

| Case | Required observable outcome |
| --- | --- |
| Global skill changes during a frozen trial | The trial retains its bound bytes; ordinary work can receive the update; actual exposure evidence remains attributable |
| Child or fresh successor starts | Applicable constraints and intended environment survive; actual identity is attached; unverified admission cannot proceed silently |
| Evaluator receives a blinded result | Only permitted inputs and metadata reach it; withheld arm mapping is not leaked through bindings or rejection output |
| Shared helper retains information from both arms | The design permits and records that influence, or the protected use is refused; an unscored helper is no exemption |
| One arm changes its own code | Authorized within-arm evolution proceeds; importing it into another arm requires that arm's permission |
| Owner changes while delivery is queued | Admission orders delivery against the transition; obsolete permission cannot silently deliver under the new binding |
| Two actors attempt the same handoff | At most one successor gains the transferred operations; the loser sees a conflict; prior exposure is preserved |
| Crash between preparation and activation | Recovery identifies the pending transfer and current owner without assuming activation or falling back globally |
| Forbidden broadcast or direct message | Managed delivery blocks it before model-visible output; an uncontrolled bypass is reported as a limitation or incident |
| Quota, cache or mutable artifact is shared | Declared sharing and any observed intervention remain visible; path separation alone cannot establish independence |
| Exposure evidence is missing | The required isolation claim remains unestablished; the owning incident/measurement rule supplies the disposition |
| Emergency correction is necessary | Authorized correction/stop remains possible and its experimental consequence is recorded |
| Execution ends before scoring | Specified worker exclusions can end while scoring/blinding exclusions remain; silence or claim expiry cannot release either |
| Phase closes with a prepared launch outstanding | The pending admission is disposed explicitly; the old launch cannot later activate under the closed phase |
| A prior participant enters a crossover phase | Its earlier exposure and phase order remain part of the record; a new label cannot manufacture an unexposed control |

## 11. Relationship to current work

R2 supplies motivating evidence, not universal experiment policy. Its
[standing skill-change rule](../metrics/probes/claude-builder-r2-prereg-20261022.md#standing-rule-for-further-skill-changes)
and amendment 5 explain why worker exposure matters; its owner retains R2's
actual end-condition and amendment decisions. This contract does not lift its hold.
The [coordination pilot](../metrics/probes/coordination-skill-pilot-prereg-20261023.md)
supplies examples of pinned arms, scripted counterpart policies, blinding and
per-trial exposure evidence. Their particular scoring rules are not inherited.

Current [chatboard wait](codex-chatboard.md) and proposed manual control
subscriptions provide observation and transport capabilities, not this
exclusion mechanism.
The [context-boundary reference](../skills/manual-orchestration/references/context-boundary.md)
continues to own manual handoff behavior. Role environment graphs are projections
of their existing owners; this document introduces no alternate role authority.

The first implementation exposes registration, admission, managed delivery,
ownership transfer, intervention and phase closure with inspectable evidence.
Its guide defines the command API and filesystem layout, and distinguishes
implemented boundaries from host-specific admission hooks still required. A
host capability assessment must establish which surfaces it can enforce before
advertising those isolation guarantees.
