# Location-reconciliation placement disposition — 2026-09-12

Plan v1 remains unchanged at SHA-256
`d17f9ab7eb5fca8814bb27d43033215ad4396461d78f7eebe3c978b73569b02e`.
It is **not ready for implementation acceptance**. Its explicit condition that
relocation preserve per-role fingerprints is not met by current identity rules.

## Review provenance

Fresh disposable native Claude review of code `ad99d46cd73996b580f1263496ccba75acc2c431`
and the exact plan; subscription route, requested model `sonnet`, no paid
fallback. Provider session `76c64e1c-2dae-4c71-9476-0d20c4588fc7` returned
`conflict`. Read-only direct-source tools; graph not provisioned in the isolated
review runtime. This is advisory placement evidence, not production acceptance.

Prompt, complete result, and transport receipt are retained under
`/home/bline/.local/state/work-engine/ppce-activation-ad99d46/placement-review-1/`.
Transport reports one successful Anthropic attempt, 180011 ms, observed model
labels `claude-sonnet-5` and `claude-haiku-4-5-20251001`. These labels do not
establish upstream routing or an exclusively single-model internal execution.

## Coordinator disposition

1. **Accept the role-identity conflict.** `runtime-manifest.mjs` resolves
   contract and identity-skill paths against the absolute identity base and
   hashes them into `runtimeEnvironmentRevision`. The adapter includes that
   revision in its binding fingerprint and rejects changed fingerprints before
   resuming a bound thread. Generation-only maintenance cannot establish the
   plan's unchanged-role-fingerprint condition. Manifest identity belongs in
   the architectural decision, not silently in an implementation extension.
2. **Accept the candidate-construction hazard.** The default worker constructs
   its live environment before bootstrap's compatibility and durable activation
   checks. The live host begins lifecycle and authorized aborted-release
   recovery during construction. Candidate staging must have an explicit
   no-delivery boundary; activation success is not the earliest possible effect.
   This is a general source-level hazard, not evidence that incident 488 replayed.
3. **Qualify the review's generation-fingerprint explanation.** The generated
   configuration diff alone cannot explain the running gate. Generation hashing
   also includes `productDevelopmentEnvironmentIdentity`, supplied by the
   artifact-root owner, and role bindings have their own fingerprint. The plan
   already required distinguishing those identities. Do not treat the reviewer
   statement that the coarse fingerprint is path-independent as established.
4. **Do not classify all absolute paths as accidental.** Repository access,
   artifact destinations, provider working directories, and retained-context
   references can be semantic. Location-invariant identity is a design choice
   requiring bounded equivalence evidence and historical compatibility, not a
   blanket hash normalization fix.

## Required next design decision

Revise the plan to distinguish canonical identity from physical realization,
including how existing path-based bindings remain interpretable. Compare an
explicit mapping that preserves an admitted canonical identity with a genuine
role-environment successor requiring separately owned binding/context migration.
Neither alternative is accepted here. Bind the actual artifact and repository
destinations before choosing; changing them is not merely relocating source.

Separate inert candidate preparation from activation and from authorized
recovery. Reuse generation lifecycle and context-custody owners rather than
creating a second release authority. Any revised plan needs fresh placement
validation before implementation. No general provider-neutral architecture,
PPCE implementation, or incident-recovery scope is acquired by this review.

## Preserved state

After review, the exact plan digest and incident-488 complete queue-row
projection digest still match their pre-review values. The latter remains
`3f7f8da96f9db64ee11854c48ae124fcdd551a8599f7d2420d9bc8bb578a0872`.
No implementation, activation, queue replay, domain turn, or campaign resumption
was performed by this review operation. Canonical login was refreshed by the
operator before review; the reviewer received only its access-token projection.
