# D1 design packet: owners for the O7 mapping rules and the organization anchor (read-only)

Role: read-only design worker. Read `/home/bline/code/work-engine/AGENTS.md`, `docs/contracts/core.md`, `docs/contracts/system-design.md` (semantic ownership and boundaries), and from `skills/manual-orchestration/SKILL.md` only `references/worker.md` and `references/call-cost.md`. Parent/sole collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Do not edit the repository; write only under `/home/bline/cbm-work/org-hardening-owners-20261117/d1/`.

## Question

Two gaps block production consumption of a compiled review contract and the next step (a read-only review on the dev-vertical hosted path). Both are ownership questions, not code:

1. **O7 — the three unowned mapping rules** emitted as labelled fixture assumptions by `rust/crates/work-engine-review-preparation/src/prepare.rs` (`BlockerKind::ScopeDigestToSubject`, `ProblemScopeToEpisodeIdentity`, `ConsumptionEvidenceMapping`; read their blocker texts and "no owner states …" reasons) and screened as `Discharge::AssumedDischarge` in `rust/crates/work-engine-review-delivery/src/gate.rs`:
   - `scope-digest-to-subject`: how an organization candidate's scope digest maps to the review-episode subject;
   - `problem-scope-to-episode-identity`: how a problem scope maps to the episode identity (run/slice/attempt/plan version/obligation/episode ids);
   - `consumption-evidence-mapping:<condition>`: how a condition's boundary and consumer participant map to an evidence admission's boundary and consumer.
2. **Organization anchor** — the organization id in `FixtureAuthority` / `OperationRequest` is caller-asserted and unanchored; Separation (O8) and ObserverIdentity (R3) are discharged relative to it. Who owns the record that anchors an organization id (issues it, binds it to principals/installations, revokes it), and how the gate would derive it rather than accept it.

## Inputs

- Records (committed): `records/org-pi-slices-20261116/s2/design/PROPOSAL.md` §4.3 and `ROOT-S2-ADDENDUM.md` (O7–O10 table), `s2/review/REVIEW.md`; `records/org-r2-core-20261117/reconciliation/design/RECONCILIATION.md` (§3, the binding table, N7, owners list); `records/org-r2-core-20261117/sb/design-review/REVIEW.md` §4 (gate-input inventory) and §8; `records/org-r2-core-20261117/sb/SB-ACCEPTANCE.json` (`carried_open`).
- Code (read targeted ranges): `work-engine-organization` (candidate, scope digest), `work-engine-organization-authority` (S-A: principals, vantage-actor bindings, grants, decision scope, `OperationRequest`, observation source), `review-episode-core` (episode identity, subject, evidence admissions), `work-engine-review-preparation`, `work-engine-review-delivery`, and any existing crate or service that already owns subject identity, episode identity, evidence admissions, or organization/principal records (search: lifecycle-store, slice-campaign, authority-store, review-execution-evidence, work-engine-review-host). Use `docs/repository-layout.md` and `DESIGN.md` routing if needed.

## Output

`/home/bline/cbm-work/org-hardening-owners-20261117/d1/PROPOSAL.md`, at most ~250 lines:

- For each of the four items: the semantic question in one sentence; candidate owners found in the existing system (crate/service/contract, with file:line evidence for what it already owns); recommended owner and why (system-design ownership: who already holds the facts the rule needs, who can be held accountable for changing it); what the owner must publish (record or function signature in prose, inputs classified host-derived / digest-bound / labelled assertion); how the gate would consume it so the corresponding discharge becomes real instead of assumed; the smallest next slice that would establish it (size class S/M/L) and what it unblocks.
- Distinguish facts (cited) from proposals and `not established`.
- A short "operator decisions" list: each a yes/no or pick-one question the operator can answer, with your recommendation. Flag any item where the honest answer is "no existing owner; a new owner/record must be created" or "the rule is ill-posed as stated".
- Dependencies and order among the four; which ones the hosted read-only review (step 3) actually needs versus production consumption.

Final message `map_ready` + path.

## Constraints

Read-only. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions or protected configuration. No network, Docker, cargo builds, live providers, or mutating git. Targeted reads (rg first, then ranges). Budget: target $4, cap $5.5; if near the cap, write what you have with the remaining items marked unfinished.
