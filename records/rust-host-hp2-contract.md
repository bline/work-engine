# HP2 claims and campaign contract for the Rust host

Prepared 2026-10-07. Planning proposal under the user's authorization for parallel
claims/campaign planning. This document does not authorize implementation,
provider execution, state migration, cutover or commit. Future implementation
and ordinary remediation use `gpt-6-sol`.

The [claims plan](rust-claims-hp2-plan.md) and
[campaign plan](rust-campaign-hp2-plan.md) produce permanent domain libraries for
HP3's native-review application. The system is inactive during development;
there is no availability requirement or need for a temporary Node bridge.
Historical identities and saved-state disposition still have owners.

## Baseline and ownership

[HP1](rust-host-hp1-validation.md) supplies typed direct episode operations,
private store/admission ownership and exact result/reconciliation behavior. Its
joined source aggregate is
`7b1b684d172528b55b57f6e9b739f9945e5bd223cd3459be3082294a3efed27f`,
based on published S2 `b2efdbfa` including S3 `634161c2`, plus the frozen C3/R3
and HP1 candidate-2 changes. HP1 remains isolated; no ordinary commit is implied.
Its 362 ms full-history diagnostic establishes successful complete output,
not a supported service budget or a passing replacement for the failed HOST run.

| Owner | Owns | Does not acquire by composition |
| --- | --- | --- |
| Claim evidence | Finding revisions, reliance, exact projections, observations, establishments, claim authority and persistence | Campaign selection, episode completion, provider execution or campaign acceptance |
| Slice campaign | Authoritative campaign/candidate/selection, operation admission, revision CAS and review-state publication | Truth of claims, episode transitions or provider settlement |
| Review episode | Episode transition validity, writer/revision fencing, result history and replay | Campaign admission, claim establishment or acceptance |
| HP3 review workflow | Composition, exact subject/profile, provider adapter and crash recovery across these owners | A generic right to mutate stores or infer successful downstream publication |
| Workspace/deployment owners | Shared Cargo registration/lock, exact executable startup and later whole-root transfer | Automatic ownership of domain state |

These are Rust libraries in one initial host process. No domain daemon, generic
broker, shared store framework or universal workflow state is proposed. Their
state stores remain separate transaction boundaries.

## References, codecs and authority

Use domain-exported reference types. No new common package is needed for HP2.
A reference is descriptive data; a public constructor or deserialized DTO cannot
create a write permit. The owning application keeps store handles and admission
construction private. The provider child receives neither.

Claims export distinct finding-revision, reliance, production-claim, observation
and establishment references. Each retains its exact domain identity, revision,
digest and fixed owner. Boundary adapters preserve the historical
`owner/reference/revision/sha256/freshness` shape where that contract applies;
missing provenance is not filled with invented values. A finding publication
retains finding ID, initial and current exact revision references and optional
reliance. A projection retains selected revisions, provenance, completeness and
limitations. None establishes authority merely by existing.

Claims admission/readback data binds the exact claim, establishment, optional
observation, consumption boundary, consumer and status. The two boundaries are
`BuilderProjection` and `CampaignTerminalization`; `Established`, `False` and
`Unestablished` remain different outcomes. The campaign owns the consumption
reference. A builder-projection establishment cannot be relabeled as evidence
for terminalization.

Campaign exports the following data, with validated identity/digest wrappers:

| Type | Bound fields |
| --- | --- |
| `CampaignIdentity` | `runId`, positive safe-integer `sliceNumber`, `attemptId`, `planVersion` |
| `CampaignRef` | Root ID, campaign identity, campaign revision |
| `CandidateRef` | Commit, tree, patch identity, receipt digest, physical-profile digest |
| `SelectionRef` | Selection ID, campaign-codec digest, distinct episode-authority digest |
| `NativeReviewRequestRef` | Root ID, campaign identity, obligation/operation IDs, operation kind, prepared revision, request digest, selection, candidate and profile digest |
| `ConsumptionRef` | Historical owner/reference/revision/digest/freshness envelope over exact claim revision, establishment, boundary and consumer |

Initial, retry and evaluation are supported request kinds; remediation remains
reserved for the later accepted join. Campaign snapshot/selection identities use
the campaign codec; episode admission uses the distinct episode selection digest;
consumption identities use the claim codec. The consumption reference retains
owner `slice-campaign` and `claim-consumption:` identity. Cross-domain values keep
typed provenance rather than collapsing into interchangeable string digests.

Required production-path claim documents currently live in the selected campaign
obligation; claims persistence contains their observations and establishments,
not a duplicate selection. Claims establishment readback receives the exact full
claim revision from campaign-owned state plus the expected references, then
checks its own stored evidence and computed claim binding. A claim reference
alone cannot retrieve a document that the claims store never persisted. Finding
lineage remains claims-owned; required-claim succession stays with its separately
accepted campaign/episode correction join.

Claim identities use their historical code-point ordering, JavaScript scalar
encoding and trailing newline. Episode identities use their own existing
codec. Campaign identities use their declared campaign codec. The owning kind
and version select the codec; an input cannot choose a weaker validator.
Cross-language vectors cover ordering, numeric/scalar behavior, Unicode and
historical reference bytes. No generic serializer rewrites existing identities.

