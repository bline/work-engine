# J2 and controlled HP3 parallel recommendation

Prepared on 2026-10-08 after publishing E0/J1/E1 as
`3b8d1f4b75375c3328f191748619ca7e515d4a22` on
`rescue-detached-20260911`. The user authorized that commit and the next parallel
planning task. This recommendation does not authorize implementation, providers,
migration, activation or another commit.

## Recommended next set

Accept a documented **J0 interface and representation contract**, then run two
Sol implementation lanes:

| Lane | Bounded result | Product owner |
| --- | --- | --- |
| [J2](rust-j2-completion-implementation-plan.md) | Exact findings, boundary-specific consumption, initial outcome publication and builder reliance/evaluation | `slice-campaign` and its tests |
| [H0 → H1](rust-hp3-controlled-host-implementation-plan.md) | Small episode admission prerequisite, then a permanent controlled initial Rust host | H0: episode admission; H1: new `review-workflow` and `work-engine-review-host` crates |

J0 freezes the proposed signatures, progress/locator types and payload meanings.
It does not require placeholder successful implementations. H0 must pass its
owner tests before H1 uses its direct write admission; it can proceed alongside
J2. H1 can develop through the already published J1 `OwnerReadsJoined` boundary.
Its J2 adapter and final actual-host qualification follow J2's accepted handoff.
That final integration is serial even though core implementation is parallel.

The first host is deliberately narrower than the original HP3 roadmap: a named
SC0-only, one-obligation, one-attempt initial profile on new private roots. It
does not generalize campaign admission, provide retained remediation, or finish
large-history and provider qualification. This restriction comes from the
published campaign constructor, not a temporary compatibility requirement.

## Reconciled boundaries

J1 remains the owner join and does not become campaign outcome publication.
J2 adds `finalize_initial_completion` and `recover_initial_outcome`; exact
builder evaluation has its own prepare/complete/recover operations. Read-only
`locate_initial_outcome` and `locate_builder_evaluation` reconstruct original
locators after a lost registration reply. Restart depends on durable IDs and
owner records, never on successful reply delivery or a saved capability.

H0 adds a trusted in-process constructor for the existing exact native episode
descriptor. A single inherited FD descriptor cannot authorize both `begin` and
the later result whose content depends on execution and claims evidence. The
new path shares the current descriptor and CAS checks, validates the actual
native root and executable, and leaves inherited-FD provenance checks intact
on that transport. It is a trusted Rust composition API, not a deserializable
wire grant or a claim that arbitrary linked Rust code is untrusted.

The host alone owns the startup capability that builds those descriptors from
installed authority and current campaign facts. Request DTOs cannot install a
root, authority, descriptor or executable. Episode `begin` occurs after fresh
campaign dispatch consumption and before E1 launch. Uncertainty there cannot
be converted into permission to launch again.

H1 owns permanent `initialize`/`serve` CLI composition and a bounded JSON-lines
consumer, with a separate control runtime and one workflow worker. It consumes
the compiler, campaign, claims, episode and E1 APIs; it owns none of their
canonical stores or reducers. No Node bridge, generic broker, duplicate workflow
journal or continuous-availability machinery is proposed.

## Decisions proposed for implementation acceptance

- **Preserve publication semantics.** `Reported` means published, not favorable
  or accepted. Native verdict, phase, limitations and questions remain visible.
  Published reported/awaiting-builder outcomes map to `Completed`; an
  evidence-unestablished result maps to `FailedWithResult`. These describe
  result processing and grant no retry or campaign acceptance.
- **Bind reserved schema-3 payloads precisely.** A checked empty native finding
  set uses a campaign-owned `no_findings` witness because CE cannot project zero
  selections. Nonempty sets use real checked CE projection output. Evaluation
  identifies CE claims/revisions while preserving native UTF-16 IDs in their
  canonical mapping. Accept these exact previously unimplemented payload
  meanings before enabling them; do not silently add a new schema dimension.
  The initial complete finding set is bounded to 0–100, with pre-effect refusal
  beyond that limit. `CorrectionRequired` remains refused until a durable
  rejection owner exists.
- **Consume only at the named consequence.** Publish the builder-projection
  consumption marker with its usable view. Preserve the terminalization
  commitment without marking it consumed: this work does not terminalize.
  Checking both required claims does not erase that distinction.
- **Accept the narrow H0/H1 profile.** Select the direct trusted descriptor API,
  permanent stdio CLI, finite SC0 startup contract, one-worker concurrency and
  bounded control/transport limits in the host plan. Partial initialization
  preserves unresolved owner state rather than wiping roots or repairing it
  through an invented journal.
- **Keep qualification truthful.** E1's actual same-UID child reports mutation
  authority and cannot attest an enforced read-only or provider-independent
  realization. The actual-host exit is J2 `EvidenceUnestablished`, preserving
  both exact claims and reasons. Existing controlled fixtures separately prove
  established/false J2 domain branches. No fixture substitutes for actual host
  execution, and no positive provider attestation is claimed.

These choices are a single concrete recommendation for the next implementation
set. The detailed plans retain the exact API, payload, authority and failure
contracts. An implementation finding that exceeds them returns to its owner;
it is not permission for the two lanes to invent incompatible types.

## Integration ownership and acceptance

One integration owner coordinates J0, H0's accepted export and final host/J2
composition. J2 owns all campaign changes. H0 owns only the bounded episode
admission change. H1 owns new host/workflow crates and consumes those exports.
Claims and E1 product changes require an explicit owner handoff if a real missing
API is found. Lifecycle S6/S7 currently owns shared Cargo/lock edits; coordinate
additive requests there. Neither lane depends on unaccepted S6/S7 or UI changes.

Final qualification uses the actual pinned host process, its permanent CLI,
real initialized owners, compiler projection and an actual controlled Rust child.
It proves current admission, exact result/outcome readback, lost-reply recovery,
partial commits and no extra child entry across restart. Domain fixtures cannot
replace this combined gate. Host responsiveness is measured externally under
the explicitly bounded profile; a passed control measurement does not establish
the earlier large-history completion claim.

J2 also needs real CE finding/reliance/projection checks for both valid and
invalid builder decisions, native codec regressions, complete-set bounds,
corruption refusals and actual process cuts. Preserve the published J1/E1 and
P1 integrity checks. Keep all scratch local, bind exact final sources and
toolchain/features, and review the composed result before acceptance.

Real supervisor dispatch, provider credentials and realized grant attestation,
retained remediation/correction, general campaign profiles, terminalization,
state migration/disposition, deployment and Node retirement remain separately
owned follow-ups. Developing the replacement offline does not require a
temporary operational host.

## Planning record

Two parallel planners were requested as `gpt-6-astra/high`; implementation
remains the user's selected `gpt-6-sol`. Parent reconciliation resolved empty
projection evidence, boundary-specific consumption, publication versus
acceptance, episode admission/ordering and lost-registration-reply recovery.
This was source-backed planning and reciprocal interface reconciliation, not
an independent implementation review or runtime qualification.

Exact plan hashes, source/coverage receipts and authority are retained under
`/home/bline/.local/state/work-engine/rust-j2-hp3-planning-20261008/`. No product
edits, builds, tests or providers ran during this planning task. The preceding
publication was separately authorized and qualified; its receipt is under
`/home/bline/.local/state/work-engine/rust-j1-e1-publication-20261008/`.
