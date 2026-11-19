# Research lifecycle: live H2 result (2026-11-19)

Root: research-lifecycle, Pi session 01a1e56d (operator-directed takeover of Codex 01a1ccc5). Status: results
verified by the root (delegated Fable verifier) and **accepted by the operator** on 2026-11-19. This record promotes
no claim; any change to the H2 wording goes through a new assessment/qualification decision (lc7#feedback).

Evidence root (outside the repository, kept as written): `/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/`
(below: `O/`). Package: `O/021-s2-r3/` (index `planning/research-lifecycle-h2-execution-packet-draft-r3-20261118.md`);
live runs: `O/027-h2-live/`; verification: `O/029-h2-verification/VERIFICATION.md`
(sha256 daa01130b4a3d49f1a9853a30f221972d138be61a9d2a17d9883a424ff7d0eca); acceptance:
`O/029-h2-verification/OPERATOR-ACCEPTANCE-20261119.md` (sha256 249ab49b669dce8db8653461b9bf43111f83a5db387400094b702fb21d60d618).

## What ran
LC-7 comparison (estimand accepted by the operator): usual policy context (baseline) versus the same plus the qualified
A4 research basis (treatment), one fresh A5 episode each, same audit objective, plus one advisory episode (option A:
the model reads request + card; the task host verifies the reconstruction out of band). Model claude-sdk
claude-haiku-5-5, adaptive thinking low; Pi Host in Docker (`node@sha256:4b150b48…`), Host binary `b97c2eef…`;
three tools, no shell; max_context_bytes 500,000; Claude subscription quota.

| Run | Source / harness | Arm sha256 | Outcome |
| --- | --- | --- | --- |
| advisory, episode 2 | f6dc724e | 5c94c88b… | passed; 5 planned A4 reads + publish; delivery check `complete` (card 034b25f9…, request c0add36d…, reconstruction 245e2c2d… host-verified); **verified** 13/13 |
| audit_baseline | f6dc724e | 6c39f122… | passed; 5 planned reads + publish; **verified** 23/23 |
| audit_treatment, rerun | 9610a8dc (= f6dc724e + read cap 16 -> 32) | 43db4aca… | passed; 18 planned reads + publish; **verified with errors** 30/33 ("15 members" for 14; a line reference 100-101 for 80-82; one insertion point in probe text) |

Retained outcomes (not excluded): attempt 0 (runner setup error, no episode); advisory episode 1 (member ids with `/`
refused by the task owner's id grammar after the read committed; fixed by renaming ids `/` -> `:`, operator re-approved);
first treatment episode (lifecycle-store refused the 17th snapshot read in one invocation; fixed by raising the cap to 32,
merged as 7f43bc19). Diagnoses: `O/027-h2-live/DIAGNOSIS-advisory.md`, `O/027-h2-live/DIAGNOSIS-treatment.md`.

## Findings (descriptive; one episode per arm)
Both audit arms: the band floor 235,000 is supported as basis (below the lowest no-limit Astra compaction 239,130, n=10;
one post-limit Sol session at 248,987, n=1); the trigger 200,000 has no measured basis; "Astra … compact a little under
250K with the limit in force" is unsupported (no post-limit Astra observation); both propose scope-only wording
corrections with values unchanged. The treatment additionally found, from its extra evidence: the n=73 / 17-thread
discrepancy, the unverified file-mtime basis of the config time, the chosen (not measured) 35,000 margin, the reuse of
the M22 116K value (relation not_established), and the register/request owner mismatch. The baseline alone flagged
some HEAD-text items both arms could read (model variance). No effect size or direction follows from this pair.

## Comparison validity
Closed identity comparison clean (all 21 request bodies equal on compared keys, tools digest = pin, system digest equal,
grants differ only in deadline), with four qualifications: the baseline question names evidence it cannot read
(accepted limitation 3(a)); harness differs by the read-cap commit only; the treatment rerun's Host serve record lacks a
receipt (`oversize input`: 10.8 MB RPC transcript over the receipt builder bound; fields present in pi-state.json;
reported to dev-vertical, board 5980); n=1 per arm.

## Cost
Catalog-rate proxy of live episodes (subscription quota consumed): advisory 53,093 µUSD; baseline 9,742; treatment rerun
184,462; first treatment 76,853; advisory episode 1 850; total about $0.325, excluding verification and engineering.

## Code merged for this result (rescue-detached-20260911, not pushed)
f6dc724e (option A delivery check; MAX_ARTIFACT_BYTES 16 MiB; fixture cap 32 MiB; MAX_FACT_BYTES 32 KiB; reviews 024/025);
7f43bc19 (lifecycle-store MAX_SNAPSHOT_READS_PER_INVOCATION 32; review 028). Follow-up (not done): lifecycle-service
snapshot validation should refuse member ids outside the task owner's id grammar at startup.