The campaign owner checks the current candidate, selection and revision at the
relevant effect boundary. Claims separately validate the exact registered grant,
action, authority source and scope through their bootstrap owner. A campaign admission handle binds the exact prepared request, owner epoch and
current revision. A durable admitted-operation slot and exclusive root writer
protect candidate/selection/profile bindings. A bounded process-local admission
lease covers the claims commit/read after revalidation; no SQLite transaction
spans stores. The durable reservation survives provider lifetime and a crash;
loss of an OS/local lock does not resolve it. Reopen acquires exclusive ownership,
changes the owner epoch and reconstructs the exact reservation. Old handles
cannot authorize effects. An exact-operation recovery lease grants no provider
entry; no timeout or clock expiry establishes settlement. The handle cannot be
deserialized or cloned into a new grant. The later HP3
composition binds both domains to the exact admitted request. Domain-only tests may
qualify bootstrap and local store behavior; they cannot claim that the joined
campaign grant or external provider boundary has been qualified.

## Effects, receipts and recovery

An applied or matching replayed write returns an exact domain-owned receipt.
Campaign receipts bind operation ID, request digest, prior/result revisions and
state/result reference. Its uncertain outcome also binds campaign identity,
operation kind, expected revision and profile digest. Claims uncertain outcomes
retain the action, claim-codec payload digest and grant digest. These are
separate owner-specific result types, not one cross-domain error enum.
A no-effect error requires evidence of refusal before commit or confirmed
rollback. An uncertain commit/delivery retains a reconciliation locator binding
the checked absolute root, operation identity, action, exact content digest and
required admission/grant identity. Readback distinguishes absent, committed,
conflicting and unresolved. An error or missing response does not prove absence.

Use the existing per-domain receipt/replay keys where they already carry the
needed identity. Claims finding operation receipts, observation event identities
and establishment operation identities have different contracts. HP2 does not
add a generic cross-domain transaction journal to unify them.

HP3 composes durable preparation, episode result, claims publication and campaign
result publication. Each edge needs its own exact readback. Recovery after an
episode result resumes missing downstream work against that result; it does not
re-enter the provider. An absent local receipt alone cannot establish definite
pre-entry failure of an external action. Conflicting or unresolved evidence
remains explicit until its owning contract can resolve it.

## Initial supported profile

Both lanes target new private roots. A bounded campaign can ingest already
created immutable candidate and physical-profile artifacts through a real,
read-only verifier. The verifier checks actual repository objects and exact
artifact/provenance bindings under a declared trusted historical producer
profile. The request does not choose trusted producer identities. Candidate
creation/checkpoint writing remains with its owner; fixture table writes or
caller assertions cannot stand in for admission.

The initial joined profile targets initial review, exact recovery without
provider re-entry, retry only when definite pre-entry evidence permits it, and
finding evaluation. Claims revision support does not itself enable retained
remediation. The inherited native Rust episode route currently refuses retained
remediation and succession; those remain closed until the separately accepted
RC/episode join supplies their semantics. Broader claim discovery/maintenance,
campaign terminalization/acceptance and workspace publication remain outside
this profile. Unsupported operations refuse before effects.

The private-root input profile is a proposed scope choice for plan acceptance,
not a declaration that existing production roots may be adopted. Old roots stay
untouched. Import versus archive/clean start, whole-root route closure, fencing,
rollback and later operational acceptance remain HP4/HP5/R4/R5 work.

## Parallel implementation and joins

The immediate parallel recommendation is **CE1 implementation plus SC0
characterization/handoff**, after plan acceptance. CE1 has a bounded domain
manifest. Campaign SC1 still needs SC0 to freeze the candidate/profile schemas,
historical producer/source/runtime bindings, gate-receipt contract and oracle
corpus. Available validator source hashes alone do not settle that input profile.
SC0 defines those exact artifacts and reports any remaining owner decision;
SC1 begins only after its handoff is accepted. The later campaign implementation
and claims slices can overlap under the agreed contract. Their owned paths and
local gates are in the lane plans. Neither lane edits the other's package or
creates a second copy of its reference types. Shared workspace/dependency/lock
changes are a serialized handoff to the workspace integration owner, currently
S4-owned; this planning makes no such changes.

The first join checks exact reference interoperability, distinct codecs,
admission ownership, root/revision/content mismatch refusal and uncertainty
readback. The later HP3 gate uses real domain stores and applications, with a
controlled peer only at the external provider boundary. Library parity and
producer-only tests do not establish a working host. Lifecycle/UI development
can continue independently of these domain libraries.

No host responsiveness claim is made by HP2. Before HP3 qualification, its owner
must freeze a supported full-history completion budget and the complete
responsiveness profile. The old failed comparison remains preserved.

## Planning evidence and status

Two separate Astra/xhigh planning contexts own the lane documents; the parent
owns this composition contract. Source/coverage receipts and exact document
bindings are retained in
`/home/bline/.local/state/work-engine/rust-hp2-planning-20261007/`.
Planning evidence is bounded to the selected review profile, not a repository-wide
Node retirement proof. A fresh joined plan review checks the stable drafts and
retains any amendment history separately from implementation evidence.
