# Proposed production custody and completion scope

Proposal only. Root and the human retain contract acceptance, implementation budget,
operator installation, semantic checkpoint acceptance and publication decisions.
No implementation, tests, credentials or provider calls were performed in this pass.

The proposed owner split fits the current code. workspace-publication owns canonical
Git mutation, topology/lock/ledger custody, admission and currentness. slice-campaign
publication retains exact candidate inspection, publication-only acceptance, offers,
authenticated decisions and consumption. Completion startup composes actual qualified
owners. Review-workflow may expose one checked composition operation; it must not
export private grant pins or become a repository writer. A generic dispatcher/new
store/backend is not required by the observed gap.

## Required capability and invalid states

Production must establish independently enforced exclusive canonical-repository custody,
not infer it from a JSON hash, same UID, marker, cooperative list or branch snapshot.
The trusted runtime owns the canonical Git common directory, refs/objects/config/hooks,
linked-worktree topology, permanent lock inode, operation artifacts and existing ledger.
Inference agents, coding children and candidate validation cannot mutate any of them.
The human/operator channel is outside the agent's granted authority. Root/operator
administration remains trusted; maintenance invalidates the installed custody epoch.

An installed, non-request-constructible workspace capability should bind: current
protected source/revision/digest and active epoch; trusted runtime/operator identities;
canonical repository/common-dir anchored filesystem identities; workspace/root ID and
ledger/config identity; exact ref allowlist; installed validator/sandbox/tool selections.
Existing candidate/manifest/prepared tree/parent/commit/offer/decision/generation bindings
remain in their current owners, rather than becoming broad authority in this document.
Path-only repository_id currently hashes the common-dir string, so replacement at that
path must be refused through anchored identity and custody checks. Hold the existing
permanent guard and current installed-capability check through the physical CAS. A
revoked/replaced policy refuses new effects; uncertain historical operations retain their
identity and read-only recovery, never restart under a replacement epoch. Fail closed if
that history no longer supports a production observation.

Invalid outcomes include self-selected policy or trusted hash, restored authority from
saved JSON, forged operator actor, same-UID operator impersonation, wrong repository/ref/
root/validator/epoch, protected-path substitution, agent or validator writes to canonical
state, an unregistered checkout/ABA writer, stale offer/parent, second physical attempt
after uncertainty, or upgraded fixture receipts. Claim epoch currentness for the actual
mutation interval; do not claim detection of arbitrary historic ABA from OID equality.

## Reuse and missing structure

Reuse existing canonical codecs, one-owner repository marker, permanent flock, durable
operation/generation records, exact accepted manifest integration, installed validation
receipt, prepared-commit identity, owner decision/consumption checks, in-process gix
expected-old CAS, irreversible no-effect closure and cold reconciliation. Existing tests
exercise delayed holders, crash cuts, second ledger, poisoned Git selection, stale refs
and external actors. They remain fixture evidence. They explicitly demonstrate unmanaged
checkout and A-to-B-to-A custody limits. Do not add a second lease service/database.

Genuinely missing: protected installed custody constructor and live currentness checks;
independent OS enforcement/deployment readback; candidate validator confinement; production
completion startup and its exact operator principal. validation::run currently executes
candidate tests as the publisher identity. env_clear does not remove filesystem authority.
A publisher UID must never be inherited by an unconstrained candidate validator.

A separately built completion binary also cannot read a review-host-created native
episode: the episode owner recomputes current_exe SHA. The smallest contract-preserving
choice is a single installed executable with initial-review and completion modes, so the
creator/reader identity stays exact; preserve one writer/close-before-reopen. A separate
binary needs a new trusted read-only delegation contract and is a larger alternative.
Do not remove or weaken the episode pin. A joint executable can still use separate
process roles and the existing owner libraries.

A distinct trusted runtime UID with all trusted owner roots is one coherent deployment
candidate, with root/operator UID separate from inference/validator UID. This implies
explicit native credential-owner provisioning for that UID; the current credential
owner requires source UID equal to getuid, so old UID1000 eligibility does not transfer.
No credential copy or account availability is assumed. An equivalent confinement design
is permissible only if it demonstrates the same denials, not merely service labels.
A new dedicated protected bare canonical repository is operationally simpler, but the
human must choose whether it is the intended canonical publication destination. A bare
repository alone does not exclude unmanaged writers or topology changes.

## Minimal anticipated write boundary

