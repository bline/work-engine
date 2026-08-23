# Proposal: Raw Agent Execution Evidence Archive

## Identity and state

- Proposal ID: `work-engine.raw-agent-execution-evidence-archive`
- Family ID: `work-engine.empirical-agent-research`
- State: placement uncertain; not reviewed, evaluated, accepted, prioritized, or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio and data-governance owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current candidate meaning. [`implementation-plan.md`](implementation-plan.md)
describes one possible delivery shape without implementation authority.

## Candidate and consequence

Provide a provider-neutral, sensitive raw-evidence archive that can losslessly
preserve and reconstruct exact agent execution evidence while binding it to
Work Engine proposal, campaign, slice, review, checkpoint, and outcome lineage.
A future authorized researcher can distinguish configured, directly exposed,
discovered, invoked, failed, and unavailable capabilities; reconstruct
model-visible inputs where the provider exposes them; inspect observable
behavior and resource use; and recompute later metrics without relying on an
agent's recollection or a lossy terminal receipt.

The archive is forensic evidence, not operational memory. Work Engine remains
correct when the archive is unavailable. Proposal packets, claims, role state,
checkpoints, review artifacts, metrics, Git, and authority records retain their
existing semantic ownership.

## Proposed product change

Define the smallest common archive contract and provider adapters that preserve:

- original raw bytes or a provider-declared exact/logically equivalent trace
  bundle;
- provider, format, schema, harness, model, session, rollout, thread, turn,
  inference, and parent/child identities when observable;
- append-safe prefix captures and explicit finalization;
- lossless compression with independently verifiable uncompressed identity;
- bundle and chunk integrity, ordering, source offsets, and reconstruction;
- capture completeness, failures, omissions, redaction, sensitivity, retention,
  and deletion state;
- exact Work Engine lineage references without copying domain-owned meaning;
- a replaceable discovery index and derived projections; and
- confidence-qualified historical correlation when exact binding is absent.

The proposal does not require every provider to expose identical evidence.
Absence remains explicit rather than being filled by agent self-report.

## Protected properties

### Original evidence remains exactly recoverable

Reason: future classifiers, provider schemas, and research questions will change.
A summary or normalized event stream can irreversibly discard evidence needed
for reanalysis.

Required property: a conforming final archive reconstructs the exact retained
source bytes and verifies them against an uncompressed content digest. Reduced
state, indexes, summaries, and classifications remain derived and replaceable.

### Live capture remains truthful

Reason: provider session files may still be appended when a checkpoint,
interruption, crash, or replacement makes preservation valuable.

Required property: runtime captures identify observed byte length, source
offsets, prefix digest, capture time, and `prefix` versus `final` state. A
partial file or incomplete JSON line cannot be represented as a completed
session.

### Compression does not change evidence identity

Reason: compression codecs, levels, dictionaries, and storage services can
change independently of the raw evidence.

Required property: the archive records uncompressed identity separately from
stored-object identity and codec metadata. Recompression does not create a new
semantic trace. Lossy compaction or summarization cannot be the only retained
copy.

### Sensitivity is part of the contract

Reason: traces can contain user messages, source, tool schemas, terminal output,
paths, browser data, credentials, or other sensitive material.

Required property: collection, access, encryption, retention, export, deletion,
and benchmark admission remain explicit authorized transitions. Raw evidence is
not stored in ordinary repository history or exposed merely because a receipt
references it.

### Forensic evidence does not become operational truth

Reason: replaying transcripts or tools can duplicate effects, resurrect stale
authority, or cause product correctness to depend on provider retention.

Required property: archive reads are observational. They do not resume a role,
replay a tool, redeliver a message, recreate a commit, infer acceptance, or
replace role-owned durable consequences.

### Capability layers remain distinct

Reason: a capability can be configured, registered, directly exposed, hidden
behind discovery, made visible later, invoked, forbidden, failed, or absent.

Required property: records retain the strongest directly observed state and its
provenance. Non-use does not prove inaccessibility, configuration does not prove
exposure, and agent self-report does not replace provider/runtime evidence.

### Cross-stage lineage references stronger owners

Reason: one research record cannot truthfully own intake meaning, proposal
identity, implementation authority, review findings, accepted code, and later
defects.

Required property: archive bindings reference exact domain-owned revisions and
state what relationship was observed. They do not copy those facts into a
universal transcript owner.

## Provider-specific evidence

### Codex

For supported Codex builds, first-party rollout tracing is the preferred source
of model-facing evidence. The provider adapter launches the process with an
isolated durable trace root and binds the resulting manifest, trace log, raw
payload tree, and provider identities to the Work Engine run. Raw payloads are
authoritative. Provider reduction output is retained as derived evidence.

Ordinary `~/.codex/sessions` rollouts remain a fallback and historical source.
They must not be described as exact inference-request evidence when that payload
is absent.

### Claude

Until a supported equivalent inference trace exists, the Claude adapter may
capture append-safe prefixes of provider session JSONL and related sidechain or
attachment evidence. Tool invocation and result evidence can be strong while
exact system instructions and per-inference capability exposure remain
unavailable or uncertain. Provider format is versioned observed evidence, not a
stable Work Engine-owned schema.

## Boundary and placement

The probable semantic owner is a research-evidence archive separate from:

- role-owned operational state, which owns correct continuation;
- metrics, which own compact operational measurements;
- review artifacts, which own findings and applicability;
- checkpoints and Git, which own immutable code subjects;
- claims, which own attributed epistemic statements if that proposal is later
  selected; and
- Studio, which may provide a future forensic projection without owning bytes.

Physical placement, storage service, encryption boundary, retention owner, and
deletion authority remain uncertain and require exercised security and consumer
evidence.

## Historical evidence

Existing provider sessions may be admitted through confidence-qualified
correlation:

```text
exact
strong
probable
mention_only
unmatched
```

Copying a historical file prevents further loss; it does not strengthen its
lineage. Later quotations of a run identifier are not original-run evidence.

## Out of scope

- hidden chain-of-thought as a required research artifact;
- transcript replay as recovery or execution;
- making all traces benchmark cases;
- permanent selection of a storage vendor or codec;
- treating reduced state or agent summaries as authoritative raw evidence;
- inferring truth, authority, accessibility, or causality from persistence;
- environmental scoring, behavioral classifiers, or reviewer-selection policy;
- changing provider software or promising unsupported provider schemas; and
- implementation authorization.

## Acceptance consequence

If later accepted and implemented, an authorized consumer can recover and
verify the exact retained provider evidence for a bound Work Engine run,
distinguish raw, derived, partial, redacted, and unavailable evidence, and
recompute future analyses without making product correctness or authority depend
on a transcript.
