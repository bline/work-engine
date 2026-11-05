# Pressure-admitted stop qualification

The standalone harness adds `--cancel-after-pressure` and
`--deadline-after-pressure`. Existing success, readiness cancellation and
same-binary recovery modes remain available. No Host or receiver source changes
are part of this qualification.

Both modes require the existing Host-validated native message and fake-request
artifacts to match the actual session and released text. They also observe an
active `docker exec` process with the fixed tool-output marker, before its output
has been collected. A finite shell delay precedes the original src/lib.rs-only
fixture edit. Missing, late or already-completed evidence keeps a case held.

Cancellation invokes the existing `request_cancel` after recording the witness.
Deadline mode assigns a fresh immutable 50-second Job deadline after setup and
holds release until at most 25 seconds remain. It uses the original SDK reading,
rejects age above 27 seconds, and never changes the admitted Job or journal.
The fixture sleeps 29 seconds; the existing command timeout remains 30 seconds.
The raw-outcome verifier additionally checks the tool command's timeout flag is
false, the RPC expiry flag is true, no cancellation marker exists, and observed
tool termination precedes the ordinary command timer. Linux boot-clock/process
start ticks provide an elapsed-time upper bound; unavailable or late timing is
held, not converted to zero.

A zero harness exit means the expected negative case was observed. Both actual
jobs must remain `not_established`, with no final acceptance proof. A completed
Host job cannot be relabelled cancellation. Receipt delivery ACK still says
`owner_pending`. Native completion events and Host task acceptance are distinct.
Actual signal exit status can remain unavailable; real daemon exits and removal
are collected separately by Host. Same-binary recovery never launches work.

Qualification uses fresh private state, environment clearing, network-none,
immutable emitted/copy/execution custody and raw native/pressure/tool evidence.
The old unknown checkpoint journal is historical and is never replayed or settled
by these cases. This is a local fake-provider qualification, not live inference,
shared service delivery, comprehension, task authority or context reset.
