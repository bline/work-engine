# Lifecycle, Evidence, and Proof-Adequacy Review

Role: fresh read-only lifecycle, evidence, and proof-adequacy specialist. This
artifact is diagnostic and advisory. It does not revise or accept the proposal,
select fixtures, establish semantic truth, change either proposal decision, or
authorize implementation or refresh judgment.

Subject: proposal revision
`1c6e0f19dfb35af1917defac56ac1d823458318d`, tree
`7ce27ed672fff9af2f787c5d61f717ee456ae58a`, and the bounded content set in
[`subject.md`](subject.md).

Readiness consequence: `revision_warranted`.

## Supported conclusions

- The backbone correctly separates stable claim/revision identity, mechanical
  impact nomination, opening/investigation/terminal judgment, causal
  attribution, and exact-revision reliance. In particular, `may_affect` cannot
  become semantic invalidation and neither a warning nor a new claim revision
  silently advances reliance (`proposal.md:51-86`; parent `proposal.md:109-202,
  279-311`).
- Git-local canonical records and a non-owning projection are a proportionate,
  reversible first boundary. Requiring independently readable records, source
  identities, a declared completeness boundary, an empty-state rebuild, and
  projection provenance is sufficient for the bounded happy-path projection
  proof (`proposal.md:116-152,200-206`; parent `proposal.md:313-340`). The
  broader migration, duplicate, cycle, dangling-reference, partial-watermark,
  and stale-read matrix remains truthfully unproved.
- Real source material is plausible. The research claim that an initially
  isolated reviewer may be retained through bounded remediation existed before
  the `fca5cbe6` change (commit `7959347`,
  `ideas/adaptive-specialized-review-panels.md:154-165`). A real review finding
  about context lineage also existed at `7959347`
  (`reviews/proposals/adaptive-specialized-review/5367d9a/lifecycle-evidence.md:47-49`).
  Public commit `fca5cbe6` implemented retained
  reviewer sessions, and accepted checkpoint `b54d9b9` has the same tree
  `5c8f7119...`. The later retained review records the finding closed at the
  proposal-semantics level
  (`reviews/proposals/adaptive-specialized-review/fca5cbe/lifecycle-evidence.md:59-62`).
  These facts support feasibility; they
  do not pre-adjudicate which event caused a semantic conclusion to change.

## Findings

### 1. High — the unchanged terminal path does not require refresh authority

Proof 3 requires only an *attributed* `retained_unchanged` judgment, while it
requires the changed judgment to be *authorized* (`proposal.md:67-78`). Both
outcomes establish a new canonical support revision. The parent contract
therefore gives every refresh episode an authorized owner and every terminal
outcome an attributed judgment (`parent proposal.md:236-252`). Attribution is
not authority: the research source explicitly says its general research-refresh
owner is unresolved (commit `1c6e0f1`,
`ideas/research-maturity-evidence-snapshots-and-staleness.md:236-285`).

Consequence: the exercise can currently pass the unchanged path using a role
that is identifiable but not authorized to refresh the claim. An execution
disposition authorizing artifact construction would not silently grant that
semantic authority. Require both terminal paths to bind the producer, an
authority reference effective for that exact claim/domain/outcome, and the
evidence establishing the grant. Keep execution authority, original-judgment
authority, refresh authority, and downstream reliance authority separate.

### 2. High — the proofs do not require both domains to exercise the shared core

Selection requires one research claim and one review finding, but proof 1 says
only “at least one” identity, proof 3 merely requires two terminal paths, and
proof 4 requires one reliance record. Nothing maps either fixture to the four
proofs (`proposal.md:46-105`). An implementation could exercise every
load-bearing transition on the review finding, serialize the research claim as
an otherwise unused record, and still report success “across proposal research
and review” (`proposal.md:221-228`).

Consequence: a passing result would not falsify the alternative that the common
fields work only for one domain. Add a closed proof-coverage table to the
fixture-selection contract. Each fixture should exercise the claimed shared
identity, provenance/authority, exact-revision, nomination/refresh, and
reliance fields; the two different terminal outcomes may be allocated across
the fixtures. Domain-specific fields and any unexercised cell must be reported,
not inferred from shared names. This does not require the full parent dogfood
boundary.

