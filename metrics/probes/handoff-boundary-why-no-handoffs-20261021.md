# Why no handoff occurred: diagnosis and candidate fixes

Status: diagnosis, 2026-10-21T22:45Z, written in the measurement role after the
[pilot score](handoff-boundary-pilot-score-20261021.md) closed at quota with
outcome A never observed. These are **hypotheses with evidence, not established
causes**: no fix below has been tested, and the pilot cannot attribute a zero
handoff rate to any single mechanism. This document adds no instruction.

Subject text is the brief's "Context check-in block for task briefs" at commit
`84fe837c`: 504 words in the quoted block.

## 1. The check is taken before the next task is known

The block says to check "at each logical task boundary, before starting another
substantial task" and to weigh "next-task size". In the clearest failure the
next task did not exist yet at the boundary.

`auth_worker`, exact sequence from its rollout:

| time | event |
| --- | --- |
| 21:50:22 | check-in, 179,007 of 258,400 (69.3%) |
| 21:50-21:52 | records `continue`, reason "finish bounded checks/report with retained context" |
| 21:52:23 | parent sends a new substantial task - the R1-R5 remediation dispatch |
| 21:57:32 | compaction at 191,069 (73.9%) |

The child weighed next-task size two minutes before the parent told it what the
next task was. "Next-task size" is the only one of the three registered
considerations that can argue for handoff, and at check time the child does not
have it. This is a sequencing defect, not a judgment failure.

## 2. The decision sits with the party that has neither the information nor the authority

A child can only request transfer; the parent verifies the packet, launches the
successor, publishes the binding and obtains acknowledgement. The parent is also
the party that knows the size of the task it is about to dispatch, because it is
writing it. The block nevertheless assigns the decision to the child.

The parent is not equipped to take it either. `agents --parent-thread --json`
returns `thread_id`, `root_session_id`, `parent_thread_id`, `agent_path` and
`rollout` - metadata only, no per-child usage - and the brief describes it as
"metadata-only child discovery". A parent wanting a child's pressure must run
`--session <child id> --rollout <child rollout>` per child. The capability
exists; nothing asks the parent to use it, and the obvious place to surface it
does not.

## 3. Two of the three registered considerations can only favour continue

The block asks for "a short reason considering next-task size, retained context
value and reconstruction cost". Retained context value is a benefit of
continuing. Reconstruction cost is a cost of handing off. Next-task size is the
only term that can point the other way, and per 1 it is unknown when the check
is taken. Every recorded reason in the campaign reads accordingly - "next checks
and hardening are bounded", "only result packaging/collection remains",
"reconstruction would cost more than continuation".

The quantity that would argue for handoff is absent from the list: **remaining
headroom before compaction fires, against the expected cost of the next task.**

## 4. The reported fraction uses a denominator where compaction does not happen

The check reports a fraction of `model_context_window`, 258,400. Across 33
compaction events the last recorded reading before the marker spans
171,541-239,002, median 199,918 - 66.4% to 92.5% of that window. A 69-71%
reading presents as comfortable while sitting inside the observed firing range,
which is where both post-change workers were when they chose continue.

A level against the window systematically overestimates headroom. Reporting
headroom against the observed trigger band instead would have shown
`auth_worker` roughly 12,000 tokens from the band's lower bound rather than
79,000 from the window.

## 5. The block is 12 prohibitions and one undefined permission

Counted in the 504-word block: twelve distinct cautions or prohibitions - "not a
product retirement rule", "not a threshold", "do not count", "do not substitute
parent counters", "does not claim task completion or permission to abandon
children/processes", "does not transfer the root", and so on. Exactly one clause
says when to hand off: "When replacement is warranted", with warrant undefined.

Nothing in the block names mid-task compaction as an outcome worth avoiding, and
the records show agents recovering from compaction routinely and truthfully via
Wind Walker plus durable continuation - `session_quota_repair_review` calls it
"automatic same-session compaction recovered via Wind Walker and durable prior
result". So the agent faces a cheap automatic fallback against a five-step
ceremony that imposes work on its parent and is described mostly in terms of how
it could be wrongly claimed. Continue is the rational choice, and the caution is
load-bearing elsewhere, so this is a balance problem rather than an excess.

## 6. The confound: a worker handoff may be redundant with the decomposition

This is the possibility that most changes the question. The orchestration
already renews context by spawning fresh children per slice - seven fresh
threads under root `01a15f02` in ninety minutes, each with its own window from
zero. A worker-owned handoff only matters when a **single** assignment outgrows
a window, which good decomposition makes rare.

If that is right, a zero handoff rate among children is partly correct behaviour
rather than a failure to comply, and the pilot was measuring the level of the
tree where the need is weakest. The level where transfer genuinely matters is
the root: it accumulates across every slice, it compacted four times, it read
77.8% and 84.1% at decision points - and it is the one role structurally barred
from transfer, recording class E at every boundary, every time.

**The mechanism is missing exactly where the need is demonstrated, and present
where the need is weak.**

## Candidate fixes, in the order the evidence supports them

