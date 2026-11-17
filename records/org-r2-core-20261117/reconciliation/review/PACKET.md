# Focused review of the R2 reconciliation (Fable xhigh, read-only, tight budget)

Role: fresh adversarial design reviewer. Skip skill loading beyond `/home/bline/code/work-engine/AGENTS.md` and `docs/contracts/core.md` (budget). Parent: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`, which decides; the operator approves design changes.

Subject: `/home/bline/cbm-work/org-r2-reconciliation-20261117/design/RECONCILIATION.md`. Source: main checkout `/home/bline/code/work-engine` at `9b289171`, read-only. Approved design it reconciles: `/home/bline/cbm-work/org-pi-slices-20261116/s2/ROOT-S2-ADDENDUM.md` and `s2/OWNER-DECISION-S2.json`.

Check only these, spot-checking the cited source lines (do not read whole crates):

1. §1/§0 H1: is it true that the hosted Host + `lifecycle-store` + `CodingContinuity` already own per-effect currentness (transaction held across the send), consumption-before-entry and settlement, so a spawn-boundary host in review-delivery would be a parallel mechanism? (`lifecycle-store/src/pi_coding.rs:165-222`, `pi-pressure-context-host/src/coding.rs:285-343`.)
2. §2.2: is the per-effect org regate inside `with_pi_coding_turn_send` / `with_pi_coding_effect` sound (no lock-ordering, re-entrancy or I/O-in-transaction hazard that makes it unworkable), and is the propagation fence alone correctly judged insufficient?
3. §4 K-c: is the claim that today's grants carry no independent authority correct (`lifecycle-store/src/sqlite.rs:621-628`, `coding-qualification.rs:168-205`)?
4. §5/§6: are S-A, S-B, S-C correctly bounded and sequenced, and are N1–N8 and the stated O-changes the right owner questions? Anything missing or over-built (remember the S1 lesson: root-authored machinery whose only consumer is its own gate)?

Severity P1/P2/P3 with evidence. Write `/home/bline/cbm-work/org-r2-reconciliation-20261117/review/REVIEW.md` (first line: `sound_for_owner_decision`, `revise_before_owner`, or `reject`). If writing fails, put it in your final message. Final message: verdict + path.

Constraints: read-only except the output; never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, configuration contents, `~/.codex`, other `~/.claude` sessions; no builds/tests/git mutation. **Hard budget $4.5**: about 15 targeted reads, then write.
