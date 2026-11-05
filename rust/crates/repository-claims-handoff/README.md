# repository-claims-handoff

Claims-owned, out-of-band handoff of **exact** repository-claim references across
a context replacement. It is a small library and CLI, not a workflow owner: it
adds no grant, schema, storage, applicability or semantic-validity authority.

## What a handoff is

One line of tool output, at most 4096 bytes (`MAX_HANDOFF_BYTES`):

```text
CLAIMS_HANDOFF_V1 {"applicability":"not_assessed","refs":[{"claim_id":...,"evidence_sha256":[...],"revision_id":...,"revision_sha256":...}],"schema":"repository-claims-handoff-v1","selected_by":...,"selection":"exact","selection_basis":...}
```

The body is canonical JSON from the `claim-evidence` codec with a closed field
set. One to four references; each names an exact claim ID, revision ID and
revision SHA-256 and optionally up to four evidence artifact digests. There is no
"latest" selector. `selected_by` and `selection_basis` are retained data, not
authenticated or assessed. A historical selection stays historical when a newer
head exists.

## Carriage

The coding continuity owner already embeds the exact Host `coding_result` (tool
output included) in the checkpoint it stages and replays as the sole recovered
prompt (cap 65 536 bytes; Host tool output cap 60 000). The handoff rides in that
tool output; no lifecycle field is added. `extract_from_checkpoint` accepts a
handoff only when exactly one distinct marker line occurs inside `coding_result`;
absence, ambiguity, non-canonical form or oversize refuses.

## Verification

`resolve_handoff` starts `work-engine-server claims-stdio <read-profile>` from a
`TrustedServer` that the caller supplies: executable and read-profile paths plus
their expected SHA-256 values. Nothing trusted is read from a handoff or
checkpoint. For every reference it calls `resolve_repository_fact` with
`projection_page` (framed pages, default 32 768 bytes) and checks offsets, totals,
continuation and the complete projection digest. That digest is supplied by the
same response, so it proves page assembly only. The verifier then binds the
content to the selection with the native codec: the projection must be the
canonical encoding of what was parsed; the revision's recomputed digest must equal
the selected digest; the request digest in the envelope must equal the digest of
the selected `{claim_id, revision_id, revision_sha256}`; and the native envelope,
admission, authority, reliance and head fields must all be present with the native
types (closed field sets, no defaults). It then reads each selected evidence
artifact with `read_repository_evidence` and checks its digest and a null terminal
`next_offset`. Any failure refuses the whole handoff. `current_head` is reported,
not required. Applicability, source currency and semantic acceptance remain
`not_assessed`. Authority is not authenticated here; that stays with the claims
owner and the trusted server pin.

## Result

`Resolution.report()` (`repository-claims-handoff-resolution-v2`) keeps, per
reference, the complete verified native `repository_fact_projection_v1` as
`owner_projection` (at most 2 MiB) plus its `projection_sha256`, the recomputed
checks under `verification`, and `selection.{selected_by,basis}` marked
`unauthenticated_descriptive_data`. No summary replaces owner data.

## Bounds

CLI inputs must be regular files; FIFOs, devices and directories are refused
without being read, and reading stops at the bound plus one byte. The MCP reader
rejects a frame over 1 MiB itself and uses a one-slot queue, so a flooding server
is blocked rather than buffered; the session kills and reaps its direct child and
releases the reader on every exit.

## CLI

```text
repository-claims-handoff emit --selected-by T --basis T --ref CLAIM,REVISION,SHA[,EVIDENCE...]
repository-claims-handoff extract (CHECKPOINT_FILE | --continuity-result FILE)
repository-claims-handoff verify (--checkpoint F | --continuity-result F | --handoff F)
  --server PATH --server-sha256 H --profile PATH --profile-sha256 H [--page-bytes N] [--deadline-seconds N]
```

Exit 0 prints the handoff or a resolution report; exit 2 prints
`{"status":"refused","code":...}`. Refusal codes: `invalid_handoff`,
`invalid_checkpoint`, `handoff_missing`, `handoff_ambiguous`, `server_untrusted`,
`profile_untrusted`, `server_unavailable`, `owner_refused`, `projection_corrupt`,
`revision_mismatch`, `evidence_not_listed`, `evidence_corrupt`,
`evidence_unavailable`, `protocol`, `invalid_options`.

## Tests

`cargo test -p repository-claims-handoff` uses a temporary synthetic registry and a
real `work-engine-server` process, plus `tests/transport.rs` fake servers (silent, oversized, flooding) and CLI input bounds. The binary is `WORK_ENGINE_SERVER_BIN` or the
sibling `work-engine-server` in the target directory (build it first, or point the
variable at a pinned server). Fault-injection tests proxy the real server and alter
one response.

## Limits

The executable digest is checked before spawn, not bound to the spawned inode. The
checkpoint's `coding_result_sha256` (an artifact digest) is not recomputed here;
equality of the recovered prompt with the checkpoint is the lifecycle owner's
receipt. Reads give no assurance that the proposition is true, sufficient or
applicable.