1. **Move the check to pre-dispatch and to the parent.** Before sending a
   retained child a substantial new task, the parent reads that child's usage and
   decides retained-versus-fresh. This puts the decision where both the task size
   and the launch authority already are, and collapses "handoff" into "dispatch
   to a fresh child", which the orchestration performs constantly. No new
   ceremony, and it addresses 1, 2 and 6 together.
2. **Give the decision the missing quantity.** Headroom to the observed trigger
   band, plus an expected cost for the next task - even coarse buckets would beat
   an undefined "size". Surfacing per-child usage in `agents` would make 1 cheap.
3. **Change the denominator** to the measured trigger band, keeping the window
   as secondary. The band is an empirical lower bound and should be restated as
   more events accumulate.
4. **Add one affirmative criterion with a number**, declared as a pilot
   hypothesis rather than a product rule. The brief's resistance to thresholds is
   sound, but "no threshold" has so far produced "no decision", and a stated
   candidate can be tested and refuted where a judgment call cannot.
5. **Name mid-task compaction as the cost on the continue side**, so the two
   options are described in comparable terms.
6. **Resolve the root case or exclude it.** Either build the verified
   root-transfer mechanism the brief describes, or record that roots are out of
   scope and score children only. Scoring class E repeatedly adds nothing.
7. **Instrument the declare-to-acceptance interval** whenever outcome A first
   occurs. It remains the gating measurement for any transfer-cost claim, and
   amendment 1 retracted a latency conclusion for want of it.

Fixes 1-5 change the instruction and so require a new cohort; under the
registration they cannot be applied to the closed one. Testing any of them needs
account quota to return.

## Amendment 1, 2026-10-22: the primitive does not exist, and I had already measured that

Raised by the user: Codex handles children as subagents with almost no overhead.
Correct, and it goes further than this document credited.

**Verified on today's rollouts.** Tool names actually invoked by role:

| role | tools |
| --- | --- |
| worker (`auth_worker`) | `exec`, `send_message`, `wait_agent` |
| supervisor (`b2a`, `b2b`) | `exec`, `send_message`, `wait_agent`, `spawn_agent`, `followup_task` |

**No context-replacement primitive appears at any level.** A worker cannot
replace itself and has no tool that could; a supervisor's only renewal mechanism
is `spawn_agent`, which creates a *new* child rather than a successor to an
existing one. The contract's `handoff_ready` to parent-verified-successor
sequence therefore rests on an operation the tool set does not provide. What it
does provide is cheap subagent spawn, which the orchestration already uses
constantly.

**This was already in the ledger, measured by me on 2026-10-14**, and this
document failed to cite it. The handoff lever row reads: *"not actuable in these
sessions: no `new_context` tool appears in any measured session's tool list
(`exec`, `spawn_agent`, `wait_agent`, `followup_task`, `send_message`,
`list_agents`, `sleep`, `wait`, `request_user_input_async`, `interrupt_agent`),
and all 76 observed window replacements fired at a 208-244K ceiling rather than
a boundary."*

### Correction

**Wrong claim:** that the zero handoff rate is explained by the five instruction
defects above plus the redundancy confound, with "move the check to pre-dispatch"
as the lead fix.

**Class:** an absence explained without checking whether the capability exists -
the same failure rule 7 covers, with the aggravating detail that the
completeness basis was in my own ledger row and I did not consult it before
diagnosing.

**Consequence:** it aimed the remedy at instruction wording when the primary
blocker is a missing primitive. Acting on it would have meant rewording a brief
to request an operation no role can perform, and the brief's authors would have
complied and still produced no handoff.

**Missing check:** enumerate the tools available to each role before explaining
why a role does not perform an operation. Now done, and recorded above.

### What survives, and what it reorders

The sequencing defect in section 1 is unaffected - `auth_worker` did weigh
next-task size two minutes before the parent sent the next task, and that is
still worth fixing. Sections 2 and 6 are strengthened rather than weakened: the
decision belongs with the parent precisely because `spawn_agent` is the parent's
tool and the only renewal mechanism there is.

But the priority inverts. **Fix 1 is no longer "reword the check" but "name the
mechanism the brief actually has."** The brief describes a transfer sequence that
reads as a distinct capability; the real capability is "dispatch the next task to
a fresh subagent instead of the retained one", which costs a spawn.

And M22 bounds how often even that is worth doing. The cost-optimal threshold is
conditional on the restart floor, not a context level: break-even total restart
is about **75K**, and at the measured **116K** builder floor every threshold
loses roughly **30%**. Spawning is nearly free; bringing a fresh child up to the
retained one's state is not, and that is where the cost sits - against a
**17,716-token** standing floor for a fresh root (M21).

So the honest reading of the zero rate changes. It is not mainly non-compliance,
nor mainly bad wording. **The mechanism the pilot asked for does not exist, and
the substitute that does exist is usually not worth using at the measured restart
floor.** A pilot asking agents to choose handoff was asking them to choose an
option that was unavailable, uneconomic, or both - and `continue` was the correct
answer more often than this document allowed.
