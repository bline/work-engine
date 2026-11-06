# Production custody and the remaining development loop

Proposal prepared 2026-11-06 against `827a119d11eba29cd3c34b06cad0e59df395bed8`.
The approved $3/30-minute pass funds this scoping result only. Implementation,
installation, task acceptance and publication remain unapproved.

## Findings that change the next move

The last proof stopped at a real production refusal: `Workspace::publish` rejects
Production because repository custody is not installed. The completion executable
also has fixture startup only. Removing either guard would misrepresent the result.

Existing W0 owners already implement exact checkpoint eligibility, authenticated
operator decisions, prepared publication, one-shot consumption, guarded Git CAS,
durable publication settlement and uncertain-effect recovery. Reuse them. The missing
custody is an enforced process/filesystem boundary and its checked installation,
not another publication ledger or an agent supervisor.

There is also a separate final consumer gap. `Jobs::read_result` currently projects
the original Pi Host receipt, including `owner_pending`. W0 publication settles a
checkpoint; it does not accept the whole job. A checked job-outcome consumer must
join the original job/export provenance to explicit task acceptance and settled
publication. Preserve the original execution receipt and delivery acknowledgement.
Neither can be rewritten into semantic acceptance.

Evidence, source hashes, coverage limitations and the exact consumer lookup are in
the [dated scoping record](../records/rust-production-custody-scope-20261106/README.md).

## A: implement and qualify repository custody first

**Finish line:** one provider-free production-mode publication in a disposable
managed repository, with independently enforced denials, durable owner settlement
and a successful cold reopen. This is a publication prerequisite, not a completed
coding job or unattended release.

`workspace-publication` remains responsible for repository custody, topology, its
permanent lock, existing operation ledger and physical Git mutation. Bind an
installed capability to the actual repository/common-directory filesystem identity,
permitted refs, owner root, validator/sandbox selection and current installation
epoch. A caller-supplied path/hash or saved JSON cannot construct authority. Preserve
existing admission/CAS/unknown-effect semantics; replacement or revocation cannot
authorize a second physical attempt. Prefer a fresh production profile and refuse
unsupported migration.

Candidate validation currently executes as the publisher with unrestricted host
filesystem access. Run it with independently enforced confinement: candidate input
read-only, separate writable build scratch, no canonical Git, lock, owner records,
operator socket or credential access. Pin helper selection outside candidate control.

The smallest deployment candidate keeps the trusted deterministic host under the
existing operator identity, with inference and validation independently confined.
W0 requires the operator channel and state to be outside the agent's granted
authority; it does **not** require a new runtime UID distinct from the operator.
A same-UID unrestricted agent is unsupported. Namespace/container confinement must
demonstrate the required denials, including process, inherited descriptor and mount
escape routes. A dedicated runtime UID remains an alternative, with explicit owner
root and native credential provisioning costs. Do not add that migration by default.
If the smaller boundary cannot be demonstrated, stop and price the alternative.

Use a disposable dedicated canonical repository for qualification. Choosing the
real release repository/ref and exact installation is a later operator decision.
Bare Git alone is not custody; unmanaged linked worktrees and ABA writers matter.

Anticipated source boundary: `workspace-publication` custody/contract/store/lock/Git/
validation modules, focused process and publication tests, necessary manifests,
and an inert installation/readback packet. Confirm the exact file manifest before
mutation. No new database, backend, dispatcher, provider integration or context
lifecycle redesign.

Required qualification includes hostile validator and agent attempts to write refs,
create linked worktrees, replace the permanent lock and modify ledger/policy; wrong
repository identity/ref/epoch/helper/peer; replaced policy and stale parent; and
crash/reopen after admission, attempt and CAS without a second mutation. Fixture
tests remain useful but cannot establish an OS deployment boundary.

## B and C: explicit remaining dependencies

**B: production completion startup.** Compose the actual qualified campaign,
claim, episode, execution-evidence and workspace owners through the existing
completion application. Prefer initial-review and completion modes in the same
installed executable: the native episode owner pins `current_exe`, so a different
completion binary is not interchangeable. Preserve one-writer/close-before-reopen
rules and private authority pins. Add a checked workflow composition operation only
where required; do not move publication semantics into review-workflow. Qualify
same-binary reopen before any provider run. Source boundary is completion-host,
review-host startup/CLI and the minimal review-workflow composition plus manifests.

**C: job-level settlement.** First accept a small explicit consuming contract at
the server job owner. It must bind original job/request/input identity, immutable
Host receipt and exports, exact reviewed checkpoint, semantic task acceptance and
the authoritative settled publication. Cold reads expose that job outcome alongside
the unchanged execution receipt. Wrong provenance, absent acceptance, uncertain
publication and changed replay must refuse terminal success. This is not permission
to clear historical receipts or widen W0's publication-only decision.

Only after A, B and C are qualified should one new bounded coding task proceed
through genuine independent review, explicit task/checkpoint acceptance, separate
publication approval and cold-read terminal settlement. Current provider/account/
model eligibility and exact candidate-bound installation remain preconditions.
Do not fund another live attempt before these routes exist.

## Recommended next budget: A only

Target **$12–15**; hard cap **$18 / 4 hours**, all actors and setup included.
Confidence is medium-low until the real confinement gate passes.

| Work | Allowance |
| --- | ---: |
| Root coordination, feasibility and accounting | $2 |
| One retained builder | $5 |
| Independent source review | $4 |
| Real provider-free qualification | $2 |
| Repairs and closure | $3 |
| Contingency | $2 |
| Total hard cap | **$18** |

The included **$2 / 30-minute opening gate** establishes available isolation tools,
the chosen process boundary and actual denial/readback feasibility before broad
implementation. It is inside the allowances, not another funded objective. Failure
ends A held with evidence and a revised proposal. Stop new feature work by 2 hours
15 minutes or $10 total, whichever comes first, preserving the remaining time and
money for review, qualification, repairs and closure. No automatic extensions;
operator delays still count against the clock. Any protected installation requires
a concrete reviewed packet and exact operator approval before execution.

The current $35 aggregate has at most **$8.78** conservative headroom after booking
this scoping pass at its full $3. A therefore needs a new approval raising the
aggregate ceiling to **$45**: conservative prior booking $26.2202516 plus $18 is
$44.2202516. These are speed-adjusted API-equivalent accounting dollars.

B is provisionally target $12–15, cap $18 / 3–4 hours for startup and checkpoint
publication qualification; its earlier forecast cannot promise job terminalization.
C has only a low-confidence incremental $6–10 forecast, not a firm funding request.
Reprice B/C together after A and the job-acceptance contract are settled, avoiding
duplicate setup and another premature live-run promise. No combined cap or release
date is asserted from these incomplete forecasts.

Approval of this proposal would fund A only. Continuity and the broader concurrency,
credential expiry, budget exhaustion, identity and inference-egress work remain on
the roadmap; they are not silently folded into this publication prerequisite.
