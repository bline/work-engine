# Slice 1 Accepted Plan: Source-Bound Intake Record Vertical

## Acceptance

- Run: `5a74bd5c-3ebc-4e6e-9d93-07dd96aede5a`
- Slice: `1`
- Plan: `accepted-v1`
- Acceptance: `procedural_auto_approval`
- Work source: `proposals/idea-to-proposal-system/raw-idea-intake/proposal.md`

## Bounded slice

Create the additive `skills/idea-intake/` capability, its version-1 closed
record/profile and validator/projector, and one representative Git-backed
semantic vertical. It turns one authorized immutable raw-idea revision into a
validated, attributed, claim-addressable assessment and a bounded surviving-
candidate projection.

A consumer can distinguish mixed per-claim outcomes and consume a surviving
candidate without rereading the complete ideas collection or reconstructing
the intake conversation.

This slice does not create production intake data, move raw ideas, form or
decide a proposal, alter proposal-packet contracts, accept shared claim-lineage
placement, or settle a production `idea-intake/` versus `ideas/intake/` record
layout.

## Placement and route

- Route: `falsified-placement`
- Placement risk: `medium`
- Placement verdict: `confirmed`
- Capability owner: new sibling `skills/idea-intake/`
- Record instances: fixtures and temporary Git repositories only

Rejected alternatives:

- `skills/proposal-former/` owns formation of surviving meaning, not raw
  interpretation or assessment history.
- `skills/proposal-packets/` owns formed packet identity and shape, while
  intake can truthfully yield zero proposals.
- shared claim-lineage remains an experimental possible dependency and does
  not own intake disposition, source movement, or readiness.
- `ideas/` alone is insufficient without a validated distinction between raw
  source and attributed assessment; choosing a production hierarchy is
  deferred.

## Accepted semantic-path certificate

- Trigger: an authorized assessor selects one raw idea and exact immutable Git
  revision for intake.
- Producer: `skills/idea-intake` identifies independently addressable claims
  and produces an attributed record.
- State owner: the intake record profile owns stable idea and assessment IDs;
  exact source commit, blob, path, range, and content integrity; assessment
  revision, producer, authority, and evidence cutoff; typed claim revisions;
  nominations versus adjudications; per-claim dispositions; uncertainty and
  reopening conditions; proposal references; and explicit non-authorization.
- Consumer: proposal formation receives one bounded surviving-candidate
  projection plus named on-demand evidence.
- Lifecycle: bind and validate an assessment against one immutable source;
  retain revisions through Git; later current-path movement does not retarget
  the assessment; cleanup remains unapplied.
- Semantic consequence: duplicate, promoted, apparently implemented, novel,
  unresolved, and no-proposal outcomes may coexist per claim while surviving
  candidate meaning remains independently consumable.
- Downstream proof: a temporary-Git vertical commits a raw source, binds its
  exact objects and range, validates a mixed assessment plus manual prior
  evidence, emits the bounded projection, moves the current raw path, and
  proves the original source and stable intake-origin reference remain
  resolvable.
- Insufficient substitute: prose-only skill, one file-wide disposition,
  current-path existence, rewritten source, packet origin alone,
  unadjudicated similarity/code matches, or coupling to experimental
  claim-lineage.

## Invariants

- Assessment never rewrites raw source bytes or silently retargets an existing
  subject after edits or movement.
- Stable IDs are unique and source object/range/content integrity is checked.
- Assertions, interpretations, observations, nominations, adjudications, and
  human decisions remain typed and attributed.
- Claim outcomes do not collapse into one file-wide status; zero-proposal is
  valid.
- Manual reconciliation is attributed prior evidence, not a competing owner
  after finer-grained assessment exists.
- Validation establishes only closed shape, references, identity, and
  non-authorization—not semantic equivalence, implementation, readiness, or
  correctness.
- Intake does not authorize cleanup, implementation, proposal acceptance,
  permanent placement, or roadmap change.
