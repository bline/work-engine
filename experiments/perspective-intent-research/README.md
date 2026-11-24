# Perspective and intent research: evidence

Evidence for the [perspective and intent pilot](../../design/ideas/pending/pilots/perspective-intent-pilot.md) and its [research note](../../design/ideas/pending/perspective-intent-research-note.md). The documents there own the research design. This directory holds captured evidence only.

This capture authorizes no research runs, inference spending, deployment or gate changes.

## Stage 0 (`stage0/`)

Captured 2026-11-24T03:42:31Z by [`stage0/capture.py`](stage0/capture.py) (`python3 -I experiments/perspective-intent-research/stage0/capture.py` from the repository root; no network, no model calls). [`stage0/MANIFEST.json`](stage0/MANIFEST.json) records every digest. The script fails without writing a manifest if any pinned digest differs.

### Case F, first for P7 (`stage0/case-f/`)

- **`packet/`:** byte-identical copies of eleven files from `~/cbm-work/pi-host-u4a-acceptance-20261121/`, keeping their relative paths:
  - contract 043 and plan 041;
  - `PLAN.md`;
  - both escalation briefs and both Astra architecture reviews;
  - the qualification amendment;
  - the operator decision request;
  - both auto-remove documents.

  Ten were pinned before capture (pilot §3, the plan reviews, or a digest cited inside `OPERATOR-DECISION.md`). `ARCHITECTURE-REESCALATION.md` was first pinned at this capture. The manifest records each file's prior pin.
- **`board-messages.json`:** board messages #6173 (operator direction relayed to RL), #6176 (RL alignment), #6184 and #6185 (ordinary local Docker scope), exported whole from the coordination board.
- **`session-excerpt.json`:** 77 entries from Pi session `01a1e2a4-f353-743a-ba23-f42394958125` within four cited windows (UTC):
  - 08:11–08:14, the overnight delegation;
  - 08:59, the repair dispatch;
  - 09:43, the parked message to the operator containing "The H1–H5 requirements are unchanged";
  - 15:07–15:38, the operator's correction.

  **Projection:** user and assistant messages only; text blocks and tool-call names and arguments; thinking omitted.

  **Verification:** each entry carries its 1-based `source_line` and the SHA-256 of the raw source line, newline included. The plan reviews cited these lines one lower (for example 9714 for the 09:43:22Z message, which is line 9715 here).

### Milestone A cases A–E (`stage0/milestone-a/`)

- **`CHECKIN-snapshot.md`:** a point-in-time copy of `~/cbm-work/milestone-a-coordinator-20261121/CHECKIN.md` (SHA-256 `8f912490…`). The original is a living newest-first log that the coordinator is still writing. Every entry the note and pilot cite (#6411, #6495, #6511, #6534, #6544, #6548, #6553, #6569, #6628, #6648, #6657, #6692) was present at capture.

### Inventories, not copies

| Source | Pin |
|---|---|
| Case F session `01a1e2a4…` (46 MB; unchanged since 2026-11-22) | full SHA-256 `2f65fbb1…` |
| Milestone A coordinator session `01a1ff25…` (61 MB; still being written) | SHA-256 of its first 13,087 lines, `b419567a…` |
| Case D source `rust/crates/work-engine-manual-host/src/pi_public_hold.rs` at `3693e48c` | git blob `1d868446…` (in this repository) |

The transcripts are not copied: they are large and contain private reasoning and prompts (pilot §9.1).

### Gaps still open for Stage 0

- **Case D:** Sol's plan-only revision proposing the bounded observation hold is not yet located.
- **Cases A–E:** board messages and design packets are not yet exported. The CHECKIN snapshot gives the chronology; the per-case cut-time packets (pilot Stage 0) remain to be built.
- **Case B transfer packet for P7:** not yet assembled.
- **Ordinary negative cases:** none captured yet. They are needed so the corpus is not only memorable successes (pilot §9.3).
