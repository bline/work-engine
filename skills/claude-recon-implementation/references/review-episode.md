# Review episode: specialist, state and recovery

For a supervisor-selected semantic specialist, also pass the exact selection,
immutable subject binding, and complete repository-local specialist contract.
Require the provider result to preserve that contract's applicability,
findings, limitations, and advisory authority rather than collapsing it into
this adapter's generic implementation verdict. A selected
`agent-instruction-review` reads its `SKILL.md` and finding contract, returns a
revision-bound `applicable` or `omitted` result, and remains read-only. Claude is
the independent-review provider in that execution; the specialist still owns
the diagnosis and finding shape.

Outside paired calibration, when authority-bound independent-review state is
available, make it part of the initial reviewer call rather than adding it
after findings exist. Before
provider entry, publish the exact review subject, reviewer UUID, writer
generation, and episode authority manifest. Launch the Work Engine MCP with
that manifest and expose its narrow review-state tools alongside the reviewer's
read-only evidence tools. The reviewer begins its episode, performs the review,
records its attributed initial result, and reads the resulting state before it
returns. The coordinator verifies the returned provider session ID; the
reviewer is not required to self-observe runtime identity through a tool that
does not expose it.

On remediation, resume the same provider session with the same episode-bound
state tools. Give it the new exact subject and bounded delta; it records the
remediation subject, re-evaluates its own findings, and publishes that
re-evaluation before returning. If state publication fails, retain the exact
structured reviewer result, report the operational state as incomplete, and
repair or reconstruct truthfully. Never report an attempted transition as
durable. This standard path applies only to retained review episodes; do not
give mutable episode state to disposable placement, reconnaissance, diagnosis,
or falsification calls.

If supervisor context is replaced while the provider session remains
available, recover the pending review obligation and runtime session ID through
`resume_active_slice.py`, then resume that session. If the provider session is
lost, reconstruct only from durable subject, findings, and gate evidence and
record a replacement or reconstructed reviewer truthfully; do not claim
retained context.
