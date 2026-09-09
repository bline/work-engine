# Production-Path Claim Evidence Audit — Round 1

## Status and authority

- Status: advisory audit result; not design activation, implementation authority,
  migration acceptance, or roadmap amendment
- Audit authority: the operator's direction to begin the production-path claim
  audit after S13 publication
- Implementation subject: local `main` at
  `b9ac323139ef807870ef1f9b21a972facbdec60a`
- S13 publication subject:
  `8abc57ba9b140b1c30367364572c8f10b55da842`, verified as an ancestor of the
  implementation subject
- Successor `DESIGN.md` SHA-256:
  `2213ab9a32925e7792353489599201bf71f3268eb8981cc192f87c57e2952095`
- `PHILOSOPHY.md` SHA-256:
  `299b0ad8192f3cc442f6491d2908f421f22333024ffb1423cae6856b69c3a307`
- Research architecture SHA-256:
  `651f8ca073b3c09dca2f8b5f63251ba3a13f98a5ca16b46841865609b7045599`
- Transition record SHA-256:
  `a36e5d0ba3a989dc669193a8ea354c0e575bc2f70e64a8b7fc4c6a5a8b5dc782`

The successor doctrine remains a candidate under its transition record. This
audit does not activate it. Findings identify where current contracts would be
sufficient or insufficient if the operator admits that exact successor.

## Scope and method

This round traces three real paths:

1. S13 Review Bench result validation and configuration comparison;
2. native specialist selection, execution, Review Episode admission, and Slice
   Campaign terminalization; and
3. canonical Git publication as a control case that already combines a
   constraint with independently owned observation.

The audit used task-directed Codebase Memory verification and exact source
inspection. Coverage was checked at graph generation
`2026-09-09T00:03:36Z`; every relied-on path reported no recorded issue and
matching metadata. That signal is best effort, not proof of completeness. The
configuration-key behavior was also exercised directly against an admitted
Review Bench result: changing only `fresh_context`, and changing only
`inference_family`, each produced the same configuration key.

This round does not cover paid comparisons, provider entry, historical AGY
recovery, cross-provider event normalization, S14 imports, context replacement,
or legacy retirement. It neither launched nor authorized any of them.

## Dispositions

### PPCE-01 — High — distinct Review Bench configurations are pooled

**Disposition:** remediation required.

`validateReviewBenchResult` makes `inference_family` and `fresh_context` part of
reviewer identity. The Review Bench artifact contract says the same. However,
`reviewBenchConfigurationKey` omits both fields while
`compareReviewBenchResults` uses that key to aggregate configuration and
route-class metrics.

Two attempts that differ only in either omitted field therefore enter the same
aggregate. The first result supplies the aggregate's displayed
`inference_family`, so the report can attribute pooled measurements to one
family even when observations came from more than one. Fresh and retained
contexts can likewise be pooled without the report identifying the mixture.

- Claim: a configuration-specific report describes attempts from the named
  reviewer configuration.
- Subject: each configuration aggregate in `review_bench_report_v1`.
- Covered state: configuration, route-class, and pairwise measurements.
- Consumption boundary: insertion into the aggregate selected by
  `reviewBenchConfigurationKey`.
- Consumer: later adjudication or research reasoning using the descriptive
  report.
- Authority source: the Review Bench experiment and artifact contracts.
- Constraint: exact result validation before comparison.
- Evidence: the result artifacts themselves, provided that their full
  configuration identity remains separated.

Remediation should version or correct the key to include every
configuration-defining field, add collision tests for `fresh_context` and
`inference_family`, and preserve the interpretation of already-produced
reports rather than silently reinterpreting them. This is compatible with the
reported S13 residual work; it does not require changing the migration order.

### PPCE-02 — Medium — Review Bench accepts path facts without establishment provenance

**Disposition:** remediation required before configuration-specific findings
are treated as established research evidence.

The result schema accepts provider, model, harness, inference family, fresh
context, tool access, reasoning effort, protocol, pass count, aggregation, and
optional session identity as supplied values. It validates their shape but does
not bind an actor recording, an independently owned execution observation, or
an establishment status. The comparison service then uses those values as the
identity and interpretation of its aggregates.

The current report is explicitly `descriptive_only`, so this gap does not turn
S13 publication into a production-review acceptance decision. It does mean
that a claim comparing configurations is unestablished when its required path
facts have no admissible source beyond the result artifact's assertion.

- Claim: an attempt ran under the named reviewer configuration.
- Subject: one Review Bench attempt.
- Covered state: its result and measurements.
- Consumption boundary: validation and admission into comparison.
- Consumer: the experiment owner and later adjudicator.
- Authority source: the frozen experiment contract for the comparison being
  run.
- Constraint: any harness restrictions actually imposed during execution.
- Evidence: currently unspecified by the result contract.

The cross-provider normalization and reviewer-profile admission work should
define the evidence profile. Actor recording can establish what the harness
requested. Provider- or host-owned observations are needed where the consumer
requires attestation of what was served or actually occurred. Unavailable
fields must remain unavailable rather than being reconstructed from model
output.

Turn, token, cache, timing, cost, and permission telemetry are measurements,
not automatically required production-path claims. Existing turn-based
telemetry therefore lowers the cost of the research path, but each later claim
must still identify which source established the measurement and what its
consumer requires.

### PPCE-03 — High — successful native-review evidence is dropped before acceptance

**Disposition:** remediation required.

