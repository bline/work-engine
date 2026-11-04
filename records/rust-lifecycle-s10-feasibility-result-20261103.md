# Rust lifecycle S10-F feasibility result — 2026-11-03

Status: first bounded attempt complete. A separate restricted Astra reviewer
accepted the local capability observation with no P1/P2/P3 findings. The
original native profile's evidence-complete feasibility remains unestablished;
full S10/M6 and prior 26-test acceptance retain their existing standing.

The user directly transferred S10 supervision to session
`01a1a2c0-7191-76f2-afa1-16d32d5c4883`, then explicitly approved the
[S10-F assignment](../planning/s10-f-feasibility-assignment-20261103.md).
The approval arrived at 19:42:43.796 UTC, with a 90-minute/$15 total modeled
speed-adjusted API cost cap. The fixed hard deadline was 21:12:43.796 UTC;
productive work stopped well before the 75-minute/$12 boundary. No historical
allocation was reopened.

## Actual observation

A fresh Rust orchestrator transferred an open-file description to a target
under actual Linux `strace`, using `SCM_RIGHTS` after exec. The orchestrator
was outside the target's traced lineage. Both cases used the same target
acquisition, read, close and exit path.

| Case | Target observation | Outside holder after target terminal |
| --- | --- | --- |
| Control | Receives the descriptor, reads 44 bytes, closes it and exits successfully | Closed before releasing the target's common execution gate; terminal check returns EBADF |
| Retained holder | The same recorded acquisition-through-exit suffix | Still open; matching device/inode and shared file offset advanced from 0 to 44; closed during final fixture cleanup |

The selected acquisition-through-exit trace suffixes were byte-identical.
Whole raw traces were **not** identical: startup mapping reads contained
different ASLR addresses. The experiment recorded selected syscall classes,
not complete syscall histories. Its creator/holder oracle is fixture-attributed
evidence outside the target projection, not independently admitted native-run
evidence.

This demonstrates a concrete limit: that target projection alone cannot
establish the relevant outside holder's absence or lifetime. It also shows
that another evidence owner can supply the missing fact in this controlled
fixture. It neither qualifies that producer for the original native workload
nor proves that every alternative tracing/observation route is impossible.

Execution used a private `bwrap` filesystem, network/PID/user isolation, dropped
capabilities, cleared environment and pinned read-only tools/inputs. Custody
recorded a successful run and zero leftovers. Both target processes were waited,
both holder descriptors were finally closed, and the private PID namespace
ended. No original S10 native workload, live-provider proof, production change,
privileged observation or existing-user-process inspection occurred.

## Evidence and review

- [Assignment, exact human approval, bindings and cost scope](/home/bline/.local/state/work-engine/s10-supervision-20261103/assignment.json).
- [Worker result, source/output freezes, raw traces, oracles and custody references](/home/bline/.local/state/work-engine/s10-supervision-20261103/witness/result.json).
- [Immutable review subject](/home/bline/.local/state/work-engine/s10-supervision-20261103/review-subject.json), SHA-256 `25d77c4eb21a01863da83a502bdc2adb3d4973fda743c8c6f791cd7a822b4502`.
- [Separate reviewer verdict](/home/bline/.local/state/work-engine/s10-supervision-20261103/review-report.json) and [root disposition](/home/bline/.local/state/work-engine/s10-supervision-20261103/root-result.json).

The reviewer inspected exact source, recorded execution, source/binary identity,
both manifests and archives, descriptor inheritance, oracle timing and trace
comparison. Its actual new session was observed from outside the reviewer as
Astra/high/default, read-only, network restricted and approval never. It did
not execute the fixture or inspect builder trajectory. This was focused
same-provider, different-model review, not a provider-independence claim or
review of full S10. A failed preliminary build was preserved separately.

Recorded cost at 19:54:28 UTC was **$2.7516064 across 43 calls**, including root,
worker and reviewer; later closing/reporting costs are additional. The root
disposition references the later cost snapshot and preserves final-tail
uncertainty. These are modeled costs, not actual subscription-quota charges.

## Feasibility disposition and next decision

The original eight completeness terms and 17 predicates remain required.
This fixture does not establish the original native initial state, all relevant
shared/inherited/outside-lineage history, native caller/build/config/store
custody, loss/conflict completeness or descendant settlement. Kernel metadata
was captured read-only; no BPF attachment or other privileged mechanism was
authorized or tested. That access boundary is not proof of technical
impossibility or exhaustion of unprivileged alternatives.

Close this attempt without admitting S10-Q. The observation resolves the narrow
trace-projection question; spending the remaining cap on routine consumer
tests would not establish the missing original world facts. No automatic second
research attempt, reviewer loop or qualification launch follows.

The supervisor recommends an explicit product/acceptance decision:

1. **Preferred delivery direction: a constrained development profile.** Define
   the exact initial resource/ownership boundary, enforceable capabilities and
   Work Engine operation/history/authority claims to deliver first. Name which
   original claims this preserves and which it changes. Its actual evidence
   owners still need to establish those claims; containment is not proof of
   independent history. Keep stronger unsupported OS-world claims and full
   replacement open. This is a recommendation, not an accepted contract change.
2. **Retain the exact original full claim.** Treat the missing producer as
   separately scoped research, with the specific fact, access, consumer and
   discriminating observation defined before another funding decision. Preserve
   the possibility of other valid mechanisms without making BPF mandatory or
   silently widening observation authority.

The acceptance owner chooses the claim/profile. The current supervisor retains
the durable result and collection responsibility, with no ongoing paid work or
unstarted commitment. The earlier [convergence plan](rust-lifecycle-s10-convergence-plan-20261103.md)
remains the planning record; this result supplies its first actual outcome.
