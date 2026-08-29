# Review-subject service

The review-subject service is a host-mediated App Server capability for creating
an immutable attributed Git checkpoint and deriving a deterministic physical
change profile before reviewer selection or execution. It does not select or
execute reviewers, infer semantic risk or quality, record findings, accept a
slice, or publish a user-visible commit.

## Ownership

The `slice-checkpoint` backend remains authoritative for checkpoint requests,
isolated-index tree construction, immutable candidate and lifecycle commits,
path attribution, private refs, receipt validation, and compare-and-swap
conflicts. The `code-change-profile` backend owns only its derived subject and
profile identities, analyzer/provenance identity, physical observations,
coverage, limitations, and measurement-state vocabulary.

The physical profiler invokes the exact checkpoint-owned lifecycle validator.
The host bridge verifies the canonical checkpoint and profiler source digests
before importing either module and refuses a checkpoint source that differs
from the profiler's analyzer-version binding. Because the legacy profiler
normally discovers its validator inside the repository being profiled, the
bridge injects the already verified canonical checkpoint module. It does not
copy or write validator code into the subject repository.

Claim Evidence's Git checkpoint observation is a separate boundary. It verifies
an already identified commit and tree as claim evidence; it neither constructs
review subjects nor owns checkpoint lifecycle or physical-profile semantics.

## Mediation and effects

`createReviewSubjectService` exposes only these mechanical operations:

- create a candidate checkpoint;
- transition a candidate to an accepted or stopped lifecycle checkpoint;
- validate a lifecycle receipt;
- create a physical change profile;
- validate a physical change profile.

The backend is invoked without a shell, with bounded time and output. Requests,
backend source identities, operation identity, and result envelopes are closed
and fail on mismatch. Import, identity, schema, and dispatch exceptions are
normalized to a bounded single-line error response; process-level signals are
not swallowed as ordinary backend errors. The service is exported for host
composition only. S7
does not register it as a dynamic tool, grant it through a runtime manifest, or
make it directly invocable by a reviewer role.

Checkpoint operations may write Git objects and only private refs below
`refs/work-engine/checkpoints/`. The canonical checkpoint backend uses a
temporary index and never moves a branch, changes the real index or working
tree, creates a tag, pushes, or infers undeclared paths. Profile operations are
read-only and derive evidence from the immutable baseline/result trees and the
attributed manifest rather than current workspace state.

The optional profile repository is deliberately not restricted to the App
Server workspace root. It identifies the Git repository containing the exact
checkpoint subject and is validated by the canonical legacy analyzer; limiting
it to the host workspace would prevent profiling immutable subjects from other
repositories and would change the retained v1 contract. This is host-mediated
input, not reviewer-selected filesystem authority.

Every operation binds the complete v1 backend identity: both canonical source
digests are loaded, and the profiler's checkpoint-validator fingerprint is
verified even for checkpoint-only operations. That coupling is intentional at
this compatibility seam. It prevents different operations exposed by one
service instance from observing different checkpoint semantics; a narrower or
split backend identity requires a separately versioned contract.

## Physical profile boundary

Version 1 observations are physical: files, additions and deletions, binary
files, hunks, file categories, test/documentation/configuration counts, bounded
Python symbol changes, and top-level module distribution. They carry no
reviewer selection, semantic risk, authority, persistence, concurrency,
security, topology, quality, findings, or acceptance meaning.

A later semantic characterization may consume a validated physical profile,
but it must be a separately attributed and versioned projection with its own
identity. It cannot acquire the deterministic profile identity, mutate the
profile, or silently revise its observations.

## Compatibility and lifecycle

S7 deliberately retains the canonical Python v1 implementations behind the
host service. App Server mediation adds no replacement Git or profiling
semantics. Candidate, accepted/stopped, and physical-profile results retain the
legacy schema and identities, and parity tests compare mediated output with the
direct legacy producers.

This is an explicit compatibility seam, not legacy retirement. A future native
backend requires a new version, exact compatibility evidence, consumer cutover,
and separately authorized ownership change.
