# Rust-owned development loop: scope and budget

Recorded 2026-11-06 UTC against shared HEAD
`d824a35f9defd33eb05e6a1adbf1fb1b735b89a3`.
Status: user-directed priority and planning; **new implementation budget proposed**.
The preceding pilot objectives are closed. Their unused allowance and execution
authority do not carry into this objective.

## Observable result

One supervised local job progresses through admission, actual Pi/Docker coding,
independent review, explicit acceptance and local Git publication. Existing Rust
owners persist the decisions and effects. A supported job read after a server
restart reconstructs the same accepted-and-published outcome and exact evidence
references without a supervising model completing bookkeeping.

This is completion of one job under a named local profile. It does not declare
the campaign complete or qualify unattended execution. The original Host receipt
remains immutable, including its historical `owner_pending` custody meaning.
The aggregate job acceptance comes from the actual acceptance owner; it must
resolve for a fully accepted job rather than merely repeat the Host placeholder.
Review, acceptance, publication and external/domain settlement remain distinct.

## Existing machinery and the important gaps

| Existing owner or composition | Reuse and boundary |
| --- | --- |
| `work-engine-server/src/job.rs` | Admission, operation identity, collection and result readback. Its current result projects acceptance directly from the Host receipt. It does not establish end-to-end job acceptance. |
| `work-engine-manual-host/src/pi_docker.rs` | Execution custody and immutable runtime receipts; delivery acknowledgement still reports `owner_pending`. Do not turn acknowledgement into acceptance. |
| `review-workflow`, `review-episode`, execution-evidence and `slice-campaign` | Existing review execution, evidence and checked review outcome machinery. Bind the actual supported independent-review profile before implementation; a recorded verdict alone does not establish required independence or confinement. |
| `slice-campaign::publication` | Checked review readback, exact inspection package, operator acceptance, publication eligibility, offer decisions, consumption and result references. Acceptance here is publication-only. |
| `workspace-publication` | Attributed delta, isolated validation, exact prepared commit, guarded expected-old ref update, durable result and reconciliation. |
| `work-engine-completion-host` | Existing application route composing those owners. The installed executable route inspected here is explicitly an offline fixture host. Production operator custody is not supplied by that fixture. |

Source observations include `Application::execute`,
`CampaignPublication::{inspect_candidate,accept_checkpoint_from_peer,record_result}`,
the publication wire/state contract and the server's `read_result`.
The [W0 contract](../records/rust-workspace-w0-contract.md) and
[W1 result](../records/rust-workspace-w1-result.md) preserve the operator/agent
boundary, fixture standing and standalone episode-reader restrictions.
The [accepted local pilot](../records/local-development-pilot-20261106/README.md)
establishes useful coding but leaves Rust acceptance pending.

These observations identify candidate composition boundaries, not a claim that
the complete join already exists. In particular, do not substitute controlled
fixture review evidence for an actual independent review, weaken an episode
executable/read pin, or activate production publication through fixture mode.

## First checkpoint: a cheap go/no-go

Within **$6 or 30 minutes**, including all bound actors, establish:

1. The exact accepted source baseline and affected paths, after reconciling peer
   claims and foreign changes. Use an isolated candidate checkout.
2. The existing owner of the aggregate job outcome and a bounded composition
   between that owner, coding, review, campaign acceptance and workspace
   publication. Store owner references and recovery identities; do not introduce
   a second canonical workflow, review, approval or publication ledger.
3. An admissible independent-review execution/evidence profile for this actual
   candidate, including its account, model, isolation and reservation. Establish
   that genuine review can reach checked owner readback without fixture promotion.
4. The operator channel and exact decisions permitted by the selected supervised
   local profile. Fixture authority remains labeled. Production activation stays
   refused without the required custody installation.
5. A reproducible provider-free test path and one credible live proof within the
   remaining money and time. Check canonical budget freshness, deadlines, tool
   availability, container route and provider/account headroom before entry.

If a supported review/authority route or owner join requires a broader contract
change, stop at this checkpoint with the precise missing boundary and a rebid.
Do not spend the remainder repeatedly probing readiness or silently reduce the
goal to a synthetic successful review.

## Implementation and qualification

Add the smallest Rust composition needed to connect the selected owners. Bind
the original admission/request, job and execution receipt to the exact candidate
commit/tree, validator evidence, independent review outcome, acceptance decision
and publication operation. Trusted startup selects roots, profiles and grants;
request JSON cannot install authority. The top-level coordinator remains
deterministic; models supply coding and review judgment within their contracts.

Use existing server/application boundaries for a versioned job readback.
Preserve old wire/receipt semantics. Accepted-but-unpublished, refused, pending,
conflicting and uncertain states stay observable with exact owner references.
An unavailable owner cannot be converted into success from a cached flag.

Record recovery identities before consequential calls. Exact retry and restart
read/reconcile the original operations. A lost response after acceptance or a
Git update cannot trigger a replacement coding job, review or physical ref update.

Qualification must establish:

- Actual Rust owners and processes complete one provider-free composed case,
  with fixture identities and substituted inference explicitly labeled.
