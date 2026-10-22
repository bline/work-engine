# First root transfer: measured

Status: recorded 2026-10-22, n = 1. Dataset:
[`claude-root-transfer-20261022.json`](claude-root-transfer-20261022.json).

The first use of the user-mediated root transfer procedure introduced by
`c56ab8ec`. Root `bf60b4c1` handed coordination to `c1d31e88` at a quiescent
boundary, with the user performing the one step a root cannot: starting the
successor session.

## The transfer

| | value |
| --- | --- |
| predecessor context at last call | **656,196** |
| successor context at first call | **51,954** |
| reduction | **92.1%** |
| turnaround | **40.0 s** |
| predecessor compactions | **0** |

Turnaround is the predecessor's last recorded activity to the successor's first,
and it sits inside the range measured for the four worker handoffs in R1
(27.9–58.4 s), despite the step between them being a human starting a session.

## Economics

| | mean weighted per call | mean context |
| --- | --- | --- |
| predecessor, last 20 calls | **73,706** | 648,583 |
| successor, 34 calls | **14,610** | 79,362 |

A saving of **59,096 weighted per coordination call, 5.0× cheaper**. The
successor's startup cost over its first five calls was 81,834 weighted, so the
transfer **breaks even after 1.4 coordination calls**. Projected over 100 further
coordination calls: 7.371 Mwt had the root continued, against 1.461 Mwt for the
successor.

## Why this pays back when worker handoffs do not

At the boundary, **87.8% of the predecessor's cost was cached-context re-send**,
2.0% uncached and 10.2% output. The successor: 51.1% re-send. For comparison the
pure-Codex baseline records ~73% re-send at 126–144K context per call; this root
at 648K was at 87.8%.

That is the mechanism. A root's context is **carrying cost** — re-sent on every
coordination call, doing no bulk work — so replacing it reclaims almost the whole
per-call bill. A worker's context is mostly task-essential reading that a
successor has to redo, which is why the `handoff_threshold` replay over Codex
builder traces found break-even restart at about 75K and found that at the
measured 116K floor every threshold lost about 30%.

Stated as the proposed reading of two measurements on different hosts and roles,
not as a measured causal claim. But the direction is large: the same operation
loses ~30% for a builder and pays back in under two calls for a root.

## Correction to the handoff's own record

The root handoff gives the transfer reason as "about 580K context at a quiescent
boundary". The measured peak is **656,196**. Class: a reading stated from
recollection rather than read from the transcript when the handoff was written.
Consequence: none to the decision, which was right in direction; the recorded
reason understates the pressure by about 76K. The check is the max of
`input_tokens + cache_creation_input_tokens + cache_read_input_tokens` over the
transcript.

## A harness fact worth pinning

**The predecessor reached 656,196 with no auto-compaction.** The 202,533 figure
these probes have been using is the *minimum of 21 observed auto-compactions* on
this machine — it bounds where compaction has been seen, and does not predict
where it fires. A session can run more than 3× above it untouched. Any reasoning
that treats 202,533 as a ceiling is wrong, including the margin framing in the
R1 campaign score, which should be read as "the trigger was set below the lowest
compaction observed" and nothing stronger.

## Procedure compliance

All five steps of the registered procedure were followed: the boundary was
quiescent (all 28 predecessor subagents finished, no child crossing, no board
claims held); a handoff was written superseding v1 with v1's digest recorded; the
successor verified the handoff digest
(`dc0b8218a7da80cf016f793329ca2ec3313746c76d2dce5f2666dfb7e4ffc36e`) and
acknowledged ownership as coordinator in the new assignment record before
dispatching.

## Limits

- n = 1.
- The successor had completed 34 calls when this was recorded; its per-call mean
  will rise as it takes on coordination load.
- Break-even uses the predecessor's last-20-call mean as the cost of continuing,
  which assumes its context would have stopped growing. It would not have, so
  **1.4 calls is an upper bound** on the payback period.
