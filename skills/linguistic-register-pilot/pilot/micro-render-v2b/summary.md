# Micro-render v2b transport result

Experiment `semantic-licensed-micro-render-v2b-2026-08-25` stopped at its
preregistered transport-completeness gate. Fifteen of sixteen fresh render jobs
were accepted. Semantic adjudication and matching were not run.

The original transport report is superseded for interpretation by
`transport-gate-report-qualified.json` and `review-qualification.json`. The
sample-level stop remains valid, but complete-manifest pre-launch immutability,
sealed-key packet binding, condition-level normalization rates, and exhaustive
renderer filesystem isolation were not established.

## Outcome

- All 16 expected jobs have pre-launch attempt markers, completed model
  receipts, raw-response digests, and event digests.
- 7 responses were accepted as direct strings.
- 8 responses were accepted by unwrapping exactly one approved single-key
  `text` wrapper without changing its value.
- S04 was rejected after one approved unwrap because the extracted prose had 88
  words, below the frozen 90--130-word constraint.
- The failed 16/16 gate sets both downstream states to
  `not_run_transport_gate_failed`.

Normalization mode by candidate or condition is transport-compliance metadata
only. It is not evidence of register recognizability, semantic equivalence, or
candidate quality.

## Causal evidence chain

1. Every launched packet embeds preregistration revision 2 checkpoint
   `558af5ff2f089b30511a4c1c5fb18028b5fe3b52`, and every per-job request is
   bound by its own pre-launch attempt marker. The manually supplied Git author
   timestamps are inaccurate and cannot establish chronology.
2. Checkpoint `922d85a293418b90c275a543d5eea3fb71f5957c` contains the 16-job
   manifest and packets, but its recorded timestamp postdates the earliest
   attempt. Complete-manifest immutability before first launch is therefore not
   established.
3. Each runner wrote its bound attempt marker before spawning Sol and retained
   a completion receipt afterward.
4. Blinded ingestion produced sample-level transport outcomes and preserved raw
   and extracted-prose digests.
5. A combined post-batch checkpoint froze the launch artifacts and all retained
   observations at `1e53322d9bf9708f4b0d821c3236339e2e6d20d6` before transport
   interpretation.

Preregistration revision 1 (`5c14960e676e698f944dd764838f687f8ecd8ed1`)
failed during local packet-path construction before any attempt marker or model
launch. The correction changed path resolution and removed an unreachable
variable; revision 2 records that limitation.

An initial post-batch checkpoint (`b3d6e87cae077601609d0480c09945fa79ad4220`)
contained the observations but not the launch artifacts because the checkpoint
adapter rebuilds each tree from the branch baseline rather than inheriting its
parent tree. No report was produced from it. The combined checkpoint above
explicitly includes both sets before interpretation.

The sealed key's sixteen stored packet digests do not match the relabeled v2b
packet bytes. Condition-level normalization rates from the original report are
withdrawn; the blinded sample-level 15/16 result does not depend on that key.
Receipts also describe only the two staged files, not an exhaustive visibility
boundary: three event streams show reads of a global generic Codex skill file.
No retained event shows a v1/v2 output read.

## Authority

This is stopped transport evidence only. It does not authorize retrying S04,
running semantic adjudication or matching, salvaging v1/v2 outputs, selecting a
candidate, constructing C0/C1/C2 artifacts, or changing `app-server`.
