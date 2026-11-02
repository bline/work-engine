# Deterministic bookkeeping host: prepared release, 2026-11-02

Status: prepared and qualified for bounded fresh-task use; **not activated**.
The [approved core plan](../planning/deterministic-bookkeeping-host-plan-20261102.md)
permits this release-ready outcome. The [skill-release owner](../skills/skill-release/SKILL.md)
says: “Deploy only on the user's decision.” No global switch is inferred from
implementation approval. Optional structured Dot grants remain outside this work.

The exact source candidate is
`a0cda34e55e8fd4c44e4a2c7f7932279d9418247`, branch
`bookkeeping-host-20261102`, clean private worktree
`/home/bline/cbm-work/wt-bookkeeping-host` at qualification. Its 25 changed files
were integrated into the shared working tree while preserving HEAD9e4d7849 and
pre-existing edits. The shared checkout is not the clean release source.

The saved release comparison is
`/home/bline/.local/state/work-engine/bookkeeping-host-20261102/integration/release-plan.txt`.
It compares current release `133946956e6cbc2f7aba0d2a9aa8d7dcedb4a423`
with a0cda34e. Installed changes are durable-state (two files) for Codex and
manual-orchestration (five changed, seven added files) for Codex and Claude.
Other installed skill bytes and install lists are unchanged. The candidate's
legacy catalog has unknown exported-support dependency coverage; an empty
potential-consumer list is not proof of no indirect consumers. The generated
session-events synopsis repair is included in the source but that skill is not
directly installed by these lists.

Skill export alone does not install the Rust executable. The tested compatible
bundle is:

```text
bundle 8b8621bdecd25de9b7893e6115b0d151296429c3b6b5fdda0e23406ad93ed56b
binary SHA256 fe560261c03dc0b25b81a645482e67bf7091b93aa2044a9316f142d6c48e1518
manifest SHA256 3476a6fe7005b02b73348d046ffcf791a1a4b83fc0ffe2f3aadd25d442ed3e90
context reader SHA256 d16c5f3799e04da374adc5595d9643354d64457ebca3cf3468cf4566824f1204
```

The bundle is under
`/home/bline/.local/state/work-engine/bookkeeping-host-20261102/implementation/provision/tools/manual-host/<bundle>/`.
It contains the executable, schemas, provisioning source and pinned Python import
closure. Its source, build custody and tested offline bind/import checks are in
`implementation/pilot-repair-result.json` under that same objective directory.
The separately pinned context reader is under
`/home/bline/.local/state/work-engine/tools/context-status/<context-reader-SHA256>/`.
No PATH alias, persistent service or global skill link was changed.

After a release decision, recheck the current release and exact plan, then run
the reviewed publisher from the repository root:

```sh
python3 skills/skill-release/scripts/skill_release.py plan a0cda34e55e8fd4c44e4a2c7f7932279d9418247
python3 skills/skill-release/scripts/skill_release.py deploy a0cda34e55e8fd4c44e4a2c7f7932279d9418247
python3 skills/skill-release/scripts/skill_release.py status --verify
```

Use the tested binary bundle explicitly for new authorized operations. Rebuilding
with `skills/manual-orchestration/scripts/provision_manual_host.sh` from the
clean candidate records a new build's provenance and verifies its scripts; do
not assume compiler/build differences reproduce the tested binary hash. Qualify
the resulting pair before using it. Existing operation state remains bound to
its original exact executor. The generated schemas and
[manual-host guide](../skills/manual-orchestration/references/manual-host.md)
define the request and explicit closure interfaces.

Rollback restores the exact previous skill commit through the same release
owner (or its verified previous-release rollback command), and returns new work
to the previous manual route. Keep all host state, bundles and receipts. An
alternate binary refuses mutation of state it does not own; `inspect` remains
available. The old b89eb556 bundle can still inspect its preserved r1 state
(qualified by `integration/rollback-inspect/old-pin.json`); it has known pilot
defects and is not recommended for new work. Cross-binary refusal is covered by
the lifecycle fixture. There is no automatic state migration. Unknown effects
remain unresolved until their owner reconciles them.

There is no claim that this rollout has already occurred, that an ended root
will be awakened, or that configuration-qualified retained tasks are supported.
Fresh-task use remains bounded by explicit root authority and collected results.
