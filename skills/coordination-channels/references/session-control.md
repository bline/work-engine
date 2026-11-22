## Session control subscription

The wait interface can also deliver local context-pressure observations on
`control:session:<verified-id>` (with `/agent:<id>` for a Claude child).
Enrollment binds the session, transcript, pinned telemetry binary and explicit
planning levels. See [manual control setup](../../manual-orchestration/references/context-control.md)
for exact options, ownership, recovery and campaign exposure constraints.

Control events are typed observations in `control_events`, separate from board
messages. They are not posted to the shared board and grant no authority.
Listening to board topics or using [Pi Chatboard](pi-chatboard.md) does not
enroll context monitoring. At startup or recovery, establish the actual
context binding and timely delivery route under the manual context contract,
or keep an owned explicit-check fallback. A quiet listener is not proof of
normal pressure or active enrollment.
The listener suppresses unchanged pressure and carries an opaque control cursor
across invocations. A forecast concerns approaching an empirical band, not an
exact reset time. Use the event to inform the owning handoff decision; preserve
explicit checks when observation or delivery is unavailable. Do not infer that
support for control subscriptions has automatically enrolled any agent.