A: workspace-publication/src/{custody.rs(new),contract.rs,store.rs,lock.rs,git.rs,validation.rs}
and focused publication/custody process tests. Add authority binding to the existing
identity/record scheme only as required; prefer fresh production profile/root and refuse
unsupported migration. Keep existing fixture behavior and unknown-effect rules. Protect
local Git config/hook execution and independently select executable/sandbox pins before
any helper entry. Candidate source is read-only to validation; writable build scratch is
separate. Installation/readback tooling is inert until the operator approves exact bytes.

B: work-engine-completion-host existing lib/client plus production startup composition;
work-engine-review-host CLI and review-workflow startup/checked completion operation if
using the recommended same executable. Dependency manifests and dated contract/results
are included. Keep slice-campaign semantic changes out unless a material consumer mismatch
is discovered. The exact implementation manifest must precede mutation after acceptance.
An additive episode read-delegation alternative would expand into review-episode/store;
it is not included in the preferred minimum.

## Qualification and vertical release proof

A provider-free proof must use a real independently installed policy and separated process
principals in a disposable managed repository: demonstrate agent/validator denial for
ref writes, linked worktree creation, lock replacement and ledger/source modification;
run a hostile validation canary that fails to cross custody; confirm exact integrated
source/validation binding; perform one allowed owner CAS and cold reopen under production
mode. Same-UID/rootless counterparts remain fixture qualification. Test wrong owner/mode/
ancestry/symlink/repository identity/ref/epoch/tool, policy replacement/revocation after
reopen and before CAS, stale parent, wrong candidate/offer/decision and authenticated wrong
peer; preserve crash-after-admission/attempt/CAS and no-second-effect behavior. Ordinary
negative tests are insufficient to claim OS deployment isolation.

B must consume actual job-aware qualified startup and real claim/episode/evidence owners,
inspect the real candidate, obtain explicit authenticated acceptance, prepare and validate
exact publication, display exact offer, obtain separate create decision, consume once,
update only the allowed canonical ref, record both owner results, then cold reopen and
read acceptance, offer/consumption, exact commit/tree/parent/message and publication state.
No replay of old provider receipts or fixture claim substitution. Use one new coding job
and genuine independent product review only after deployment/input gates and reservation.
Current model availability/quota, native account eligibility under the chosen UID, exact
catalog/compiler choices, claims issuance and protected installation remain separate gates.

This proves at most an accepted checkpoint and local publication under W0. W0 explicitly
is not whole-campaign terminalization. Existing rust-development-loop-closure planning
already records that work-engine-server job result projects Host receipt acceptance and
Pi delivery acknowledgement remains owner_pending. The exact end-to-end job-result consumer
join is NOT established by this scoped pass. It must be admitted as a separate bounded
composition/dependency before claiming a fully accepted job. Do not silently clear a
historical delivery receipt or call this prerequisite a finished development loop.

## Product choices, forecast and stop conditions

Human choices: canonical repository/ref destination and allowed baseline policy; acceptable
OS custody/operator channel (including distinct principals and credential provisioning);
actual validation policy; exact installed documents and revocation/maintenance semantics;
exact later acceptance and create offers. Mechanic choices such as module names, codecs
already owned, lock usage and targeted test layout are implementation judgment.

Preliminary medium-low confidence estimates, not authorized spend:

| Slice | Target / recommended cap | Clock | Includes |
| --- | --- | --- | --- |
| A custody + validator + real provider-free qualification | $12-15 / $18 | 3-4 hours | builder5, control/setup2, independent review4, qualification/deployment2, repairs/closure3 within cap |
| B production completion startup + bounded genuine checkpoint proof | $12-15 / $18 | 3-4 hours | builder5, control/operator2, independent review4, coding/product review3, repairs/closure2 within cap |
| Combined shared-context alternative | $24-28 / $32 | 6-7 hours | both source scopes, separate retained review episodes as needed, real isolation and live-proof reserves; no duplicate setup estimate |

Neither A nor combined production work defensibly fits the approximately $9-10 conservative
aggregate headroom Root supplied. New explicit implementation funding and an aggregate
cap increase would be needed; Root must reconcile actual accounting before proposing it.
Estimates depend on available operator-managed principals and existing sandbox support;
account provisioning, source-owner handoff or the final job-result consumer can increase
scope. If those inputs are not settled, propose A as prerequisite only, not another promise
to finish the full loop. At A's early gate require actual denial/readback proof before
spending on broad source. At B's early gate require actual same-binary episode/campaign
reopen, source ownership and job-result consumer composition before coding/provider entry.
Stop on a missing authority contract, unsupported owner reopen, or exhausted review/closure
reserve. No implementation in this scoping pass and no automatic follow-on.
