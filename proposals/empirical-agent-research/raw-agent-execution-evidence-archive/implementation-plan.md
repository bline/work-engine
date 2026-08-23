# Implementation Plan: Raw Agent Execution Evidence Archive

This is a possible delivery shape for the formed candidate, not implementation
authority or a mandatory route. Phases may be reordered, combined, or revised
when another route preserves exact reconstruction, integrity, sensitivity,
provider truthfulness, and ownership boundaries.

## 1. Freeze the evidence and authority contract

Define a versioned bundle manifest that separates:

- uncompressed source identity;
- stored-object identity;
- provider and source format;
- prefix versus final capture;
- Work Engine lineage references;
- raw versus derived evidence;
- sensitivity, access, retention, redaction, and deletion state; and
- known absence, limitation, and reconstruction confidence.

Exercise the contract with one completed trace, one live prefix, one interrupted
session, one redacted or access-restricted object, and one missing-provider
field. Do not use null or zero to blur unknown, unsupported, unobserved,
inapplicable, capture-failed, and intentionally omitted evidence.

## 2. Codex first-party trace vertical

Launch one bounded Codex worker outside user-authored repository state with a
run-specific `CODEX_ROLLOUT_TRACE_ROOT`. Bind the trace manifest and rollout,
thread, turn, inference, model, provider, and Work Engine identities before the
trace can be mistaken for an unrelated session.

Exercise:

- a full first inference request;
- direct model-visible tool schemas;
- deferred or nested capability discovery;
- one harmless nested tool invocation and result;
- one compaction or provider context transition if supported;
- one child-agent relationship; and
- successful `trace-reduce` output marked as derived.

Prove that the raw payload chain reconstructs the provider-declared logical
model input when transport reuse emits incremental requests.

## 3. Claude append-safe session vertical

Discover the exact primary and sidechain session identities for one bounded
review episode. Capture immutable source prefixes at review start, material
phase completion, checkpoint, and termination without assuming the provider
file is closed.

Record source path, provider/client version, session and parent identities,
observed byte range, modification metadata, prefix digest, capture state, and
limitations. Finalize only after the source stops changing. Preserve incomplete
final-line state without parsing it as a complete event.

Demonstrate that invoked tools, arguments, results, messages, model, and session
metadata remain available while exact system instructions and per-inference
tool exposure remain explicitly unavailable when not observed.

## 4. Lossless compressed object storage

Exercise independently compressed immutable chunks rather than recompressing a
whole growing session at every boundary. A plausible first adapter uses Zstandard
at a moderate level with an implementation-selected chunk size. The contract
requires only lossless reconstruction, bounded retrieval, integrity, and codec
replacement; it does not make one codec or chunk size permanent doctrine.

For each chunk retain:

```text
source byte offset
uncompressed length and digest
codec, version, level, and dictionary identity if any
stored length and digest
encryption and access metadata
```

Compression precedes encryption. Already compressed attachments may be stored
without redundant compression when their original bytes and media identity are
preserved. Measure compression ratio, capture latency, finalization cost,
partial retrieval, and exact byte reconstruction on representative Codex and
Claude evidence.

## 5. Bundle finalization and crash recovery

Publish prefix manifests and a final manifest through a crash-safe,
integrity-checked transition. Exercise interruption before chunk publication,
after chunk publication but before manifest update, and during finalization.
Unreferenced objects must not become evidence; garbage collection must not remove
objects reachable from retained manifests.

Finalization verifies the reconstructed full-source digest. Recompression or
encryption-key rotation preserves the uncompressed evidence identity and emits
new storage provenance rather than a new semantic session.

## 6. Work Engine lineage binding

Bind archive evidence by reference to the strongest available identities:

- raw intake source and proposal revision;
- campaign, run, slice, attempt, and accepted plan;
- builder or reviewer role and provider episode;
- candidate, repair, and accepted checkpoints;
- review artifact and finding identities;
- completion commit; and
- later correction, defect, or reopened claim.

Do not require every stage for every trace. Missing lineage remains explicit.
The archive does not copy or transition the referenced owner's semantic state.

## 7. Replaceable discovery index

Build a derived index sufficient to find evidence without scanning compressed
objects. Candidate fields include provider, format, model, harness, timestamps,
repository, revision, proposal/run/slice/checkpoint IDs, event-type counts,
capture completeness, sensitivity, archive object IDs, and reconstruction
confidence.

Rebuild the index from manifests and raw evidence. Deleting or corrupting the
index must not destroy the archive or change evidence identity.

## 8. Historical preservation and qualification

After data-governance approval, inventory existing Work Engine Codex and Claude
sessions, snapshot raw bytes, and correlate them against metrics, Git subjects,
checkpoints, prompts, findings, and timestamps. Preserve every match candidate
and its basis. Admit only confidence-qualified evidence; distinguish an original
session from a later session that merely quotes its identifiers.

Historical backfill is not required to prove future capture. It is a separate
bounded preservation and research-quality exercise.

## 9. Security, privacy, retention, and export

Before routine collection, establish authorized defaults for:

- which roles and repositories may be captured;
- encryption and key ownership;
- raw and derived access;
- sensitive-field handling and redaction provenance;
- retention and legal deletion;
- incident response;
- research and benchmark admission; and
- export without accidental disclosure.

Prove that ordinary Work Engine receipts reveal only safe bindings and cannot
read raw payloads without archive authority.

## 10. Downstream proof

Use one authorized archived Codex run and one Claude review episode to answer a
bounded subset of the future-question inventory from raw evidence. Recompute a
derived environment/behavior projection with a revised analyzer and show that
the raw source remains unchanged. Confirm that removing the archive does not
prevent role recovery, proposal consumption, checkpoint validation, or ordinary
slice completion.
