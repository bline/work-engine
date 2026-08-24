# Idea intake contract

The intake capability owns one attributed interpretation of an exact raw-idea
revision. The raw source owns what its author wrote. Proposal formation owns
whether surviving candidate meaning becomes a proposal, proposal packets own
formed identity and lifecycle, stronger domain owners retain their facts, and
repository mutation owns any later cleanup.

## Subject and history

An intake record binds a stable idea ID and assessment ID to a full Git commit,
the blob at a repository-relative historical path, a bounded line range, and
the SHA-256 of the exact bytes in that range. Current-path movement or later
edits do not retarget this subject. A changed interpretation increments the
assessment or claim revision instead of overwriting the earlier Git history.

The referenced narrative owns nuanced current interpretation. The closed JSON
record owns only identity, provenance, authority, evidence cutoff, references,
claim revisions, relationships, dispositions, uncertainty, reopening
conditions, proposal references, and downstream non-effects that consumers
must distinguish mechanically.

## Evidence, judgment, and authority

Evidence entries remain attributed references to their actual owners and carry
a closed integrity, freshness, and verification profile. Repository-file and
Git-object evidence is mechanically resolved against its immutable bytes.
External attestations remain explicitly unverified and cannot authorize an
adjudicated disposition. A manual
reconciliation snapshot is prior evidence; after a finer-grained assessment
exists it is not a competing disposition owner. Similarity, a proposal match,
or implementation evidence may nominate a relationship but does not adjudicate
it. An adjudicated claim or relationship must cite human-decision or accepted-
contract evidence with authority for that decision.

The assessment may investigate, compare, decompose, ask focused questions, and
revise its interpretation in any useful order. These are affordances, not a
questionnaire or universal state sequence. If product intent or authority is
missing, preserve the unresolved claim and the smallest reopening condition
instead of inventing a disposition.

## Validation and projection

Run the repository-local validator with either command:

```text
python3 scripts/idea_intake.py validate <record.json> --repository <git-root>
python3 scripts/idea_intake.py project <record.json> --repository <git-root>
```

Validation checks the closed version-1 shape, contained local references,
source commit/blob/range integrity, unique identities, relationship and
evidence endpoints, authority references, handoff-state consistency, and false
non-authorization fields. It does not establish that extraction is accurate,
two ideas are equivalent, implementation is complete, a candidate is valuable,
or formation should proceed.

The projection contains only candidates marked ready for proposal formation,
their exact intake/source identity, relevant evidence and relationships,
authority, uncertainty, proposal references, and non-authorization. It is a
deterministic convenience view; the intake record and Git history remain the
owners. A projection may be empty.

## Downstream effects

Intake never accepts a proposal, authorizes implementation or cleanup, settles
permanent placement, or changes roadmap priority. `proposal_refs` describe
already separately owned proposal state; they do not create it. Movement,
archival, merge, redirect, or deletion remains a separately authorized
repository mutation whose provenance must continue to resolve.
