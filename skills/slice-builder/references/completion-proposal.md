# Completion commit proposal

`<skill-dir>` is the slice-builder skill's directory.

When the supervisor returns an accepted checkpoint under an enabled
`engine_config.slice_completion_commit.prompt`, retain this builder while its
context remains useful and ask it to compose one compact schema-version-2
`commit_proposal`. Bind its one-line subject, body, and exact task-owned path
set to the accepted checkpoint commit, tree, and task-patch digest. Include
structured production provenance naming the producer and the durable evidence
that supports the proposal; provider or model identity is descriptive, never
authority. Return the proposal as a separate completion artifact beside the
receipts. It conveys semantic context; it is not part of the audit receipt and
never authorizes a Git mutation.

If this context is lost or no longer trustworthy, a replacement may reconstruct
the proposal only from the accepted checkpoint, attributed manifest, and
durable compact semantic consequences. It records that route truthfully in the
same open provenance structure. When those artifacts do not support an accurate
subject or body, refuse the proposal or surface the uncertainty; do not infer
semantic equivalence or replay a transcript merely to satisfy the schema.
