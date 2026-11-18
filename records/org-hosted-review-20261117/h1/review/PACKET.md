# H1 code review packet (read-only, Fable)

Role: read-only adversarial code reviewer (fresh context). Read `/home/bline/code/work-engine/docs/contracts/core.md` and `docs/contracts/evidence-and-claims.md` §3.9; from `skills/manual-orchestration/SKILL.md` load only `references/worker.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Write only `/home/bline/cbm-work/org-hosted-review-20261117/h1/review/REVIEW.md`. **Write the review file before you approach your cap** (a previous reviewer lost its verdict at the cap); update it if you continue.

Subject: worktree `/home/bline/cbm-work/wt-org-hosted-review-20261117` (uncommitted vs `700f45e3`: new crate `rust/crates/work-engine-review-hosted`, `rust/Cargo.toml` +1, `rust/Cargo.lock` +16). Root verified the files equal `h1/runs/main-1/hashes.stdout`. Contract: `h1/PACKET.md` (outcomes 1–6, binding operator decisions quoted there); design `d3/DESIGN.md` and its Fable review `d3/review/REVIEW.md`. Builder report `h1/RESULT.md`, `h1/MATRIX.md`; custody `h1/runs/main-1` (9/9), `h1/runs/mutants-1` (9 killed). Paths under `/home/bline/cbm-work/org-hosted-review-20261117/`.

Priorities (this crate gates a live provider run on real repository content):
1. Operator conditions: requirements revision exactly as authorized (only the two realization-class constraints removed; new run id; truthful proposition; frozen fixture untouched); subject identity (commit→tree, recomputed patchIdentity, first parent, scope-digest binding); size enforcement against the real Host bounds (snapshot, 400 000-byte context, 32 768 prompt incl. Host suffix); secret/private-material rules (closed, named, applied to every member, no content echoed) — are there obvious gaps (e.g. `.env`-style files, SSH/known key formats, tokens in diffs, home-dir paths in other forms)?
2. KnownIncompatibility refusal and the realization claim constant vs what the Host enforces (cite).
3. Checker: strict parse, subject equality, contract recompute from retained inputs, delivery prefix against the retained request body and its digest, grounding, envelope fields (`owner_pending`, `consumable:false`, ...). Can any path emit a bare review or claim more than CLAIMS.md allows?
4. Mutants faithful; tests exercise real refusals (not tautologies); synthetic repos only.
5. Overclaims in docs/CLAIMS.md/DERIVATION.md; scope creep; anything that would make the later `analyze.py` run unsafe.

Output `REVIEW.md` (≤ 100 lines): verdict `accept_private_readiness` | `changes_required`; findings P1–P3 with file:line and concrete fixes; not established. Final message: verdict + path.

Constraints: read-only; no builds, no mutating git, no network. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, `~/.pi`, credentials, `~/.codex`, other `~/.claude` sessions. Targeted reads. Budget: target $3.5, cap $5.
