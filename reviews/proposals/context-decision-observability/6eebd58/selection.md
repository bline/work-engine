# Review Selection: Context-Decision Observability

## Subject

- Proposal: `work-engine.context-decision-observability`
- Checkpoint commit: `6eebd58acd991e62e6a85b5ac89de42f0712a74e`
- Checkpoint tree: `7dddbc8b449df056b7923167f40ea121f5247828`
- Task patch digest:
  `414c00adf90b490eb0c0fb2111db271fa176e8ae093597a856194fb47c842f92`
- Plan version: `context-decision-observability-proposal-review-v1`
- Scope revision: `formed-context-decision-observability-v1`
- Gate receipt digest:
  `d31ce3d9499e0ea6142890def0d2ecba8ea1f7f0fcdae3437962b8223f918815`

The subject is a private immutable checkpoint. Review is advisory and cannot
accept the proposal, decide roadmap priority, or authorize implementation.

## Selected perspectives

### Architecture and ownership

Selected because the proposal composes host/runtime facts, role-owned state,
and a model-facing projection while leaving three plausible physical and
semantic placement boundaries open. Review must test whether the proposed
owners, causal dependency, family placement, and first vertical preserve the
producer/owner/consumer distinction.

Provider session: `35aabf3d-f4c7-4c5c-b65b-4a13b4d70011`.

### Lifecycle and evidence

Selected because context-window identity, observation cutoff, checkpoint
revision, cross-window freshness, observed/derived/estimated/unavailable
classes, and before/after rehydration measurements determine whether the packet
can support truthful decisions.

Provider session: `00a95b10-d079-4c7e-bd7c-e96605b8f7c7`.

### Authority and doctrine

Selected because the proposal must separate replacement safety from benefit,
preserve user and role authority, and avoid turning an observation surface into
a mandatory reset procedure or automatic completeness proof.

Provider session: `41c5beb8-9829-4bbc-84e3-d3c7b409d803`.

### Security and visibility

Selected because model-visible context metrics, checkpoint references,
rehydration manifests, provider identity, and cross-role reuse may expose
sensitive or independence-scoped information. Review must test visibility,
trust, injection, and least-disclosure boundaries.

Provider session: `ac535096-8fe9-4c06-81ac-5edf891cbd0b`.

### Agent instruction structure

Selected because the current proposal contains materially normative
agent-facing decision semantics: safety must precede benefit, certain source
substitutions are forbidden, and the packet must not create a threshold or
authority. The reviewer must determine applicability and assess structural
necessity, ownership, audience, scope, and whether any exact route is actually
causal.

Provider session: `a627499b-7913-4ab5-aea3-2ee3228b6859`.

## Material omissions

- UI/process review: omitted because the subject proposes no present UI or
  interaction surface. A UI is not required to consume the first vertical.
- Accessibility review: omitted with the absent user-interface consequence.
- Implementation validation: omitted because the subject forms a proposal and
  claims no implementation.
- Migration/compatibility review: omitted because no accepted schema or
  deployed consumer is being replaced. Schema evolution remains an evidence
  question for a later implementation proposal.
- Empirical-methodology review: deferred because the longer-lived
  reset-outcome research candidate remains unformed. The selected
  lifecycle/evidence perspective covers only the live packet's estimates and
  observation provenance.

## Runtime and provenance

The three initial Claude provider attempts failed before reading the subject or
producing evidence because the provider reported a weekly quota limit. The
exact failed-attempt provenance and fallback decision are recorded in
[`provider-provenance.md`](provider-provenance.md).

The configured bootstrap policy permitted fresh Codex fallback because this
proposal requires specialist advisory review, not an exact provider or an
independent-evidence class. Each fallback specialist began in a separate fresh
process with the immutable snapshot, read-only repository access, and no
proposal-former, coordinator-synthesis, builder history, or other reviewer
conclusions. The resulting evidence class is
`accepted_same_model_review`, with `independence_claimed: false`.

Each Codex review is retained in an authority-bound review episode. The
manifests under [`authority/`](authority/) bind the exact reviewer target,
writer generation, subject, and permitted readers. The durable episode owns the
full attributed findings and remediation status; the adjacent specialist files
are repository projections for proposal review and Git history.
