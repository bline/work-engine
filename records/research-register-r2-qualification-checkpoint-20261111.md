# Research register R-2 (Rust anchors): qualification checkpoint, 2026-11-11

Status: **not integrated; independent acceptance not completed.** This record closes the R-2 phase for now, as the
work moves to designing the Research Knowledge Lifecycle and Decision Workflow
([planning reference](../planning/research-knowledge-lifecycle-and-decision-workflow-v0.1-20261111.md),
[next-phase handoff](../planning/research-to-decision-workflow-contract-handoff-20261111.md)). It records state and
does not decide or authorize anything. Frozen reviewer reports and budget records are cited, not changed.

## Implementation

| Item | Identity |
|---|---|
| Base (shared `rescue-detached-20260911`) | `6f3d6c716d5c499fec7a694bf43b52fb4b2d2548` |
| Original candidate, branch `r2-anchors-20261111` | `c67c51245e6bd9c1e6cf98391bbbb48b02d618fe`, tree `8978848812ed582447443ad67765638c4b4ca95d`; local tag `r2-original-candidate-20261111` |
| F-01 repair candidate, branch `r2-f01-repair-20261111` | `0d021609482ad89841eb94f5c0a156e328efb7a6`, tree `38a00905fc631232a453331272b258571d04a152`, parent c67c5124; local tag `r2-f01-repair-candidate-20261111` |

- The Rust anchor implementation (`rust/crates/research-register/src/anchor.rs`, `src/anchor/diff.rs`, the `anchors`
  subcommand and tests) exists only on those branches. Neither commit is an ancestor of the shared branch.
- The F-01 repair changes two files (`src/anchor.rs`, `tests/anchors.rs`, +80/-11). A revision commit that is missing
  after validation is now `cannot_read` (F-4). A revision that has become ambiguous stays `invariant_failure`. The
  `Prefix::Unique` path is unchanged.
- The repair discriminates against the original defect. The new regression, compiled against the c67c5124 library,
  fails with `invariant_failure` where it expects `cannot_read`, and passes on 0d021609 (executed by the recheck
  reviewer). The unit test separates the Missing and Ambiguous arms. The builder's mutants M1/M2 fail it.
- Previously established anchor behaviour is preserved within the executed coverage. The candidate crate suite
  passes: builder 88 passed / 1 opt-in ignored; recheck reviewer 87 / 1 ignored with the Myers oracle filtered out,
  including the 29-target historical test. The builder's 35-target / 385-row historical differential has
  0 mismatches.
- No push, release or deployment. Do not push or publicly distribute the R-2 implementation while Q-B1 is unresolved.

## Independent qualification

| Step | Outcome |
|---|---|
| Original independent review of c67c5124 (Astra xhigh flex, Pi/Docker) | **REJECT** on F-01 (P2, verified): a missing revision commit after validation was classified `invariant_failure`; F-4 requires `cannot_read`. O2 and O5 passed within coverage. |
| Delta recheck of 0d021609 (Astra xhigh flex, Pi/Docker) | **No reviewer verdict.** The proxy refused request 15 (`usd_limit: spent 0.5159 + worst case 1.5374 > limit 2.0000`). The reviewer executed checks 1, 2, 5 and 6 and source-reviewed 3 and 7. It did not complete check 8 (no new P1/P2) and wrote no verdict. |
| Recheck coordinator (Claude Opus 5.5 high) | **ACCEPT WITH RESERVATIONS** on the combined evidence. This is a coordinator conclusion, not independent acceptance. |

**R-2 has not received completed independent acceptance.** A further paid recheck was not initiated.

## Unresolved matters

1. **Q-B1**: provenance and licensing of the Git-style diff implementation (`src/anchor/diff.rs`; the original
   review named lines 258-358 as its most distinctive part). Owner/legal reservation. It blocks push or public
   distribution.
2. **Q-R2-2**: document and rename anchor behaviour. Rename following is not implemented. The original review found
   it safe only if A4 excludes null anchors (0 of 17 frozen cards affected). Owner-reserved.
3. **Incomplete repair testing**: a missing *packed* revision commit and post-validation ambiguity are not exercised
   end to end. Ambiguity is covered at unit level only. The packed case is inferred to take the same
   `resolve_prefix -> Missing` path. Check 8 of the recheck was completed only by the coordinator's source review.
4. **Resource and CI coverage limits** (carried from the original review):
   - FO3-01: there are no built-in time, byte or decision bounds. Myers worst case is O((N+M)D) time. A debug probe
     timed out at N=16000 reversed lines (10 s), and untrusted inputs need external limits.
   - Q-B5: the history test panics without full repository history, by design. Shallow or archive CI runs were not
     executed, and CI must supply the history.
   - The workspace-wide `cargo check` was not completed in either review (crate scope only).

## Evidence

Preservation manifest: `~/cbm-work/research-phase-closure-20261111/r2-preservation/MANIFEST.md` (sha256
`f300c3e7e8761617d1289aeff4ee1b2bd699e78561edbeb1ddb2e28ef2e3a69a`). It lists the bundle
`r2-candidates-20261111.bundle` (sha256 `1a1f3257333905cf7c1c25b51c5b7fac359721c7ba0cee35add8f11864457a76`,
complete history, both branches and both tags; verified by `git bundle verify` and by a bare clone with `git fsck`).
It also gives the sha256 of each artifact below:

| Artifact | sha256 (prefix) |
|---|---|
| Original review package manifest `~/cbm-work/r2-independent-review-20261111/package/MANIFEST.sha256` | ebb1521f |
| Original frozen reviewer report `.../coord/reviewer-output-frozen/report.md` | 5a1c06c0 |
| Original decision packet `.../decision-packet.md` | 840deca0 |
| Repair recheck packet manifest `~/cbm-work/r2-f01-repair-20261111/recheck-packet/MANIFEST.sha256` | 6a714107 |
| Recheck package manifest `~/cbm-work/r2-f01-recheck-20261111/package/MANIFEST.sha256` | 9277034a |
| Recheck frozen reviewer report `.../coord/reviewer-output-frozen/report.md` | 7b5b8714 |
| Recheck decision packet `.../decision-packet.md` | 9ae10405 |

The evidence lives under `~/cbm-work` on this host. The bundle and manifest are the portable copy of the commits,
not of that evidence.

Spending as recorded in the closed budget records:
- Original review: reviewer 2.4641 actual flex; coordinator about 2.8 of 5.00.
- Recheck: reviewer 0.5159 actual flex of 2.00; coordinator 1.3069 of 2.50 at the last reading before closure.
