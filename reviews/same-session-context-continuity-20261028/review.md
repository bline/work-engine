# Advisory instruction review: same-session context continuity

Reviewed 2026-10-28 by Codex session
`01a1839d-27bf-7882-b2ac-0233b11805f1` using agent-instruction-review. This is
author self-review in retained context, not independent review, architecture
acceptance, implementation approval or runtime verification.

## Applicability and exact subjects

Applicable: the proposed insertion defines materially normative host behavior,
and the changed manual-orchestration reference governs coordinators and
supervisors. [subject.json](subject.json) binds exact source digests and ranges;
[subject.md](subject.md) preserves their original bytes as excerpts. Relative
links in those frozen excerpts retain their original source coordinates.
The frozen excerpt file's SHA-256 is
`64a6648eb5a4a1c31d44e01af4a148908e236b7ebf9c42c4d85eb63984d28d5d`.

The retained re-evaluation is bound to the exact edits and new full-source
digest in [remediation.json](remediation.json). The final proposal source is
`15f544e8fbbecbdf41216764a8b0a77700037093a9d9024fff6a9277e075c6f6`;
the guidance remains
`9a40fc07faaae664e54071b01a479793e7d3b375829ca40b95d81f276daa8da2`.

## Advisory findings and disposition

### SSC-IR-1 — low — restate — resolved

The original authority paragraph in proposal source lines 55-59 required the
host to resolve authority before actuation and "again" before resumption.
The protected distinction is current authority at both effect boundaries;
the exact repeat-resolution route is not necessary when revision fencing can
establish that authority remains current. Requiring a second resolution risks
turning that outcome into redundant procedure.

The semantic owner is the lifecycle authority contract, its consumer the
implementing host, and its authority source DESIGN sections 3.3, 3.5 and 11.
The paragraph exposes its causal parent: a checkpoint cannot issue authority.
Confidence is high about route ambiguity, not evidence of a runtime defect.
The author accepted `restate`. The exact replacement requires current authority
at both boundaries and invalidates stale admission when authority changes,
without prescribing a second lookup. Retained re-evaluation resolves the issue.

### SSC-IR-2 — medium — restate — resolved

The original Recovery and effect custody paragraph, proposal source lines
82-84, said the host owns "one durable transition operation" without stating
the unit. The existing transition protocol's One durable transition owner
section scopes exclusivity per logical role instance. A host-wide singleton
would unnecessarily restrict unrelated roles and could obscure the actual
admission owner.

The semantic owner is the existing transition protocol; the consumer is the
host implementer. That section supplies the authority and scope evidence.
The draft paragraph exposes replay/duplicate-effect failure, but initially
omitted this scope from the implementing agent's effective text. Confidence is
high about the omission, not evidence that any implementation uses a singleton.
The author accepted `restate`. The exact replacement names each transitioning
logical role instance. Retained re-evaluation resolves the issue.

## Remaining bounded advice

No additional unresolved instruction finding in the inspected subject. The
manual projection keeps Wind Walker's semantic prerequisite separate from
effect authority, retains the human root-transfer boundary, explains wakeup
authorship and recovery evidence, and preserves collection ownership. Its
effective loading is the existing manual-orchestration coordinator/supervisor
reference; detailed host requirements remain in candidate design rather than
worker startup context. The new text carries the relevant reasons and failure
distinctions without requiring a particular controller, transport API, pressure
threshold, approval cadence or receipt schema.

Pilot case count and route order are local research choices, not general
runtime instructions. This review does not establish provider semantics,
checkpoint completeness, host admissibility, net benefit or contract
acceptance. Those remain with the named owners and prospective pilot evidence.
