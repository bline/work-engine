# SC3: retained review after builder changes

Status: implementation recommendation only. J2/H1 originated at committed
`c4952af0542763de365ead11ba69d70a1347d831`; this revision binds the
accepted R1/R2 fix commit `7b39d6905d7d2c120b182daec2b92914a5e728f1`.
No implementation, test execution, provider invocation, activation, or state
disposition is authorized by this
document. DESIGN.md governs; PHILOSOPHY.md supplies rationale. This is the
bounded successor to [HP2 SC3](rust-campaign-hp2-plan.md#later-scope-and-state-disposition)
and [J2](rust-j2-completion-implementation-plan.md), coordinated with the separate
[real-provider plan](rust-review-provider-execution-plan.md).

The historical review reported `J2H1-R1` (derived native identifier preflight)
and `J2H1-R2` (aggregate reliance readback), archived at
`/home/bline/code/.work-engine-tmp/j2-h1-review-20261013/review-report.json`.
The committed repairs passed the combined 79-test gate (zero failures, one
existing ignored), and retained review verified both findings resolved with
no new material issue. Successor validation preserves both checks without
reopening their disposition absent a changed premise.

## Recommendation

Implement one serial retained-remediation round for a changed immutable builder
candidate, with a repeatable round contract. Keep the campaign, selected
obligation, original episode authority and actual reviewer session; give the
round its own candidate, accepted production-claim selection, execution attempt,
subject/result transitions and completion receipts. Publish the next exact
finding revisions only after campaign, episode, execution-evidence and CE owners
join. Repeating that protocol is allowed only after the prior round settles.

Split delivery into a domain join and a provider/host integration. The domain
join can be implemented and tested without S8/S9 or a live provider. A joined
production proof needs the provider lane's actual retained-session custody,
qualified evidence scope and enforced capability boundary. Current controlled
E1 evidence cannot substitute for those facts.

J2 already implements initial result publication and builder evaluation of an
exact finding revision. A builder's valid/invalid evaluation is neither a code
change nor reviewer verification of remediation. SC3 supplies that missing
review round. It does not supply result correction/rejection, reviewer
replacement, general claim succession, campaign acceptance or terminalization.

## Existing semantics to preserve

| Evidence at the baseline | Decision for SC3 |
| --- | --- |
| `slice-campaign/src/initial_outcome.rs::check_initial` rejects a later episode revision and a different consumer tree. | Keep initial APIs initial-only. Add a generation-aware join; do not weaken that check globally. Historical initial receipts remain available through explicit historical reads. |
| `review-episode-core/src/reducer.rs::record_remediation_subject` accepts active remediation/reported episodes, changes the current subject, and preserves current result and evidence admissions. | Record the replacement subject through the episode owner. The authority's `initialSubject` remains original. A `same_session` reducer value is not evidence of actual provider continuity. |
| `record_result` rejects stale required admissions by producing `evidence_unestablished` when previous admissions exist and fresh admissions are absent. | Derive and supply fresh checked admissions for the replacement subject. Do not erase old admissions or reuse the initial pair. |
| `implementation_review_v1.rs::preserve_finding_lineage` requires every prior finding, preserving its attribution and immutable substance; status/remediation evidence may evolve. | Send the complete prior result and finding lineage to the retained reviewer; use the same validation on the next result. New findings may be added. |
| Legacy `service.mjs::runNativeRemediation` admits awaiting-builder or reported zero-finding closures. It does not require every finding to have a builder evaluation first. | Initially preserve that campaign eligibility, with no pending evaluation operation when the round is frozen. Do not silently broaden admission to all reported nonempty results. |
| Legacy `review-finding-bridge.mjs::publishFindings` revises existing stable finding claims, retains initial revision references, and does not carry reliance onto changed revisions. | Preserve claim IDs and historical evaluations; derive new revision references and require new reliance/evaluation for the new revision and consumer tree. |
| Legacy recovery infers provider entry from subject/result transitions and can allow entry again. | Do not port this inference. Campaign dispatch plus execution-owner custody decides uncertainty; absence of an episode transition never grants another entry. |
| Legacy remediation repairs a historical wrong `authority.initialSubject`. | Do not introduce this compatibility path in fresh Rust roots. Reject conflicting authority. |

The relevant legacy files are under
`app-server/src/services/slice-campaign/{service,native-review-closure,native-review-host}.mjs`
and `app-server/src/services/claim-evidence/review-finding-bridge.mjs`. They are
semantic evidence, not new runtime dependencies. Native UTF-16 identities remain
lossless across subjects, findings, transitions, receipts and command digests.

## Contract checkpoint before implementation

The following are proposed owner decisions, not accepted contracts merely
because they appear here. Freeze them with the implementation authorization:

1. **Campaign format:** use a fresh-root successor profile/schema (proposed
   schema 4). Schema 3 is explicitly initial-only. No migration/import of an
   existing schema-3 campaign is included. A schema-4 campaign must run its
   initial review in that format before it can remediate; schema-3 behavior and
   historical receipts remain supported under their existing profile. RP2's
   bounded real initial admission uses this fresh schema-4 root; RP2 does not
   authorize a remediation entry.
2. **Replacement subject:** keep an immutable initial candidate plus immutable
   round records and a current-generation pointer. Require a changed candidate
   tree bound to the same campaign workspace, original baseline and exact gate
   evidence through the SC0 subject verifier. The replacement selection must
   explicitly accept that tree and gate evidence; DTO fields cannot do so.
3. **Selection authority:** the campaign/claims acceptance owner installs an
   exact successor selection for each round, including the complete production
   claim pair and retained-continuity profile. Preserve obligation, reviewer
   identity and acceptance authority. Do not derive authority for the new tree
   by copying the initial selection or changing `fresh_initial` to `retained`.
4. **Eligibility:** retain the legacy awaiting-builder / reported-empty entry
   rule above; require the current generation settled, no pending builder
   evaluation, and an episode phase that admits remediation. An
   `EvidenceUnestablished` generation requires a separately accepted evidence
   correction path; SC3 does not route around it.
5. **Session handoff:** the provider owner issues a private, non-cloneable,
   non-deserializable `RetainedReviewSession` after checked actual session
   response and protected retained-store custody. Its serializable locator is
   descriptive. Resume requires this capability and a fresh `Applied` campaign
   permit. The provider's initial CBM graph is bound to the initial candidate;
   the replacement candidate receives a new private index generation before
   SC3-H reviewer entry, with the same read-only isolation/coverage contract.

Campaign, episode and CE owners jointly accept items 1–4; the provider owner
joins item 5 and the execution profile. Missing or contradictory authority is a
bounded refusal, not a host interpretation. These are the exact open decisions
needed to authorize the proposed implementation; broader deferred features are
not prerequisites.

The recommended tradeoffs are explicit: fresh schema-4 roots avoid migration
but require a new initial review before remediation; immutable changed-tree and
gate binding adds preparation evidence but prevents old results from silently
covering a new candidate; a newly accepted claim pair/profile keeps authority
with its owner instead of reusing a stale initial grant; settled-generation
eligibility preserves the awaiting-builder and empty-result routes while
excluding unresolved evidence and pending evaluations; the private session
capability plus fresh permit adds protected custody but prevents a locator or
replay from becoming provider-entry authority. None is accepted solely by this
recommendation.

## Owner records and public handoff

The campaign owns a typed `RemediationRound` and immutable stage intents. A
round records the original initial-outcome locator; predecessor generation;
candidate and gate references; full accepted claim pair/profile/selection;
retained-session reference; prior episode/result revision; complete prior
finding revision references; and all operation, attempt, subject-transition and
result-transition IDs. It also records each downstream command digest, returned
receipt and exact owner read target. Round IDs are campaign generation IDs;
execution attempt IDs are distinct, never reused initial attempt IDs.

The initial episode reference and authority generation remain stable. The
current subject and result advance through episode transitions. CE finding IDs
remain episode/finding identities, while finding revisions and candidate-bound
production claim IDs change as their owners require. Preserve the original
initial finding revision and every historical builder evaluation. A current
pointer must never make an old result appear to review the replacement tree.

Proposed API surface, with Rust names to freeze at the checkpoint:

| Campaign operation | Required inputs and result |
| --- | --- |
| `prepare_remediation` | Expected campaign revision, caller operation/round IDs, predecessor outcome locator, and privately constructed `RemediationSelection` installed by trusted composition. Verifies subject/selection/session references and freezes the complete request. Returns a descriptive prepared locator. |
| `authorize_remediation_dispatch` | Exact prepared locator and frozen dispatch command, including episode authority, subject/result transition IDs and unique execution attempt. Only `Applied` returns `RemediationDispatchPermit`; replay returns the original dispatch receipt without a permit. |
| `prepare_remediation_completion` | Original dispatch locator plus checked execution-owner reference. Registers the round's immutable CE observation/establishment/admission commands and consumption intentions before issuing those effects. Returns exact pending owner stages and locators. |
| `register_remediation_episode_read` / `complete_remediation` | Bind the exact subject/result transition read targets and finish the CE/ER1 join using owner readers. Return pending, refused, evidence-unestablished or joined progress with original identities. They do not publish a usable builder projection merely because stores joined. |
| `finalize_remediation` | Checked joined round plus exact builder selection. Publishes/revises the complete finding projection (or explicit empty witness), finalizes campaign outcome and consumes only the builder-projection commitment when usable. |
| `locate_remediation` / `recover_remediation` | Stable campaign/obligation/round/operation identities; return original preparation, dispatch, stages and outcome references. Recovery can finish already registered owner writes/readbacks, but returns no execution permit. |

All names in this table are proposed additions. Do not reuse an initial locator
with altered fields. Preparation, dispatch, completion registration and outcome
locators need discovery before their acknowledgements are received; a host
must not reconstruct hidden profile digests or read owner tables. Exact request
replay rejects changed content even when the caller reuses an operation ID.

`RemediationSelection` construction checks installed owner capabilities and
exact immutable artifacts; deserializing a request never constructs it. The
host composes owners and exposes progress. It does not keep a separate cached
semantic campaign state or decide whether a review, claim or session is valid.

Schema-3 `request_slot_one_active` allows only one active request per obligation,
and J2 retains the initial slot. In the proposed successor format,
`prepare_remediation` must atomically retire eligibility of the settled prior
slot and insert the next slot in the same campaign transaction. Preserve the
prior consumed row, operation receipts and `may_have_entered` forever; neither
retirement nor recovery reissues its permit. The new generation-aware reader
validates this predecessor chain. Do not delete the old row or independently
toggle slots from the host.

## One round and its partial commits

1. Freeze the replacement candidate, gates, accepted selection, complete prior
   result/findings and session binding. Validate representable payload sizes,
   identities and complete request bytes before effects. The builder may
   describe changes; only the retained reviewer resolves its findings.
2. Persist dispatch before possible provider entry. Through the native episode
   owner, record the exact remediation subject and read its first introduction
   back through ER1. Enter the provider only with the fresh permit, checked
   subject cut and actual retained-session capability. Provider retention does
   not authorize campaign advancement by itself.
3. The execution owner records reservation, actual entry/session observations,
   terminal evidence or unresolved custody, binding candidate, selection,
   profile, request digest, episode, attempt and dispatch receipt. Requested
   session/model/capabilities remain distinct from observed facts. A missing
   session, lost custody or interrupted uncertain entry never triggers an
   automatic fresh reviewer or retry.
4. Reconcile new production observations and establishments through CE, with
   distinct builder-projection and campaign-terminalization consumers. Record
   the next episode result using freshly checked admissions and original
   authority. Read the exact result introduction through ER1. Cross-store
   writes are individually durable; their join is not a transaction across
   campaign, execution, episode and CE stores.
5. Validate full finding lineage and publish each next finding revision using
   its exact previous revision as CE's CAS predecessor. New findings use CE
   creation. Recover each completed publication by its saved owner command and
   receipt; do not invent replacement operations after a lost acknowledgement.
   Exact projection requires 1–100 findings, or the campaign-owned empty
   witness for zero. An oversized result is an explicit refusal before
   projection effects, not a truncated success.
6. Finalize the round with the same J2 outcome distinctions. Established usable
   findings await builder disposition; usable empty findings are reported;
   incomplete verdicts, limitations and unresolved questions remain visible.
   Established false and unestablished evidence stay distinct in their claim
   pair even when the campaign outcome is `EvidenceUnestablished`. A failed
   round retains historical usable prior results, clearly bound to the prior
   tree, and cannot present them as the replacement result. Only a usable
   builder projection consumes its commitment. Terminalization remains
   untouched. `Reported` is not acceptance.

Before dispatch, recovery reports prepared state and leaves a separately
authorized fresh dispatch action to the owner protocol. After dispatch,
recovery never issues another permit, even if no subject transition, provider
output or result is visible. A stopped process with a lost permit remains
unresolved; solving that with a new attempt or replacement reviewer is outside
this slice. Conflicting/missing historical owner evidence also stays unresolved.

Here, no-repeat means no additional host-authorized provider entry/prompt.
It does not prove one upstream inference or disable a provider CLI's internal
transport retries. The qualified provider contract must state its retry facts;
a stronger remote at-most-once claim needs its own evidence.

Current CE production-path validation allows only a null establishment
predecessor. SC3 therefore uses freshly accepted candidate-bound production
claims and establishments, with the predecessor relationship owned by the
campaign round. This does not claim general CE supersession. The terminalization
claim is checked for the pair but its consumption is still deferred.

## Delivery and file ownership

**SC3-D — domain join.** After the contract checkpoint, implement the campaign
round machine and exact owner integration against native stores. Exercise real
episode and CE APIs with bounded fixtures and explicitly qualified test
evidence. A test fixture cannot certify production isolation/session custody.
Complete recovery and failure behavior in this slice; avoid a prepare-only API
that leaves the usable result undefined. Existing J2 evaluation stays available
before the initial generation advances; its historical evaluations remain
readable afterward. Current-generation evaluation uses the new revision and
tree, with no inherited current reliance/disposition. Do not reopen evaluation
of an old tree through the initial API after remediation advances the episode.

**SC3-H — retained execution integration.** After SC3-D and the provider lane's
qualified initial execution/session-custody slice, compose actual resume with
the frozen SC3 permit/request and a new candidate-bound CBM index, then run the
joined crash-cut proof. A live provider proof needs separate authorization.
The provider plan's initial
real-provider work may retain a session without enabling remediation entry;
SC3-H is that later entry integration.

| Owner | Expected files / responsibility |
| --- | --- |
| Campaign | `rust/crates/slice-campaign/src/{application,contract,completion,store,lib}.rs`, new `remediation.rs`, fresh schema-4 initializer and focused `tests/remediation*.rs`. Owns round state, selection admission, slots, commands, locators, finalization and generation-aware builder evaluation. Keep initial-outcome semantics intact; share only proven common helpers. |
| Episode | `review-episode` native admission/application/read integration and `review-episode-core/tests/transitions.rs`. Prefer existing `record_remediation_subject`, `record_result`, native admission and ER1 APIs. Add only demonstrated missing typed admission/read contracts; do not rewrite reducer semantics to make a test pass. |
| CE | Existing `claim-evidence::{application,findings,production_path}` APIs and focused owner tests. Owns stable IDs, revision CAS, read/admission and consumption. No duplicate campaign implementation of CE validation; request an owner API if a concrete gap appears. |
| Provider / execution evidence | Separate provider plan owns retained session custody, protected resume, candidate-bound private CBM index/daemon evidence, qualified E1 writer/reader and actual observed evidence. The current `QualifiedProvider` enum alone is not an implemented qualified scope. |
| Workflow / host | `review-workflow` remediation composition and `work-engine-review-host` bounded command/progress/recovery integration in SC3-H. Trusted startup installs the accepted successor profile; an initial-profile DTO cannot opt into it. No Node compatibility primitive or host semantic owner. |
| Joined integration owner | Shared Cargo edits, serial downstream gates and exact publication candidate after each owner freezes its files. |

Freeze the provider plan's narrowly required real-profile campaign admission
and SC3's successor format together before either edits `TrustedConfig`.
SC3-D owns the domain format and can qualify it without a provider. The RP2
initial call intended for SC3 continuity starts only on a fresh accepted
schema-4 root with the exact initial selection/claims/subject and provider
profile. RP2 remains initial-only; this coordination does not enable
remediation there. SC3-H alone joins a later `Applied` permit, checked private
retained session and replacement-candidate index to actual resume.

No dependency on lifecycle S8/S9 completion is imposed on SC3-D. The presently
planned provider route is direct Anthropic Claude Code, not the Codex lifecycle
runtime. If the selected actual provider route later depends on lifecycle
activation or legacy state transfer, the joined proof must name and satisfy
that dependency; domain tests cannot establish it.

## Required validation for implementation acceptance

- Initial then replacement flow with native episode/CE stores: zero findings,
  retained finding plus new finding, incomplete result with limitations/questions,
  valid/invalid builder disposition, and verified-resolved status. Assert exact
  original subject/authority and current candidate at every owner boundary.
- UTF-16 identity vectors; dropped/rewritten prior finding attribution; wrong
  candidate/gate/selection/profile/session; wrong episode/result introduction;
  stale or missing fresh admissions; false versus unestablished claim results.
- New finding revision CAS and unchanged stable claim identity, historical
  initial revision/evaluations, no new-tree reliance inherited from the prior
  tree, distinct consumer commitments and no terminalization consumption.
  Reconcile every exact reliance admission used by a returned aggregate, not
  only the most recently recovered evaluation; corrupted/missing older
  admissions must refuse. Preflight all derived native identifiers and complete
  envelopes before preparation effects.
- Failure cuts around preparation/slot retirement, dispatch, subject write,
  provider reservation/entry/terminal, each CE stage, result write/read,
  publication and final campaign acknowledgement. Recovery must return original
  IDs, finish only registered effects, and never increase the provider entry
  count. Test missing session custody and permit-loss stops explicitly.
- Complete 100-finding result and refusal of 101; bounded request/history reads
  without silently omitting prior findings; conflicting replay; backpressure
  before effects; schema-3 initial regression and schema/profile mismatch.
- SC3-H: prove an actual initial call and one actual retained resume on the
  same observed session, with exact new subject/request/evidence bindings and
  enforced capabilities, including distinct immutable candidate CBM generations
  and no cross-project or shared-daemon access. Kill the composed host at each
  effect boundary and
  show recovery does not invoke an additional provider call. Separately label
  deterministic fixture evidence and live-provider evidence.

Correction of malformed results, rejection contracts, evidence correction,
authority/session replacement, retry after uncertain entry, broad claim or
selection succession, specialist expansion, terminalization, completion commit,
workspace publication, old-root migration and runtime activation remain separate
successors. None is smuggled into a DTO flag or generic stage payload here.

## Planning evidence

Tier 2 graph verification used project `home-bline-code-work-engine`, root this
repository, generation `2026-10-13T04:35:20Z`; watched Git state was rechecked at
`04:48:31Z`. Relevant symbol search pages were consumed, with exact reducer
snippets and bounded inbound/outbound traces for remediation. Injected legacy
calls are incompletely represented by graph edges, so exact source supplied
the operational ordering. A regex/filter search that returned no useful rows
was replaced with narrower global queries; it is not evidence of absence.

Coverage checked all 27 relied-on paths. The campaign SQL initializer had
partial parse ranges 10, 15, 19, 27 and 41; its entire source was read, including
the active-slot unique index. Other checked paths had no recorded gap. This is
bounded, best-effort evidence, not an exhaustive repository audit. Source hashes
and the path inventory are retained outside the shared tree at
`/home/bline/code/.work-engine-tmp/sc3-planning-20261013/`.
The final coverage refresh used generation `2026-10-13T04:51:15Z`, recorded at
`04:52:11Z`; all 27 source hashes still matched the named commit.

No tests or provider operations were run for this planning task. Acceptance of
this plan would authorize only the subsequently specified implementation scope;
it would not authorize live execution, migration, disposition or activation.
The revision also used main graph generation `2026-10-13T06:06:14Z` / store
`ue03742e60f24fcbeg2`; current plan, provider plan, adapter, E1 contract and
startup paths were content verified with no recorded gaps. The present main
graph is planning evidence only, not a reviewer candidate index. Revision
notes are under `/home/bline/code/.work-engine-tmp/rp-sc3-cbm-plan-20261013/`.
