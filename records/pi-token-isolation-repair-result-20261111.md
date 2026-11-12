# Pi token-isolation repair — November 11, 2026

Status: bounded repair accepted in the working tree after retained separate
review. No user-visible commit, skill release or push was made by this repair.

The released host at `ab2a0f84` allowed the credential proxy's implementation
and import directory to overlap a model-writable checkout. A tool could plant
`zstandard.py`, which a subsequent host-side compressed request would import.
Keeping the real token outside the container did not protect mutable host code.

The repair rejects writable mounts overlapping the active host implementation,
Python interpreter/import roots, command-search directories and resolved host
executables. Lexical launcher paths and resolved paths protect symlink parents.
Launch, retained environment re-planning and the common container profile check
this boundary, including implicit writable state/socket mounts. The proxy runner
uses Python isolated mode (`-I`), excluding script/current-directory,
`PYTHONPATH` and user-site imports. An independently installed host outside an
editable checkout remains usable; read-only code mounts remain available.

Validation passed: **147 tests** (51 container, 48 subscription, 27 Anthropic,
21 host), plus `git diff --check`. Container tests used real local Docker/Pi,
fake API credentials and local mock endpoints. The poisoning regression used
fresh actual proxy processes, compressed requests and synthetic subscription
credentials; it simulated module planting between turns. Mount regressions
covered source-checkout refusal, independent installation, ancestors,
descendants, symlink aliases, import/PATH roots, executable targets, launcher
symlink parents, implicit state, refusal before launch effects and legacy
continuation. Custody records, input/output digests and candidate bytes were
verified; all processes were collected without recorded leftovers.

The retained Astra reviewer accepted the original P1 as resolved with no new
P1/P2/P3 within this boundary. Observed runtime: Astra/high/default/read-only,
session `01a1ce4e-efcf-7292-a98c-9da252170266`, remediation turn
`01a1ce60-8fc8-77a1-9611-929a7a7c88d0`. This retains the original independent
audit episode; it is not a new fresh-independence claim.

Exact accepted implementation subject:

- Candidate: `3811b26106faef54a2e76a6047bf7de5b0ff723a`
- Tree: `65ec258a615476e7f81308a5492a35ad6c887a33`
- Patch: `f41d79b17dcf76c8a52af399fa7084a2718e07535762d3fc734eebfe98d4ab8c`
- Gate: `ba74cc59b8adcd5aa5faaa4c46f9ed32bfdf790fe933af301a47a366f5f0457d`
- Plan/scope: `protected-host-v1` / `pi-token-boundary-repair-v1`

Evidence and acceptance:
`/home/bline/cbm-work/pi-token-isolation-repair-20261111/acceptance.json`,
`REVIEW.md`, `review-grant.json`, `checkpoint-receipt.json` and `gate-v2/`.
Initial gate-v1 named the wrong host test file and ran zero host tests; that
failure is preserved and superseded by gate-v2. Earlier root messages and the
review packet said 145 tests / 46 subscription; the final gate logs establish
147 / 48, as independently checked by the reviewer.

No real credentials or provider calls were used. This is not an actual Docker
credential-exfiltration test or live subscription/quota qualification.
Arbitrary pre-existing operator hard-link/bind-mount aliases remain unqualified.
Original R-2 integration/distribution holds are unchanged. The installed release
still needs a separate commit/release of this accepted repair.