- Reopening the server and owner stores preserves the same terminal aggregate
  and evidence references. A duplicate request does not duplicate effects.
- Missing, stale, mismatched or adverse review evidence cannot manufacture
  acceptance; a reported review remains separate from semantic acceptance.
- Withheld acceptance, changed candidate/tree, stale publication parent and
  changed request reuse hold or refuse at the owning boundary.
- Controlled interruption after an acceptance commit and after a physical Git
  update preserves the known/unknown result and original operation identity.
- One separately admitted actual coding job and genuine independent review use
  the qualified path. Explicit operator decisions over the resulting exact
  inspection package and prepared offer precede their respective effects.
- Relevant tests, formatting/lint, separate review of the implementation, actor
  collection, local source integration and an evidence-backed terminal record
  complete within the cap.

Product-result review and review of this implementation are different subjects;
preserve their provenance and required independence separately. A complete
provider-free proof is useful partial work, not the claimed actual live result.

## Budget and stopping rules

Propose **$60 maximum / four elapsed hours**, targeting **$35–45 actual proxy
spend**. This is a build/integration forecast with a live qualification tail;
the target is judgment, not a measured per-type prediction. Include preparation
for this proposal once, all implementation/review actors, product inference,
failed attempts, repairs, integration and closing tails in the new accounting
window. Keep raw usage, API-equivalent proxy cost, reservations, actual billing
and subscription quota distinguishable.

| Allowance | Maximum USD |
| --- | ---: |
| Owner/contract binding and feasibility | 6 |
| Rust composition and focused tests | 12 |
| Product coding inference reservation | 18 |
| Product-result and implementation independent review | 8 |
| Bounded repairs | 6 |
| Integration, local publication, accounting and closure | 6 |
| Contingency | 4 |
| **Total** | **60** |

The $18 coding allowance accommodates the previously recorded **$17.680004**
four-request Sol/high catalog hold; it is not expected billing or permission to
reuse an old grant. Recompute current bindings before admission. Review usage and
its outstanding reservation must fit the separate $8 review allowance and total
cap; if they cannot, hold the live phase and report the mismatch. Replace holds
with established actual accounting without charging both for the same calls.

- First checkpoint: **$6 / 30 minutes**, whichever arrives first.
- Stop new productive work at **$40 / 150 minutes**, whichever arrives first;
  protect **$20 and 90 minutes** for review, repairs, collection and closure.
- Hard stop: **$60 / 240 minutes**, whichever arrives first. Bind absolute times
  at admission, including already incurred preparation. No automatic extension,
  fresh clock on replacement, or additional live attempt after a failed entry.
- At most **one new live coding job**. Same-operation readback/reconciliation is
  permitted; replacement inference requires a new decision within explicit scope.
- No live launch until provider-free qualification, current capacity and full
  outstanding reservations fit. If an exact operator decision remains unanswered,
  preserve it and stop safely within the existing deadline.

Use existing host-side budget observation and pressure; do not build a new
accounting mechanism or make model-authored JSON authoritative. Select any
builder/reviewer roles only at admission under the applicable orchestration
contract. This proposal launches none.

Budget approval funds the work, not acceptance of an unseen candidate or a
future publication offer. Present those exact artifacts for the decisions
required by their owning contract. Local publication uses a fresh managed test
repository/ref for the product proof; shared source integration uses its own
coordinated publication claim. No push or deployment is included.

## Coordination, exclusions and successor work

At preparation, another Root owns the Sol/high live pilot, preparation guards,
`context-pressure/src/timestamp.rs` and shared-branch publication. This objective
does not collect or repair that operation. Do not edit its claimed files or
publish on its branch claim. Rebind accepted peer source at the first checkpoint;
wait or report a dependency if required shared paths remain owned elsewhere.

Likely implementation candidates are existing server job composition/readback,
review/completion application boundaries and focused joined tests. Exact write
ownership is fixed at the first checkpoint before delegation or source mutation.
Changing admission or review/publication semantics requires explicit amendment,
not opportunistic expansion of those paths.

Excluded: continuity redesign, coding-after-reset qualification, broad concurrent
job/fault matrix, unattended Dot activation, connector identity/OAuth changes,
egress deployment, UI, new provider routes, benchmark comparisons, global skill
release, production state migration and historical receipt rewriting. Priorities
2 and 3 remain successors, with required identity/authority checks retained in
this slice rather than postponed past its effects.

## Evidence method and limitations

Repository intent: orientation and targeted trace, Tier 2. Graph project
`home-bline-code-work-engine`, generation `2026-11-06T05:32:10Z`.
Material source was read directly; graph snippets and a one-hop acceptance trace
supplied supporting structure. Targeted campaign publication symbol pagination
was exhausted; broad BM25 discovery was candidate-only and not exhaustive.
Coverage checked the seven material source paths above and the named W0/W1,
pilot, layout and Rust-direction references. Source files were content-verified;
most reported unresolved call edges. Direct source of the material methods and
contracts governs these observations; no complete call-graph or runtime claim
is made. Best-effort clean coverage is not proof of completeness. No builds,
tests, provider calls, implementation edits or deployment establish this proposal.
