# AH3 selected-composition qualification — 2026-11-05

Status: accepted synthetic local qualification. This dated record preserves the
accepted result and four reviewed proof helpers. The user authorized its commit
and local merge after qualification acceptance. The copied reports retain their
original private-acceptance and publication wording as historical facts.

The tested subjects were Work Engine server commit
`839b8c4ee9f2c5f83d4b5ba342997021f29b8f35` (tree
`150656892c1a17756ada6d98cbf2d00cb89c097f`) and Web consumer commit
`285638858097d656797709fb376aecc41d5b0240` (tree
`51291774fcad56745840863957176beee3124dea`). The server executable SHA-256 was
`587c24a0d387906b29174043ad3178ec4e035031fc1ce0bcac40f1efda97b5b4`;
the actual query CLI executable SHA-256 was
`a994c4bb7a5200d9b9c47b944a436dcd10e13b2f3adc949662c66089e1d92d4d`.

Five final composed scenarios and six additional Web scenarios passed. All 13
custody gates recorded exit 0 and no remaining processes. The selected HTTP,
MCP, public Rust client, query CLI and actual browser paths exercised shared
task+query operation with MCP enabled and disabled, exact signed-string
preservation, unknown reads before first consume, lost-acknowledgement/restart
reconciliation, denied and disabled capabilities, currentness and routing.
Task+command startup was rejected. A separate native Astra reviewer accepted
the exact private candidate with no open findings; the review report records
its observed read-only grant, retained retry and evidence limitations.

The accepted scope is cooperative local Linux with synthetic fixed policy,
mailbox, authenticator, disposable stores and clock. Task readiness remained
false/unestablished. Live providers, deployed passkeys or mailbox, execution
admission, Pi, A5, global release readiness and existing user-state migration
were not qualified. Some early failed attempts lack complete dynamic snapshots;
retained final evidence does not reconstruct those missing snapshots.

The [publication manifest](publication-manifest.json) maps every copied artifact
to its original path and SHA-256. The [original freeze manifest](evidence/freeze-manifest.json)
binds the 102-entry candidate. This record contains a selected source/evidence
projection; the full candidate binaries, runtime snapshots, caches and archive
remain in the referenced private state directory. Historical absolute paths in
the copied reports and [reproduction notes](evidence/REPRODUCE.txt) remain as
written. Reproduction requires those source worktrees and existing build/browser
caches; this record alone is not a portable runnable test suite.

The four files in [qualification/](qualification/) are byte-identical historical
proof snapshots. Their original worktree paths were
`rust/crates/work-engine-server/examples/ah3_client.rs` and
`rust/crates/work-engine-server/tests/{ah3_composition,ah3_queries,ah3_profiles}.py`.
They were not added to the production build or permanent test infrastructure.
The accepted AH839 and Web285 production sources were unchanged by AH3.

See [acceptance](evidence/ACCEPTANCE.json), [final result](evidence/FINAL-RESULT.json),
[review](evidence/review/REPORT.json), [scenario results](evidence/final-summary.json)
and [additional Web results](evidence/web-summary.json) for the precise subjects
and limitations. Publication of this record does not qualify later production
commits or expand the original acceptance scope.
