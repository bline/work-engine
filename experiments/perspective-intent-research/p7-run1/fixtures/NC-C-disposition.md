# Cleanup implementation disposition (synthetic fixture NC-C, derived document)

Status: constructed negative control for P7 run 1.

## Implementation choices (the runner team's, revisable)

These are our own choices for meeting the cleanup requirement. They are not new requirements, and we may change them if a better route appears.

- We will create each job's container with the container engine's automatic-removal option, so the engine removes the container after the job's process exits.
- On cancel, the runner issues the stop request to the job's process.
- We will keep the independent post-exit absence check exactly as the requirement states.

## Acceptance

Unchanged from the cleanup requirement: a cancelled job is reported as cleanly cancelled only when the independent post-exit check confirms the container's full ID is absent from the launching engine.