### 3. Medium — stable-identity proof does not test its fork rule

Proof 1 asserts both continuity and the rule that a material subject, question,
quantification, or authority-domain change forks identity, but its executable
minimum requires only two revisions whose identity fields remain constant
(`proposal.md:51-57`). The only explicit fork exercise is in the parent’s
adversarial lifecycle matrix (`parent proposal.md:466-487`), which this proposal
defers (`proposal.md:157-176`).

Consequence: a representation that always reuses an identity can pass the
stated backbone fixture while violating half of proof 1. Either narrow the
backbone claim to continuity only and label fork behavior unproved, or add one
bounded counterfixture that changes exactly one identity-defining field and
must create a new stable identity plus an explicit cross-identity relationship.
This single negative case is causally inseparable from the asserted proof; the
rest of the adversarial matrix is not.

### 4. Medium — prebinding records does not prevent retrospective cherry-picking

The selection artifact is bound before dogfood records are produced, but
historical fixtures are expressly allowed and their outcomes are already
visible (`proposal.md:88-110`). Binding a known favorable pair before encoding
it prevents output mutation; it does not prevent selection after seeing the
semantic outcomes. The plausible `7959347`/`fca5cbe6` examples above illustrate
both the benefit and this limitation.

Consequence: the exercise can prove representability of selected cases, but
cannot on that basis claim outcome-independent evidentiary strength. Require an
outcome-independent selection rule or bounded candidate population, record who
selected it and what outcome information they had, and preserve exclusions and
failed candidates. If purposive historical selection remains preferable,
describe the result truthfully as a two-case demonstration rather than an
anti-cherry-picked falsification.

### 5. Medium — real historical reliance is not part of fixture selection

Proof 4 says the consumer revision “used” an exact claim revision, yet fixture
selection does not identify a pre-existing consumer, its decision scope, or
evidence that it actually relied on the selected source judgment
(`proposal.md:80-105`). A dogfood-authored reliance edge can mechanically prove
exact targeting and non-advancement, but it cannot retroactively prove that a
historical proposal or review decision relied on a claim merely because their
texts correspond. The evidence report’s requirement to list records and manual
interpretation does not close that semantic gap (`proposal.md:140-152`).

Consequence: bind either (a) a real versioned consumer with direct reliance
evidence and authority provenance, or (b) a newly versioned dogfood consumer
that explicitly adopts the exact claim revision during the exercise. Label
retrospectively reconstructed reliance as an attributed inference, not observed
historical use. Then query the same reliance before and after nomination and
refresh to prove that only the derived warning changes.

## Hidden dependence on deferred lifecycle evidence

Only the fork assertion is hidden inside the explicitly deferred adversarial
matrix while also being claimed by a backbone proof. Duplicate/out-of-order
delivery, branching revision races, unresolved outcomes, authority loss,
inactive recipients, partial publication, retry recovery, compaction, schema
migration, corrupt projection inputs, and cross-owner delivery are not needed
to establish the four narrowed proofs. A bounded projection rebuild must still
bind its exact canonical input manifest, schema/build version, cutoff,
exclusions/failures, and output digest so “deterministic” and “complete” remain
falsifiable within this fixture.

## Evidence and limitations

Evidence mode was direct reading of immutable Git objects at `1c6e0f1`, plus
read-only inspection of the named historical Git objects and completion-commit
validation contract. `completion_commit.py:132-190` re-establishes receipt,
commit, checkpoint, parent, tree, and message facts; this supports source-event
binding but does not establish semantic impact or causality. No current
worktree content under `ideas/` was used.

Codebase Memory project `home-bline-code-work-engine` was at HEAD `1c6e0f1`.
Verify-tier coverage checks returned `no_recorded_issue`/`metadata_match` for
all relied-on paths, with no recorded partial or skipped files. The observed
coverage generation was `2026-08-23T01:50:12Z`; this best-effort signal is not
proof of completeness. I did not execute a dogfood schema, projection, refresh
judgment, or reliance query, and I did not inspect other specialists’ outputs
for this episode.

Persistence of this review is not new evidence and grants no proposal,
placement, refresh, reliance, or implementation authority.
