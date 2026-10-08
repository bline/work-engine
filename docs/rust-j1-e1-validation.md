# J1 and HP3-E1 implementation validation

Accepted on 2026-10-08 under the user's “Plan is authorized” instruction, against
baseline `035a970443c4b4ca8e663dd0e66f4b8b28746268`. Implementation used
`gpt-6-sol/high`: E0 first, followed by parallel J1 and E1. This record covers
implementation and controlled qualification, not publication or activation.

## Result

The permanent `review-execution-evidence` crate owns bounded controlled-process
capture, append-only SQLite custody, checked readers, and explicit failure
observations. J1 installs the concrete campaign-owned CE3 admission port,
persists exact completion stages, joins both claims with historical episode
readback, and recovers partial commits through their original owner references.
Campaign schema 3 and evidence schema 1 apply to new private roots only.

The combined test uses a pinned actual `tee` process through E1's producer and
reader, real campaign and claims stores, the native episode descriptor, and ER1
historical readback. Its independent output file remains exactly one copy of
the nonempty request through replay and recovery. This is bounded controlled
execution evidence, not general process-count or provider attestation.

The same-UID controlled child cannot attest an enforced read-only grant. Its
mutation authority is recorded truthfully, and J1 preserves unestablished
claims and their reasons. A successful owner join is not a campaign outcome.
J2, real providers, migration, disposition, deployment and activation remain
outside this implementation.

## Exact accepted subjects

| Subject | Source archive SHA-256 | Retained review receipt SHA-256 |
| --- | --- | --- |
| E0 contract | `9ef6d838502965337780b707b4d3733384c2d80932321ee548a3179461dee31a` | `1946a38a5b062631b5598ef975207527b7a528e3efc177b9b39cbf68e6f2ecc4` |
| E1 candidate 2 | `53eafb6b762e6734ffa7f55cd007300b49a05ebc7d7f2d8a603f45de9f0866c2` | `98421ad061301ed05b3388bd461cb22177fa935a757a9c58205be86796771f19` |
| J1 candidate 3 | `952b48ed49098d2ad7f735cec1d73d4769a754e9cefe4ad79ee82b279fae7163` | `3408a7d65df8bc49ce3b9bcc8313bcf7cf22e7812fc507a3773398099a32b953` |

Evidence root:
`/home/bline/.local/state/work-engine/rust-e0-j1-e1-implementation-20261008/`.
Manifests, archives, gate logs, retained controlled roots, review findings and
remediation receipts are stored there. Review was read-only with retained
remediation context; no provider-independent review claim is made.

## Validation

The parent ran Rust 1.92.0 with the shared lockfile, offline, all features and
serialized tests across `slice-campaign`, `claim-evidence`, `review-episode`,
`review-episode-store` and `review-execution-evidence`:

```text
cargo +1.92.0 test --locked --offline \
  -p slice-campaign -p claim-evidence -p review-episode \
  -p review-episode-store -p review-execution-evidence \
  --all-features -- --test-threads=1
```

Result: **150 tests and 14 doctests passed, zero failures**. Five explicit
historical-import/performance diagnostics were ignored. No new broad performance
claim is made. The 82 bound source/configuration hashes remained unchanged
during the final run. The Git index matched the task's starting snapshot.
Receipt: `integration/parent-final-gate.json`; log SHA-256
`aa058c708d78208e6a363ba0b85ea825688972dfa2730de4fb8fb7a7be61747f`.

Both lanes also passed their focused default/debug and release suites,
all-target Clippy with warnings denied in the relevant feature configurations,
and formatting checks. E1 covers actual child capture, fresh-process readback,
five process crash cuts, tampering, file replacement, fencing, limits and failure
facts. J1 covers nine actual process cuts: preparation, observation, both claim
establishments, both admission reads, episode commit, episode read and joined
campaign commit. Recovery uses owner APIs and preserves the original execution.

Retained review closed three E1 findings (file identity/fencing, refusal before
mutation, structured failure facts) and two J1 findings (post-effect progress
preservation and native codec fidelity). The final parent build caught one
test-only missing import; a fresh Sol worker verified the fix before the passing
rerun. Temporary-directory quota failures were infrastructure failures; successful
gates used local scratch under `/home/bline/code/.work-engine-tmp/`.

## Coordination and workflow

Lifecycle S6/S7 retained ownership of shared `rust/Cargo.toml` and `Cargo.lock`
and applied this task's additive dependency edges. Those shared files also
contain the lifecycle team's work and require separate attribution at publication.
No files were staged, committed or pushed by this task.

Workflow used two implementation workers, one retained read-only reviewer, and
one fresh final-gate verifier. E1 required one review remediation round; J1
required one semantic remediation round and final mechanical reconciliation.
Implementation and final validation took approximately 94 minutes, excluding
the earlier planning task. Token and cost totals were not exposed. No Claude
calls or live-provider calls were made. Repository graph checks were supplemented
by exact source reads for new or changed files; retrieval totals were not measured.
