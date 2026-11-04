# V2 placement review and required implementation clarifications

2026-09-12. Exact subject:
`executable-generation-location-reconciliation-plan-v2.md`, SHA-256
`9a4b65e35ee9f78cdc0b3952fe50218ba919d412945d4b02123928f1353b4801`.
The reviewed file and v1 remain unchanged. This companion records review
consequences; it does not authorize implementation or claim a clean independent
review verdict.

## Provenance

Fresh disposable native Claude, Anthropic subscription route, requested model
`sonnet`, session `69850861-992a-4f32-9544-ca6b186e72c8`. No paid fallback.
Direct-source read-only review against `ad99d46`; graph unavailable in the
isolated provider. Exact prompt/result/transport receipt are retained at
`/home/bline/.local/state/work-engine/ppce-activation-ad99d46/placement-review-2/`.
The provider returned a mixed conflict/unresolved assessment, confirmed the
artifact-identity and CAS placements, and labeled its requested plan deltas
non-scope-breaking. It did not execute tests or establish production acceptance.

## Coordinator assessment

The supported direction is to preserve operational repository/artifact identity
while relocating executable source. The artifact-root fingerprint can preserve
its actual target inputs, and generation-store CAS remains the activation owner.
No competing semantic owner was identified within the reviewed scope.

The remaining findings concern explicit implementation seams already inside
v2's scope, not evidence that the selected outcome is impossible. They are still
required to close before treating an implementation plan as exact:

1. **Identity propagation:** explicitly thread the admitted identity base from
   proxy/bootstrap configuration through `roleEnvironmentSource` into both
   `projectRuntimeManifest` and the captured role-environment configuration.
   Physical source/capture paths must not silently become identity defaults.
   Keep actual source provenance and exact skill bytes separately verifiable.
2. **Requirements propagation:** explicitly distinguish candidate-byte hydration
   from canonical identity-path checks in `satisfyRuntimeRequirements`.
   Bootstrap's capture already calls `projectRuntimeManifest` directly; the
   review's suggestion that it necessarily goes through `loadRuntimeManifest`
   is not supported by that call site. Inspect actual consumers before changing
   the loader. Do not add an unused parameter or a second requirements owner.
   Require a compiled-role fixture where identity and capture roots differ.
3. **Deferred recovery:** add an explicit lifecycle entry point that separates
   read-only construction/validation from admitted recovery activation. Today
   host construction has no such seam. Wire the worker's lifecycle callbacks
   to it; do not merely rearrange imports. Keep live-store creation/migration,
   scheduler work, and queue release out of validation. Moving the current live
   construction into a validate callback would not satisfy the plan.
4. **Publication ordering:** bootstrap currently invokes worker activation
   before `activateStartupCandidate`. Preserve inert readiness before CAS and
   admit effects only after the durable transition. Define restart behavior
   for lost acknowledgments and the maintenance-only state. An `activate` RPC
   name alone is not evidence that this ordering holds.

These are proposed requirements for the bounded implementation plan, not edits
to runtime doctrine. Preserve exact fingerprint equality, the existing registry
and custody owners, and the plan's rejection conditions. Any need for a new
identity interpretation, binding successor, or release authority requires a
revised plan and explicit owning decision.

## Next gate and authority

Present v2 plus these clarifications for exact implementation planning/acceptance.
The builder must confirm the file boundary and demonstrate the feasibility of
the named seams; report any failed premise rather than silently expanding scope.
Independent implementation review and synthetic validation remain required.
This review has not established a passing implementation or authorized startup.

No source implementation, runtime activation, provider domain turn, PPCE
continuation, or incident-488 replay occurred. Only the draft plan, review
artifacts, and this disposition were produced.
