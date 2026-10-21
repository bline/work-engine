# Pre-registration: an Anthropic agent as builder, n=1

Status: pre-registration, written 2026-10-21T23:30Z **before any source line of
the subject slice was written**. The pre-slice cost marker is frozen in
[`anthropic-builder-arm-marker-pre-20261021.json`](anthropic-builder-arm-marker-pre-20261021.json).

## Why this exists

The [pure-Codex baseline](../baselines/pure-codex-20261009/README.md) was frozen
on 2026-10-09 to be compared against once development moved. It has never been
compared against anything: in the [efficiency ledger](../efficiency-ledger.md),
M14 (Plus-units per accepted slice, marked primary outcome), M9 (candidate
attempts per slice), M10 (review findings per slice), M11 and M12 all still read
`—` in the Latest column.

Separately, no Anthropic agent has done development in this repository. Of 301
commits carrying Claude co-authorship: 748 `.md`, 41 `.json`, 40 `.py`, 16 `.rs`,
and **zero** path-touches under `rust/` or `app-server/src`. The 16 `.rs` files
are one commit that brought an existing experiment folder into version control.
The standing role was measurement and verification, which states "qualify and
report defects; do not fix them".

**The user authorized the role change for this slice.** That is recorded here
because it changes my contamination status: I become an arm, and an arm does not
score itself.

## Subject slice

`work-engine-context-status agents` reports child metadata only - `thread_id`,
`root_session_id`, `parent_thread_id`, `agent_path`, `rollout` - so a parent
cannot see its children's context pressure without running a separate
`--session` query per child. The
[handoff diagnosis](handoff-boundary-why-no-handoffs-20261021.md) named closing
that gap as candidate fix 2, on which its lead fix depends.

**In scope:** an opt-in flag on the `agents` subcommand that attaches each
discovered child's latest usage observation to its row.

**Out of scope, deliberately:** changing the reported denominator to the measured
compaction trigger band (diagnosis fix 3). That needs a constant the brief's
owners should choose, not me. Also out: any instruction change, any edit to
`rust/Cargo.toml` or `rust/Cargo.lock`, any new dependency, and any file outside
`rust/crates/work-engine-context-status/`.

**Why this crate.** It is the only clean crate in the Rust workspace: 42 modified
and 17 untracked files sit under `rust/crates`, none of them here. Working
anywhere else would mean building on other sessions' in-flight edits. Codex is at
quota, so no concurrent build holds the shared target lock - this is the safe
window, and it will close when quota returns.

## Acceptance conditions, fixed in advance

1. Without the new flag, `agents` output is byte-identical to the current binary
   for the same inputs, in both text and `--json` form.
2. With the flag, each child row carries its latest `input_tokens`,
   `cached_input_tokens`, `output_tokens`, observation timestamp and `epoch`,
   its `context_window_tokens`, a context fraction, and a lineage marker count.
3. Per-child values equal what `--session <child> --rollout <child rollout>`
   reports for the same child at the same time. This is checked, not assumed.
4. A child whose usage cannot be established carries a **distinguishable reason**,
   never a zero and never a silent omission - the brief's own rule and
   CLAUDE.md rule 2.
5. New tests cover: usage present, usage unavailable, and flag-off byte
   compatibility. The existing `tests/cli.rs` suite continues to pass.
6. `cargo fmt --check` and `cargo clippy -- -D warnings` clean for this package.
7. `rust/Cargo.toml` and `rust/Cargo.lock` unchanged.

Conditions 1-7 are the slice's own gate. They are **not** acceptance of the work
as a product change: that belongs to the crate's owners.

## Measures

| measure | baseline | how this arm is recovered |
| --- | --- | --- |
| weighted tokens for the slice | `uncached + cached/10 + 5*output` | delta from the frozen pre-marker, `metrics/scripts/claude_transcript_usage.py` |
| Rust lines added | per-slice `rs_lines` | `git diff --stat` on the slice commit, `.rs` only |
| **weighted tokens per 1,000 Rust lines** | **2.0-5.1 Mwt** (16-40 Plus-units/krs at 7.826 units per Mwt) | the ratio of the two above |
| candidate attempts to acceptance (M9) | mean 2.8, range 2-5, none accepted first | **deferred** - needs an independent acceptor |
| review findings resolved (M10) | mean 6.0, range 4-9 | **deferred** - needs an independent reviewer |
| wall-clock | not in baseline | first to last slice commit |

**M9 and M10 cannot be measured now and are recorded as `not established`, not
as zero.** Both require an independent reviewer and acceptor; the baseline's
figures come from adversarial review by a separate agent, and Codex is at quota.
Writing code that passes its own author's gate is not an accepted slice. Any
later claim about quality must come from that review when it can run.

## Confounds and limits, stated before the result

- **n=1.** One slice, one builder, one crate. No distribution follows.
- **Small-slice penalty is expected and is not a finding.** Baseline slices are
  5,236-43,296 Rust lines; this one will be a few hundred. Fixed costs - reading
  the crate, the test suite, the conventions - divide by a much smaller
  denominator, so weighted tokens per 1,000 lines should come out **worse** than
  baseline for arithmetic reasons. Report it, do not explain it away, and do not
  treat a bad ratio as a capability result.
- **Spec independence is weaker than baseline.** I identified this gap myself in
  the diagnosis, so builder and specifier are the same party. Baseline slices had
  separate planning. This favours the arm.
- **Different harness, not just a different model.** Claude Code's tools, caching
  and context handling differ from Codex's multi-agent v2. The comparison cannot
  separate model from harness, exactly as the handoff pilot could not separate
  telemetry from instruction.
- **Cache accounting is analogous, not identical.** Anthropic
  `cache_creation_input_tokens` is counted as uncached and `cache_read_input_tokens`
  as cached, to match the baseline formula. The underlying price ratios are the
  baseline's assumptions, carried over unverified.
- **No Plus-unit or currency claim.** A Plus-unit is 1% of a Codex weekly quota
  and an Anthropic agent consumes none. Only weighted tokens are common.
- **The 158.03 Mwt already spent** on this session's measurement line, over 4,541
  calls since 2026-10-09, is **not** slice cost and is not attributed here.

## Scoring ownership

I may report the arithmetic and the recovered denominators. **I must not score
this arm**, for the reason recorded in the role boundary: an arm that scores
itself authors its own result. Judgment on condition 1-7 compliance, on M9/M10
once review runs, and on what the ratio means belongs to the user or an
independent party. If I state a verdict on my own slice, that is the error to
catch.

## Reproduction

```
python3 -I metrics/scripts/claude_transcript_usage.py <out.json>   # cumulative, this session
git show --stat <slice commit> -- '*.rs'                            # lines added
cd rust && cargo test -p work-engine-context-status
cd rust && cargo clippy -p work-engine-context-status -- -D warnings
```
