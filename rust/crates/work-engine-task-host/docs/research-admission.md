# Controlled research admission seam

Optional trusted startup `research` nomination names the audit task and subject.
It guards fresh ordinary submission before first preparation, while the task
owner independently guards every library submission variant. Native task
operations have no research producer capability.

Default builds have no production installer. Fresh `research.prepare.v1`,
`research.invalidate.v1` and `research.adopt.v1` refuse without installed
qualification evidence. Scoped `research.read.v1` and
`research.read_adoption.v1` retain readback, including recovery-only operation.
Request/response framing retains the existing 128 KiB envelope and strict JSON
parser. Artifact payload bytes stay outside RPC.

With `controlled-proof`, nomination may explicitly name `controlled_fixture`,
a private file containing version 1 `ControlledFixture`. Its exact bounded
basis variants and notice payloads are loaded once, checked before nomination
publication, and retained in memory. DTO labels cannot select producer roles or
expand these inputs. Successful responses say `qualification: controlled_only`.
The fixture is synthetic evidence, never a production protection/authority or
actual A5 delivery claim. No inference runner is added.

Fresh adoption obtains existing lifecycle preflight outside the owner lock,
then commits through the typed task-owner seam. Existing lifecycle revision and
grant fences still apply. Historical duplicates use their exact saved command;
selected-host unknown reconciliation follows the existing policy and cannot
rebase to newer preparation, context or grant. Recovery-only refuses fresh
mutation and reads saved receipts; later invalidation does not automatically
adopt again. The controlled owner-commit cut remains available for crash/retry
qualification.

Controlled fixture revision pins equal the SHA256 of the referenced dependency
artifact bytes. The retained result artifact is typed JSON with `dependencies`
(the full exact pin map) and `text` (advisory recommendation text). Missing or
mismatched bindings refuse, including when a notice predates every preparation.
Changed retained inputs plus a newly bound recommendation can resolve that
notice. Canonical material retains the advisory text and exact result digest;
these checks do not authorize an issuer or establish research judgment.

The complete selector count, identifier syntax and supported `research.` or
`decision.` prefix are checked before durable nomination. Unsupported-selector
startup refusal preserves the store and subsequent ordinary handler submission.
