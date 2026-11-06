# Rust development loop: runtime entry and one live proof

Recorded 2026-11-06 UTC. **Proposed budget; implementation/live admission pending.**
Human direction is “proceed” after the accepted [source prerequisite](../records/rust-review-admission-20261106/README.md). The previous objective and its deadlines are closed. This proposal supplies a new bounded allowance; it does not reopen or transfer the old balance.

## Readiness and correction

Baseline is `d14bedc4c6987d5906cd014e4bfcfc3c5c6258cf`, including reviewed source `8144a70878c9187cf3b04b7b2aee0f947d41a852`.

The previous next-step description omitted an executable integration: `work-engine-review-host/src/main.rs:23` initializes with `TrustedStartup::load`, and `src/worker.rs:43` uses that loader in serving. The new `load_for_job` requires an independently derived admission pin. The process paths inspected still use the historical loader; installing the new policy alone does not wire those paths.

The policy `/etc/work-engine/review-host/job-policy.json` is absent. Its parent is root-owned0755. The existing deployment registry selects a historical exact admission and must remain separate. Caller UID1000 cannot perform unattended installation: `sudo -n -l` requires interactive authentication. Docker's server answered with version29.7.2. Current provider/account eligibility and a complete successful job startup remain unestablished. No credential file, provider execution or protected write was used for this readiness check.

Evidence and coverage are retained in [readiness](../records/rust-loop-runtime-entry-readiness-20261106/README.md). This is a targeted source/host finding, not an exhaustive repository audit.

## One bounded outcome

Complete the existing Rust review-host process integration, prepare an exact operator installation packet, and attempt one supervised disposable job through coding, genuine qualified review, explicit operator acceptance and local publication. The actual Rust owners must persist/read back the accepted result; a cached success flag or fabricated review cannot clear `owner_pending`.

The runtime integration must obtain fresh admission from the fixed protected policy and use the resulting capability at both initialization and serving/reopen. Private requests/descriptors remain data. They cannot choose policy paths, grants, approval flags or a deserialized pin. Current policy, exact candidate/artifact/session/attempt binding, writer confinement, one-shot entry and uncertain effects retain their existing owners and contracts. Preserve historical and fixture routes, including fixture provider refusal. Keep new first-party implementation in the existing Rust owners. No new provider backend, generic grant store, supervisor agent, Pi/Docker redesign or context-lifecycle redesign is included.

Before live admission, qualify a complete positive provider-free startup/serve/inspect/reopen case through the actual executable and meaningful authority-negative cases. The prior startup guard control deliberately stopped at missing runtime artifacts; it cannot substitute for this proof. If completing startup requires a broader authority redesign, stop at the included feasibility gate with the precise dependency.

Separate source review must accept the exact implementation. Then prepare the concrete policy/configuration bytes, executable/catalog/credential metadata bindings, paths, expiry, expected hashes and a confined installation/readback command. Human budget approval does not approve unseen configuration bytes. The operator must perform the privileged step interactively after reviewing that packet; do not request passwords, change sudo policy or grant broad privileges to an inference process.

Live entry additionally requires successful installed-policy readback, current tool/isolation/account headroom and enough allowance for genuine review and closure. Existing qualified Claude product review is distinct from Astra review of this implementation. Actual product acceptance and publication require their exact inspection/offer decisions through the owning operator channel. If those decisions or installation remain pending, persist a held outcome within the cap.

At most one new live coding job is funded. Preserve operation identity during readback/reconciliation; uncertain effects do not authorize replay or a replacement job. Keep all work disposable and local; remote publication, production activation, unattended execution and priority-three identity/egress qualification remain outside this supervised proof.

## Budget and stopping rules

Propose **$35 maximum / three hours total work allowance**, targeting **$20–25**. This includes current preparation once, every actor, failed attempts, product inference, reviews, repairs, accounting and closure. These are API-equivalent accounting dollars, not billing or subscription quota.

| Planning allocation | USD |
| --- | ---: |
| Root preparation, coordination and integration | 4 |
| Existing-host wiring and focused qualification | 8 |
| Separate implementation authority review | 5 |
| One live coding job and genuine product review | 10 |
| Repairs and closure | 5 |
| Contingency | 3 |
| **Hard total** | **35** |

Allocations are forecasts within the hard total. Reserve outstanding review, live and closing costs before further admission. Do not sum nested objective totals or treat missing usage as zero.

The current readiness preparation has a self-imposed **$3/20-minute ceiling**, including a $0.75 pre-initialization hold. That cost belongs inside35 if approved. It authorizes no delegated or product inference and no protected write. Reserve20 minutes of the proposed three-hour allowance for this preparation; the funded run must end at most160 elapsed minutes after approval, with absolute times bound at admission. Later operator waiting does not stop that clock.

The included first checkpoint is **$5/30 total work minutes**, counting readiness. Confirm the exact runnable owner path, complete fixture inputs and operator installation route before committing the live reservation. A broader missing contract or unavailable route returns a precise held result; it does not fund repeated feasibility probing.

Stop new feature work at **$15/90 total work minutes**, whichever comes first, preserving $20 and 90 minutes for source review, the admitted proof, repairs, collection and closure. Hard stop is35 dollars/180 total work minutes, including the reserved preparation. There is no automatic extension or replacement live job.

## Supervision and closure

Keep one direct builder's useful context when a current scoped check supports retention. Launch the source reviewer separately with independently observed read-only realization. Root reads bounded results and deterministic custody/accounting; existing chatboard wait delivers context and budget pressure. Do not build another bookkeeping service or infer savings from one run.

Keep implementation completion, source acceptance/integration, live execution, operator acceptance and publication separate. Collect every actor/process/listener and release claims at closure. Preserve baseline fixture failures without broad cleanup. Commit accepted owned source and dated evidence locally with destination-HEAD guards. A held or failed live outcome remains such even when source work is accepted.
