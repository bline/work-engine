Root: H1 remediation 1. Fable review `/home/bline/cbm-work/org-hosted-review-20261117/h1/review/REVIEW.md` (verdict changes_required; read it fully). Implement in the same worktree and crate only (same constraints as your packet; no other crate):

- P2-1: secret rules `v2` per (a)–(e), one positive test per added alternative (and keep the typed-declaration false-positive guard only where it does not hide a literal secret).
- P2-2: checker takes the run's inputs/job artifacts and adds failing checks `snapshot_served`, `subject_bound_to_scope`, `job_id_bound`, `limits_bound` exactly as described (cite the analyze.py lines you rely on); grounding counted against the served manifest.
- P2-3: delivery requires each body's `model` = REQUESTED_MODEL and tool names = HOSTED_TOOLS.
- P3-1..P3-9 all (P3-5: narrow the sentence or make it true; P3-8: add recorded mutants at least for context-fit bound, prompt bound, tree mismatch, subject equality, and one new secret rule; more if cheap).

Evidence: custody runs into `h1/remediation-1/runs/` (same plan shape as main-1, hashes extended) and mutants into `h1/remediation-1/mutants/` (exact restore, hashes). Output `h1/remediation-1/RESULT.md` (changes mapped to findings, runs, mutants, carried) and append "Remediation 1" to `h1/MATRIX.md`. Budget: target $3.5, cap $4.5 — write RESULT.md before you approach the cap. Final message `map_ready`/`handoff_ready` + path.
