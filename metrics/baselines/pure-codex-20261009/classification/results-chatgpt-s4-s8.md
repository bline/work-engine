# Independent classification results — S4–S8 (ChatGPT, 2026-10-09)

Produced by ChatGPT (web, higher reasoning setting) from
[packet-s4-s8.md](packet-s4-s8.md) only, without the proposing analysis's labels.
Recorded as returned; condensed tables preserve every item, bucket, dependency
and confidence. The classifier's own interpretive rule: a prior review finding
does not by itself become a settled fact; the underlying fact must already exist
in an accepted/published slice, accepted plan/contract, or committed code/schema.

## S4 — integrated service

| item | bucket | basis | dependent-on | confidence |
|---|---|---|---|---|
| S4C1-F1 | arguable | S1 exact-conflict semantics plus S4 coherent continuation imply it, but no explicit "newest entered attempt governs" rule | — | med |
| S4C1-F2 | novel | SIGTERM ordering versus an already-active verifier; not settled by S4 | — | high |
| S4C1-F3 | arguable | replay-before-capacity precedence not explicitly stated | — | med |
| S4C1-F4 | clear | published S3 carries remaining caller budgets through waits/reconnect | — | high |
| Candidate 2 | undetermined | mixed causes from C1 findings | S4C1-F1–F4 | med |
| Candidate 3 | arguable | incomplete repair of F2 | S4C1-F2 | med |

## S5 — native provider and bounded operation

| item | bucket | basis | dependent-on | confidence |
|---|---|---|---|---|
| S5C-F1 | clear | accepted entry/commit-bound authority guarantee | — | high |
| S5C-F2 | arguable | generation-switch/reconciliation composition not fully specified | — | med |
| S5C-F3 | clear | accepted operation-specific outcome/settlement evidence | — | high |
| S5C-F4 | clear | published S2 subprocess supervision and exact child accounting | — | high |
| S5C-F5 | clear | accepted raw-ingress identity preservation | — | high |
| S5C-F6 | novel | installed route behavior (`functions.exec` denial, async question) | — | high |
| S5C-F7 | arguable | versioned reader/acquisition interface not fully stated | — | med |
| Candidate 2 | undetermined | mixed C1 causes | S5C-F1–F7 | high |
| Candidate 3 | clear | F1/F3 residuals (with F7) | S5C-F1/F3/F7 | high |
| Candidate 4 | clear | F1 residual (subject-clock rollback) | S5C-F1 | high |
| Candidate 5 | clear | F1 residual (discarded trusted clock update) | S5C-F1 | high |

Candidates 3–5 are downstream churn from the F1 chain, not independent misses.

## S6/S7 — semantic preservation and recovery/admin

| item | bucket | basis | dependent-on | confidence |
|---|---|---|---|---|
| S6-C1-F1 | clear | accepted S6 fencing; freeze v7 made `semantic_required` durable | — | high |
| S6-C1-F2 | clear | accepted answer-free challenge (A1/A10) | — | high |
| S7-C1-F1 | clear | accepted inactive destination adoption | — | high |
| S7-C1-F2 | arguable | one freshness revision covering all custody mutations is derived, not frozen | — | med |
| S7-C1-F3 | clear | accepted fenced source handoff | — | high |
| S7-C1-F4 | arguable | fresh-coordinate contradiction fencing not explicitly stated | — | med |
| S7-C2-F5 | novel | adoption/readback lock contention and indeterminate readback | — | high |
| AIR-C1-F1 | clear | committed consumer enum/schema contradicted by prompt | — | high |
| AIR-C1-F2 | arguable | source-mapping projection not previously frozen | — | med |
| Freeze v2 | arguable | method-name alignment found in co-builder coordination | — | med |
| Freeze v3 | arguable | selector/provenance fields, launch-guard proof | — | med |
| Freeze v4 | arguable | shared contract/Yielded semantics after co-builder findings | — | low |
| Freeze v5 | arguable | third verifier-session role to avoid reusing an exited child | — | med |
| Freeze v6 | arguable | exposure identity, staleness, revocation proof machinery | — | low |
| Freeze v7 | clear | accepted no-bypass semantic/currentness boundary | — | high |
| Freeze v8 | clear | accepted S6 A2/A7/A8 negative and currentness behavior | — | high |
| Freeze v9 | novel | staged-before-publish crash cut, joined validation | — | high |
| Freeze v10 | clear | published S3 strict protocol-v1/schema compatibility | — | high |
| Freeze v11 | undetermined | bundles AIR, S6 and S7 C1 repairs | AIR-C1-F1/F2; S6-C1-F1/F2; S7-C1-F1/F2/F3 | high |
| Freeze v12 | undetermined | bundles S7-C1-F4 with S7-C2-F5 | S7-C1-F4; S7-C2-F5 | high |
| Freeze v13 | arguable | another F5 cleanup/readback hole | S7-C2-F5 | med |
| Freeze v14 | arguable | corrects v13's overbroad wording | Freeze v13 | high |
| Candidate 2 | undetermined | mixed C1 causes | Candidate-1 findings | high |
| Candidate 3 | undetermined | arguable F4 plus novel F5 | S7-C1-F4; S7-C2-F5 | high |
| Candidate 4 | arguable | further F5 residual | S7-C2-F5 | med |

## S8 — retained workflow

| item | bucket | basis | dependent-on | confidence |
|---|---|---|---|---|
| Admission v2 | undetermined | packet contained only the opening, identical to v1 (packet defect) | — | high |
| Admission v3 | clear | committed S1 schema: `effect_id UNIQUE`, one active effect/input | — | high |
| Admission v4 | novel | provider history omits tool items | — | high |
| Admission v5 | arguable | two-cycle authority map/grant selection | — | med |
| Admission v6 | clear | committed S4 synchronous single-loop actor | — | high |
| Admission v7 | novel | killed incarnation's native session cannot be reopened | — | high |
| Admission v8 | arguable | v6's grant lane could commit after service close | Admission v6 | high |
| Admission v9 | novel | idle runner retained a sender | — | high |
| Admission v10 | novel | new T22 spawn-custody proof coverage | — | high |
| Admission v11 | novel | zombie versus live child | Admission v10 | high |

## Classifier summary (as returned)

Raw counts over 52 items (S4 1/3/1/1, S5 7/2/1/1, S6/S7 8/11/2/4, S8 2/2/5/1 as
clear/arguable/novel/undetermined) are **not a contradiction rate**: findings,
later candidates and corrective revisions are causally dependent.

The hypothesis survives independent classification in a narrower form: a
recurring class of churn reopens a specific dimension that earlier accepted
code or contracts had closed (S4 caller budgets; S5 authority, raw ingress and
process custody; S6 semantic fence and answer-free challenge; S7 inactive
adoption and ownership; S8 S1 schema and S4 actor). Productive discovery is also
clearly real and should not be suppressed. The large arguable bucket, mostly
S6/S7 freeze churn, supports better contract formation and selective projection
but does not establish a specific missed fact in each case.

Metric cautions raised: candidate-level classification is coarse when a
candidate addresses findings of several classes; multi-change revisions (v11,
v12) share that problem; and a broad accepted consequence such as "recovery must
be safe" must not count as the settled fact behind every recovery defect.
Recommended operative question: **did this failed premise reopen a specific
dimension that an applicable authoritative artifact had already closed before
the attempt began?**
