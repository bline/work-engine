# Native Codex token-budget recovery

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: using token_budget, preparing native new_context, or recovering after its automatic or requested replacement.

## Codex token-budget refresh deadline

When the session uses Codex's `token_budget` feature, `new_context` replaces
conversation history without generating a summary. The automatic limit takes
the same route. Missing preparation can leave the continuing agent unable to
recover its assignment or outstanding obligations. Apply this rule to every
role, including native workers and retained reviewers; ordinary compaction and
its summarization are not the recovery fallback in this mode.

Choose preparation and completion levels for the session's actual model,
rollover configuration, expected call growth and preservation work. These
levels are configurable operating margins, not universal token cutoffs or
provider timing guarantees. An explicit deadline in the owning session or
campaign configuration remains binding. A preparation warning calls for a
capacity decision and making recovery safe; it need not mean an immediate
reset unless the owning instructions require one. Account for recovery and
rediscovery cost as well as remaining headroom when choosing when to refresh.

A configured deadline is not a cost-optimal reset interval. The
[M22 replay](../../../metrics/efficiency-ledger.md#what-an-earlier-handoff-is-worth-replayed-on-44-builder-threads)
found that earlier replacement's modeled benefit depends on total restart and
rediscovery cost; its older models and host do not calibrate this route.

- Before intentional replacement, invoke Wind Walker, preserve required meaning
  through its existing owners, and publish a concise chatboard recovery note
  with the verified session ID, assignment and continuation reference. Ensure
  the note can be found after reset; the latest 20 messages may not contain it.
- Leave enough headroom for preservation and invoke your own `new_context`
  before a configured completion deadline. If the deadline is near or ordinary
  work could consume preservation headroom, suspend ordinary work and make
  recovery safe. A supervisor may request this action, but cannot execute the
  model-only tool for its child. Do not use a compaction helper for this route.
- In the new window, follow AGENTS.md, invoke Wind Walker and recover through
  the board note and its existing owners before resuming the assignment. An
  unchanged session ID or lower reading alone does not establish recovery.
  Missing continuation-critical meaning is a recovery failure: report it to
  the current collection owner and stop dependent work instead of guessing.

Keep the continuation usable during ordinary work so automatic rollover also
has a recovery path. This uses the existing continuation, board and pressure
listener; it requires no successor handoff or transfer of ownership. Preserve
review episodes and process/child collection responsibilities. If the tool or
its recovery route is unavailable, stop ordinary work and report the limitation
while preservation headroom remains. These instructions do not enable the
feature or set its numeric thresholds. The [native child probe](../../../metrics/probes/codex-native-new-context-20261114.md)
established one supervised child recovery, not autonomous pressure handling or
an optimal reset interval.
