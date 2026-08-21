# Slice 2 — Make Completion-Commit Receipts Authoritative

## Protected consequence

A durable terminal record claims `slice_completion_commit.state: created` only
when the owning adapter authoritatively established the repository, proposal,
commit object, parent, tree, message, publication target, and resulting state.

## Observed defect

Checkpoint receipts pass through lifecycle projection that observes their Git
objects and private refs. Completion-commit receipts are copied directly into
the assembled terminal record. The append validator checks their shape and
digest fields but does not establish that the repository or commit exists.

The review passed a fabricated `created` receipt with a nonexistent repository
and invented commit OID through `validate_completion_commit_projection`.

## Likely ownership boundary

The completion-commit adapter owns Git observation and receipt authorship. The
terminal finalizer owns consuming an authoritative adapter result. The generic
append boundary should validate durable contract consistency without silently
pretending that string-shape validation is host observation.

Planning must decide how authoritative adapter evidence reaches finalization;
the campaign does not prescribe importing a particular module, signing a
receipt, or re-running every mutation precondition.

## Acceptance consequences

- A caller cannot fabricate `created` by constructing a shape-valid object.
- Finalization rejects a missing repository, missing commit, wrong parent, wrong
  tree, wrong message/proposal binding, or publication mismatch whenever the
  receipt contract claims those facts.
- Verification does not mutate the repository.
- `declined`, `refused`, and any retained pending representation preserve their
  actual evidence boundary and do not claim a Git object was created.
- Checkpoint and completion provenance remain distinct while both are truthful.
- Resume never promotes an unverified completion claim into current state.

## Required vertical proof

Feed the finalization boundary a correctly shaped fabricated `created` receipt
whose repository and commit do not exist. Observe rejection without appending
bytes. Then feed a genuine adapter-produced receipt and observe one durable,
correctly bound terminal projection.

## Insufficient substitutes

- More regex or object-shape validation.
- Trusting model instructions to call the adapter first.
- Testing only the happy path in which the same test directly calls the adapter
  and then finalizer.
- Revalidating the proposal digest without validating the claimed Git result.