- The candidate projection is derived convenience state, not a canonical
  owner.
- Agent instructions describe outcomes and affordances rather than a fixed
  questionnaire or universal state sequence.

## Expected task-owned boundary

New paths only:

- `skills/idea-intake/SKILL.md`
- `skills/idea-intake/agents/openai.yaml`
- `skills/idea-intake/references/intake-contract.md`
- `skills/idea-intake/schemas/intake-record-v1.schema.json`
- `skills/idea-intake/scripts/idea_intake.py`
- `skills/idea-intake/tests/test_idea_intake.py`
- minimal representative fixtures below
  `skills/idea-intake/tests/fixtures/representative/`

No baseline overlap is expected. Pre-existing dirty or untracked proposal,
roadmap, metrics, campaign, planning, and unrelated idea files remain outside
the task-owned boundary.

## Implementation and semantic proof

1. Define the minimum closed schema/contract and short skill entrypoint.
2. Implement repository-contained validation and deterministic projection with
   exact Git source binding, uniqueness, endpoint, authority, and non-effect
   checks.
3. Establish the temporary-Git vertical before broadening implementation. If
   move-safe resolution or the consumer projection cannot be demonstrated from
   this owner, return a boundary-change request.
4. Add negative cases for tampered or stale bindings, unknown/missing fields,
   duplicate IDs, dangling endpoints, nomination-as-adjudication, file-wide
   collapse, and cleanup/implementation-authority overreach.
5. Complete concise metadata and contract documentation without adding
   production records or sibling edits.

## Validation mapping

`semantic_proof`:

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest skills/idea-intake/tests/test_idea_intake.py -v`
- The module must contain the end-to-end temporary-Git move/projection proof.

`risk_proportional_checks`:

- `python3 /home/bline/.codex/skills/.system/skill-creator/scripts/quick_validate.py skills/idea-intake`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest skills/proposal-former/tests/test_proposal_former.py skills/proposal-packets/tests/test_proposal_packets.py -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile skills/idea-intake/scripts/idea_intake.py skills/idea-intake/tests/test_idea_intake.py`
- Run a revision-bound `agent-instruction-review` against the exact immutable
  candidate/checkpoint commit, tree, manifest, and content digests for the
  skill entrypoint, agent metadata, and normative contract clauses. It is
  advisory, cannot self-certify, remains separate from implementation
  acceptance, and returns exact-subject applicability/findings under its
  finding contract. The builder owns remediation and the retained instruction
  reviewer re-evaluates any exact delta.
- Run a fresh configured authority-bound Claude adversarial-review episode
  after deterministic checks against the exact task-owned subject and accepted
  certificate, retaining the reviewer through remediation. This separately
  challenges code, schema, runtime semantics, provenance, failure behavior,
  tests, and overall slice correctness; it does not substitute for instruction
  review.
- The requirement passes only after focused checks and both distinct review
  consequences complete with every accepted blocking finding resolved.

`workspace_integrity`:

- Compare the exact task-owned manifest and workspace status with the baseline.
- Run `git diff --check --` over task-owned paths.
- Confirm no unrelated path or pre-existing overlap was modified.

The broad repository suite is initially omitted because the slice is isolated
and adds no production registration or sibling modification. Add it if shared
runtime changes appear or focused evidence reveals wider coupling. No
production generated-artifact freshness stage is expected.

## Deferred decisions and scope

- production intake record placement and publication;
- shared claim-lineage integration or acceptance;
- behavioral fresh-model proposal-formation dogfood and immutable formed-
  packet publication-before-review;
- broader representative intake cases and complete reconciliation migration;
- real raw-idea cleanup, movement, redirect, or deletion;
- concurrent/external storage, UI/inbox, scheduler, or continuous assessor;
- proposal review, evaluation, decision, priority, or implementation semantics;
- the separate supervisor-state structural discussion.

No product, ownership, architecture, or authority decision remains open for
this slice.
