# Executable-source maintenance: approved baseline reconciliation

The operator approved this reconciliation on 2026-09-12 in response to the
binding incompatibility recorded in chatboard 515. The original implementation
plan remains at SHA-256
`72d214f83bc7761f4f2920f59309e5015398eccad3b6dccb27ac616e58600203`.
This record changes its implementation baseline, not its protected outcomes.

The reconciled subject consists of:

- published lifecycle baseline `ad99d46cd73996b580f1263496ccba75acc2c431`;
- preserved PPCE Candidate 8 `f5ed0bf362e2c510a28ff1d699192c30168405c4`,
  tree `32572e25c3c1863eda10b2d43474d503ad5eeaf3`, patch digest
  `a4c01d50c93c0765bca1a96eb85eeeb555f4947a1045661a84075d54953cd721`,
  scoped to its PPCE paths rather than its older lifecycle tree;
- unfinished maintenance checkpoint `efe86afc169304998cfbe0e4926883d77211e2aa`,
  carried forward as unfinished work, not accepted implementation.

Main already contains earlier PPCE versions. Candidate 8's PPCE deltas supersede
those earlier versions where they differ; newer lifecycle fixes and unrelated
documents on main remain intact. The only overlap with the maintenance patch is
the capability-host runtime: it retains Candidate 8's baseline-succession and
absent-subject wiring alongside maintenance's code-source/operational-target
split. No original dirty worktree is used as the source of merged bytes.

The added reconciliation scope is Candidate 8's differing PPCE paths:

- `app-server/src/index.mjs`
- `app-server/src/services/claim-evidence/production-path-contract.mjs`
- `app-server/src/services/claim-evidence/production-path-service.mjs`
- `app-server/src/services/slice-campaign/baseline-succession.mjs`
- `app-server/src/services/slice-campaign/capability-contract.mjs`
- `app-server/src/services/slice-campaign/capability-host-runtime.mjs`
- `app-server/src/services/slice-campaign/contract.mjs`
- `app-server/src/services/slice-campaign/native-review-closure.mjs`
- `app-server/src/services/slice-campaign/native-review-host.mjs`
- `app-server/src/services/slice-campaign/service.mjs`
- `app-server/tests/ppce-host-contract-succession.test.mjs`
- `app-server/tests/services/claim-evidence/production-path-service.test.mjs`
- `app-server/tests/slice-campaign-native-review-vertical.test.mjs`
- `app-server/tests/supervisor-campaign-capabilities.test.mjs`
- `app-server/tests/supervisor-native-review-hosting-integration.test.mjs`

The original maintenance path allowance, strict fingerprint equality, inert
validation, independent implementation review and exact activation gates remain.
Later checkpoints retain Candidate 8 as their Git parent for PPCE review lineage;
the maintenance WIP remains separately reachable and attributed by receipt.
Neither reconciliation nor normal-operation activation grants incident 488 input
release, paid API use, campaign acceptance, publication to main or push authority.

Read-only verification of the reconciled source reproduces the live generation
fingerprint and both retained role fingerprints exactly. The diagnostic is
recorded at
`/home/bline/.local/state/work-engine/ppce-activation-ad99d46/reconciled-compatibility.json`.
It performed no activation or provider entry and left live bindings and generation
state unchanged. This closes the specific baseline incompatibility, not the
remaining implementation, review or activation obligations.
