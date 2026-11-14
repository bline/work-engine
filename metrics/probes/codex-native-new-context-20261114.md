# Native Codex child context replacement, 2026-11-14

One disposable native child completed two preparation turns, then invoked its
own `new_context` once and recovered through durable state. The parent
independently inspected the native rollout and collected the child's normal
final result. This establishes the bounded recovery fixture, not autonomous
pressure handling, an optimal reset interval, or every host's behavior.

The child was `01a1d862-7d4f-71e1-8687-5f4e6a3e0bf0`, with native parent
`01a1cd53-cd0c-7791-a365-22bbb91b00cd`. Its rollout records the call at line 104,
replacement at line 108, and post-replacement applied settings at line 111.
The session ID and observed `gpt-6.1-sol` / high / default settings survived.
Replacement history contained no generated summary and excluded the secret
nonce used to check recovery. The child read Wind Walker, the chatboard pointer
and the continuation, recovered the exact nonce and product 391, and returned
through normal collection. Input fell from 58,688 tokens before replacement to
23,170 in the first real model request afterward; the intervening zero was a
counter reset, not a request.

Exact parent result and child recovery evidence are retained locally at
`/home/bline/cbm-work/codex-child-new-context-probe-20261114/RESULT.json` and
`recovered.json`. The result records call/replacement timestamps, native
evidence locations and the parent's collection checks. The earlier external
child compaction-RPC failure is a separate experiment; this result does not
invalidate it.

The root subsequently rolled over automatically at 04:45:10 UTC after a
254,921-token input, then invoked its own tool at 04:55:24 under the session's
185k preparation instruction, around 191k tokens. Recovery reads after the
first rollover grew input from about 24k to 98k in two minutes. These observations
show substantial recovery overhead and do not establish a benefit from those
numeric cutoffs. The user chose configurable thresholds for the skill release.
