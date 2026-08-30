# Deterministic profile contract

Version 1 binds a full accepted or stopped slice-checkpoint lifecycle receipt,
including repository, baseline commit/tree, result commit/tree, disposition,
run/slice/attempt, plan/scope revision, attributed paths and manifest digest,
task patch digest, construction method, evidence cutoff, and limitations.

Version 2 adds a discriminated `slice_checkpoint_candidate_receipt`
construction for a validated immutable candidate checkpoint. Candidate
construction delegates to `slice-checkpoint.validate_candidate_receipt`; the
version 2 lifecycle construction continues to delegate to the strict
accepted/stopped lifecycle validator. The two constructions cannot be
substituted for one another. Candidate evidence cutoff is the immutable
candidate `created_at` value. Version 1 and its schemas remain unchanged.

`slice-checkpoint` remains authoritative for checkpoint refs, lifecycle,
attribution, and receipt integrity. This capability owns only the derived subject
and profile identities, analyzer version, canonical encoding, observations, and
measurement-state vocabulary.

Every analyzer version binds the exact SHA-256 of the checkpoint-owner validator it
is allowed to execute. The digest is checked before Python import. A missing,
dirty, or revised validator fails closed and requires an explicit analyzer
revision; mutable validator bytes never silently change validation behavior or
profile identity.

Every measurement has one of these states:

- `observed`: measurement completed and has a value, including a truthful zero;
- `unknown`: the source does not establish a value;
- `unsupported`: this analyzer does not support the source kind;
- `failed`: an attempted measurement failed and records an error identity; or
- `not_applicable`: the measurement has no meaning for the subject.

Only `observed` carries `value`. Other states carry no fabricated value and must
give a reason. Profiles retain analyzer identity, exact derivation inputs,
coverage, limitations, a subject digest, and a profile digest. Recomputing with a
revised analyzer creates a new profile identity; it does not rewrite the subject
or earlier profile.

Profile validation closes every nested analyzer, subject, checkpoint, manifest,
coverage, and observation envelope. It verifies subject/profile digests and
cross-checks counts, categories, modules, line totals, symbol coverage, and
manifest coverage so recomputing outer digests cannot legitimize contradictory
or fabricated measurements.

Every first-record profile also carries a closed, profile-digest-bound
`provenance` structure. It repeats and cross-checks the producer identity,
analyzer source, and checkpoint-validator binding; identifies the exact subject,
base/result trees, and patch; and records the Git and Python runtime versions
that supply diff, AST, and canonicalization behavior. Each runtime identity also
binds the SHA-256 content digest of the independently resolved executable. Git
operations use that resolved executable directly. Full runtime-bound profile
validation therefore requires the bound analyzer/runtime environment and
recomputes both runtime identities locally; it never trusts a recorded path.
Detached JSON Schema inspection establishes structure only, not executable
identity or runtime equivalence. Structural graph,
invariant catalog, and classifier sources are explicitly `not_used` with reason
`deferred_by_profile_scope`. Their non-use entries are provenance, not evidence
or observations. A later analyzer may mark one used only with a nonempty exact
revision or source identity and a corresponding contract revision.

All current observations are deliberately physical: files, additions/deletions,
binary files, hunks, file categories, test/documentation/configuration counts,
bounded Python symbol changes, and top-level module distribution. It does not
classify semantics, architecture, claims, review truth, outcomes, or policy.
