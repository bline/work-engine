# Re-evaluation: subject-v2 (`1e6f1077…`)

**Verdict: `acceptable_as_proposed`.** No blocking finding remains. The verdict covers the proposed contract and the unreleased guidance only; the packet stays `formed` with `implementation_authorized=false`. The root keeps acceptance authority. Fable's recommendations were weighed as evidence, not as authority.

## Dispositions (all 11 `verified_resolved`)

| ID | Resolution in v2 | Fable qualification |
|---|---|---|
| 1 (P1) | `proposal.md:76-77` adds the reason; `:90-96` says a restriction ordered before actuation, or one that may apply, leaves re-actuation unestablished, and recompiling it as open is not authority; pilot `:102-103` | upheld |
| 2 (P1) | `proposal.md:79-81` binds the human-usable input paths, and testing only the mediated path cannot prove wider coverage; pilot `:37-42` | accepted: v1 already covered all human input, so the gap was realization and pilot evidence (P2 would have sufficed) |
| 3 (P1) | `context-boundary.md:30-32`: the reference sets up no host contract for automatic resumption and authorizes no improvised controller; recovery still goes through Wind Walker | accepted: `:25-29` is a limitation, and "reads as permission" overstated it |
| 4 (P2) | `proposal.md:68-72`: a role may request; the host keeps readiness and admission; effects dispatched outside the host operation leave the claims unestablished | accepted (correction below) |
| 5 (P2) | `proposal.md:98-105`: the wakeup has its own ordered admission identity, and the delta from the protocol is declared | — |
| 6 (P2) | packet reopening condition 0 and `proposal.md:51-56` now agree | accepted: it was an ambiguity, not a contradiction (correction below) |
| 7 (P2) | `proposal.md:107-117`: author and deliverer recorded separately; attribution ≠ compliance; the pilot wakeup now adversarially asserts approval (`pilot.md:60-64`) | accepted in part: the material defect was the pilot case |
| 8 (P2) | `proposal.md:149-153`: a route for failure outcomes; pilot `:113, :117` | upheld |
| 9 (P2) | `pilot.md:74-118`: a finite set of action channels; evidence sources E1–E5; host records are classed as actor records | accepted as pilot-only |
| 10 (P3) | `proposal.md:35-40`: reuse of the transition protocol is now an obligation on the realization; `pilot.md:41-42` | upheld |
| 11 (P3) | `proposal.md:168-175`: neighbors named, no ownership taken | — |

**Fable's ambiguous-scope finding (SSC-FABLE-1):** resolved. `proposal.md:95-96` says ambiguous scope leaves the affected actuation or admission unestablished rather than narrowing the pause. Pilot `:103` adds the case, and unrelated authorized work does not have to halt.

**Corrections to my v1 findings:**
- **SSC-OPUS-4.** I claimed the text contradicted the owner's architecture, in which the service, not the role, decides. That treated a *proposed* view's invariant (`design/architecture/context-lifecycle.md:143`) as binding and gave too little weight to the manager's invariant 9. As a result, the placement half was overstated; only the custody split was needed. Reading the view's status block together with invariant 9 would have caught this.
- **SSC-OPUS-6.** I read the ambiguous word "permit" in its stricter, affirmative-grant sense, against DESIGN.md §11 and the adjacent "no new approval" sentence. A P3 wording fix would have been enough. Reading v1 `:47-51` as one unit would have caught this.

## Remaining, non-blocking
- **Optional wording.** At `:91`, name the human as the owner of a human-issued restriction's scope. At `:85`, say the input's "authority effect" participates, matching `:100-104`.
- **Reconciliation note.** If v2 is accepted, `:68-70` departs from invariant 1 of the proposed design view. Reconciling that belongs to the view's owner and changes nothing about this candidate's authority.
- **No owner decision is required.** It remains a fact that no supporting host contract currently exists for any root environment. No pilot or realization has run.

## State
- Remediation subject recorded at `fea97b9d5d67ad25a496751b9dbd0cb38d2936cd`.
- Re-evaluation `opus-v2-result` recorded at **`94687646864fe4d00ce7d1f5d124c7ebb5490159`**, predecessor `fea97b9d…`.
- Read-back confirms 11 `verified_resolved`, status `active`, `pending_next_action: await_remediation`.
- Two earlier attempts were rejected because finding evidence and observation text are immutable; the state did not change. The final record therefore keeps each finding's v1 text and evidence verbatim. The v2 dispositions are carried only in `status` and in `remediation_references`, whose freshness rules give the exact resolving line ranges. The causal reasons are in this report.

## Specialist applicability and limits
**agent-instruction-review: applicable.** The changed root guidance now carries the establishment-status statement at its loading boundary. The new contract clauses carry their reasons inline, for example at `:76-77`.

**Limits:**
- Read-only review of the delta, the remediation note, the Fable report and the changed v2 ranges.
- No tests, no commands, no Fable follow-up.
- Fable reviewed v1 only, so it does not attest v2.
