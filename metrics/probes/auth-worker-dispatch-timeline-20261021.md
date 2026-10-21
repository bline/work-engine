# auth_worker: the compaction followed a retained dispatch

Record, 2026-10-21, read from two rollouts. It corrects one line of the
[diagnosis](handoff-boundary-why-no-handoffs-20261021.md) and is the source for
the 190,902 reading cited by the manual-orchestration skill.

- Child: `/root/bounded_inference_b2a/auth_worker`, thread
  `01a1605b-9162-70a0-8810-43df23b45e7f`, rollout
  `~/.codex/sessions/2026/10/21/rollout-2026-10-21T15-26-16-01a1605b-9162-70a0-8810-43df23b45e7f.jsonl`.
- Parent: `/root/bounded_inference_b2a`, thread
  `01a1605a-2b3a-7601-97a2-7ad17fe9f237`, rollout
  `~/.codex/sessions/2026/10/21/rollout-2026-10-21T15-24-44-01a1605a-2b3a-7601-97a2-7ad17fe9f237.jsonl`.

| time (Z) | rollout | record | child input before it |
|---|---|---|---|
| 21:50:22.589 | child | context-status check-in, records `continue` | 179,007 |
| 21:52:03.260 | child | `send_message` to parent (result), then `wait_agent` | 189,518 |
| 21:52:23.795 | parent | `send_message` target `auth_worker` | - |
| 21:52:23.821 | child | inbound `Message Type: MESSAGE` | 190,902 |
| 21:57:06.853 | parent | `followup_task` target `auth_worker` | - |
| 21:57:32.427 | child | `compacted`; last reading before it 191,069 at 21:57:06.869 | 191,069 |
| 21:57:34.306 | child | inbound `Message Type: NEW_TASK` | 0 |

Readings are `event_msg.token_count.info.last_token_usage.input_tokens`, window
258,400. Message bodies are encrypted in both rollouts, so the task's content
(the diagnosis calls it the R1-R5 remediation) is `not established`.

**Correction to the diagnosis, section 1.** It places the new substantial task
at 21:52:23. That record is a `send_message`; the substantial task is the
`followup_task` at 21:57:06, and the compaction came 26 seconds after it. Error
class: a record identified by timestamp without printing its type. Consequence:
the sequencing finding stands, but the decision that preceded the compaction was
the parent's retained-versus-fresh dispatch, not the child's check-in. The check
that catches it: list the parent's tool calls by name over the window. The
child had already returned its result and was idle; the parent made no
context-status call on `auth_worker` between 21:52 and 21:57:06.

The skill cites 190,902 as the child's latest reading while it waited, which is
also the value a pre-dispatch check would have seen; the final pre-compaction
reading is 191,069.
