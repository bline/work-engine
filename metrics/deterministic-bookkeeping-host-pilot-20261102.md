# Deterministic bookkeeping host: bounded pilot, 2026-11-02

The fresh-child path worked after repair. This pilot did **not** establish the
proposed 50% bookkeeping-turn reduction or lower operational cost. The
[machine-readable observation](deterministic-bookkeeping-host-pilot-20261102.json)
contains selected normalized calls, exact source locations, pricing identity and
limitations. It uses the repository usage ledger, not a reconstruction of the
user's unavailable historical study scripts.

| Observation | Manual normal task | Hosted normal task |
| --- | ---: | ---: |
| Root lifecycle calls through accepted closure | 5 | 5 |
| Child calls | 1 | 1 |
| Root API-equivalent spend, speed-adjusted | $1.238968 | $1.348320 |
| Root plus child spend | $1.268571 | $1.377917 |
| First command to accepted closure, wall time | 100.55 s | 118.17 s |
| Root call-latency proxy | 140.85 s | 156.26 s |
| Isolated active model time | Not established | Not established |

Calls include authorization setup, review collection and semantic acceptance;
they are not an independently classified count of bookkeeping-only turns.
The first call's latency begins before the first command, so the latency sum can
exceed the reported command-to-closure wall interval. Neither quantity is a
measurement of isolated active model time. Both children used observed
Sol6.1/low/default and returned the same tiny JSON contract. Root context and
candidate revision differed, and the comparison was unblinded.

The manual arm authored one custom JavaScript capture/hash/write operation
(root rollout line1101). The hosted normal arm delegated that capture to the
pinned host; the root still authored authority and closure records. Neither
normal arm wrote a new standalone Python program. Setup, fault qualification,
integration and reporting outside those arms still used ad hoc programs, and all
related model cost remains in the objective ledger.

Hosted completion at 19:21:14.595Z reached a collected receipt in the root tool
result at 19:21:27.399Z: 12.804 seconds. The foreground executor first yielded to
the root, causing an extra root wait call. Subsequent replay, receipt
acknowledgment, accounting refresh and failure cases are excluded from the normal
pair and included in the whole objective cost. This is one descriptive pair,
not a steady-state benchmark or an estimate for an engineering campaign.

The initial hosted attempt at source275c0d5c failed before creating a child.
It incorrectly treated a context heuristic as a capacity veto and omitted the
external reviewer from a supposedly complete accounting total. Both were fixed
at a0cda34e and reviewed before resuming with fresh state. The original protocol
was interrupted; the revised comparison is not represented as an unchanged
preregistered trial. The diagnosis-to-resumption interval contains 30 observed
calls costing $5.9066224 across actors. That interval also includes review,
waiting and integration preparation; it is not isolated repair effort and must
not be added again to the objective total.

Fresh hosted work established exact-turn settings, filesystem read-only
realization, bindings, native result capture, collection, acceptance and delivered
receipts. A retained turn produced its result but lacked selected-turn settings
attestation, so the root abandoned it. A deliberately missing result stayed
`result_missing` and was abandoned. No failed mechanical check became semantic
success. Replay evidence covers the recorded observation interval. Later native
history reads were unavailable; the corresponding JSON stdout files are empty,
and the errors are in the root rollout at line1358.

Native crash/race/cancellation and rollback were not exercised by those paid
probes; deterministic fixtures cover their stated properties. Filesystem
read-only evidence establishes no wider tool-isolation or review-independence
claim. The original 22.4/52.8-hour ratio remains an unverified historical estimate.
Default adoption needs representative, comparable operational measurements and
authoritative settings evidence before configuration-qualified retained use.