The native Claude adapter produces a host-owned execution receipt containing
the profile configuration digest, registry revision, requested and observed
model, Claude version, session identity, fresh-versus-resumed continuity,
mutation authorization, and transport receipt digest. The runtime also
constrains the reviewer through a read-only capability profile and binds an
immutable subject.

On success, `native-review-closure.recordResult` passes only the semantic
review result and unresolved questions into Review Episode. The resulting
native binding retains episode and runtime-session references, findings, and
authority booleans, but not the execution receipt or an integrity-bound
reference to it. Slice Campaign terminalization accepts selected native review
when each obligation is `reported`; it does not require the production-path
claim to be established. By contrast, the result-contract failure path retains
transport and session recovery evidence.

- Claim: the selected result was produced for the bound candidate by the named
  reviewer realization under the independence, capability, and continuity
  properties required by the selection.
- Subject: the selected review episode and immutable candidate.
- Covered state: the admitted result, findings, and acceptance consequence.
- Consumption boundary: Review Episode `record_result`, followed by Slice
  Campaign terminalization.
- Consumer: the Slice Campaign acceptance owner and the builder relying on
  findings.
- Authority source: the configured review selection and validation profile.
- Constraint: immutable subject, fresh or retained-session binding, read-only
  capabilities, and no mutation authority.
- Independent observation: the App Server reviewer host's execution receipt.

The receipt already exists at the right owner. Remediation is primarily a
state-and-schema change: persist an integrity-bound execution observation with
the admitted result, project the required claim identity and establishment
status into the native binding or terminal receipt, and make terminalization
reject or route an unestablished required claim to its acceptance owner. Do not
infer establishment from the reviewer role label or replay provider work to
reconstruct discarded evidence.

### PPCE-04 — High — review selection commands independence without carrying its evidence contract

**Disposition:** remediation required.

The slice-supervisor review-selection instruction requires every selected
specialist to run in a fresh role-scoped context that does not inherit builder
reasoning or another specialist's conclusions. The persisted review-selection
schema contains only owner, selection identity, subject, and specialist
dispositions. It has no required production-path claim, covered state,
consumption boundary, consumer, evidence profile, or establishment status. The
terminal receipt projection similarly records selection, execution,
applicability, result reference, and findings without the evidence that
establishes independence.

This is both a contract and a loading defect. A role receiving the execution
command can obey the constraint without being told what evidence must reach the
consumer. A terminalizer receiving the current schema cannot distinguish a
preserved-but-unestablished independence property from an established one.

- Claim: a specialist result is independent of the builder state and other
  specialist conclusions covered by the acceptance condition.
- Subject: the specialist episode and exact candidate.
- Covered state: the result admitted into builder and campaign acceptance.
- Consumption boundary: first projection of the result into the builder's
  context and later terminalization.
- Consumer: the configured validation-profile and campaign acceptance owner.
- Authority source: the supervisor-owned selection and configured validation
  profile, not the builder or specialist.
- Constraint: fresh role-scoped context and separated provider/specialist
  identities.
- Evidence: not represented by the selection or terminal receipt contract.

Remediation should let a selection identify the required claim and its evidence
profile, then bind an admissible observation or recording at result admission.
It must not promote independent observation into a universal requirement:
recording remains sufficient when the external consumer requires only
attributable execution, and a review that makes no route-sensitive acceptance
claim need not acquire one merely because a reviewer was selected.

### PPCE-05 — Sufficient — canonical Git publication establishes its path claim

**Disposition:** sufficient as implemented for the inspected claim.

Canonical publication constrains mutation through an expected parent, a
fenced lease, exact checkpoint and manifest validation, and atomic
`update-ref`. Workspace Coordination owns mutation admission and exposes an
observation independent of the publishing role. The publication receipt binds
the operation, commit, tree, parent, branch, fencing token, validation,
authorization, accepted checkpoint, starting tip, observed branch generation,
and admission record. Reconciliation re-observes the branch and admission when
the immediate result is uncertain.

- Claim: the authorized checkpoint became the observed canonical branch
  generation through the admitted mutation.
- Subject: the named publication operation and branch generation.
- Covered state: the canonical Git reference and publication receipt.
- Consumption boundary: publication acceptance and any later reliance on the
  canonical revision.
- Consumer: the publication and migration acceptance owner.
- Authority source: the explicit publication authorization and Workspace
  Coordination contract.
- Constraint: fencing, expected-parent compare-and-swap, manifest validation,
  and atomic ref update.
- Independent observation: the coordination-owned mutation admission plus
  observed branch state.

This is the pattern to preserve: constraint protects the world-history fact;
independent observation establishes that the constrained mutation actually
occurred.

## Audit-round conclusion

The new doctrine does not require a major change to ordinary coding turns. It
does expose two places where normal review currently loses or under-specifies
evidence at durable boundaries, despite the host already observing much of it.
That makes the principal migration work receipt and admission plumbing rather
than more cognitive procedure.

S13 can remain published and the existing migration roadmap can continue. The
four remediation findings should be owned as follows:

- PPCE-01 and PPCE-02: S13 residual reviewer research, normalization, and
  profile-admission work;
- PPCE-03 and PPCE-04: post-S13 control-plane residuals affecting native review
  admission and terminalization.

This round is not the transition completion record. The remaining bounded
inventory includes planning-role read-only claims, provider fallback and
recovery claims, role-instance replacement, context replacement, and other
acceptance consumers reached by S14-S17. Exact activation of the successor
design and any implementation response still require their respective operator
authority.
