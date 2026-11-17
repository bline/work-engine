# H2 execution packet draft (S2 artifacts index)

**DRAFT — not approved.** This is an index of draft bytes. It is not an approval, grant, installation, attestation or manifest publication, and it decides none of D1–D9 (plan section 3). Plan: `planning/research-lifecycle-m4-s2-plan-20261117.md`, section 4. Worker: successor Pi worker under root `01a1e56d-7ceb-71fc-9c46-c9fcd9618234`. Source read at shared HEAD `a3c8285628c99004cb83e578c3aa4d29c2f90480`. The artifacts live outside the repository in `/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/007-h2-artifacts/` (`O/007-h2-artifacts/`).

## Artifacts

| File | SHA-256 |
| --- | --- |
| `gate-trace.md` | `0940e595677715f3583cee68bc0a13609a4db4f16b3615b80c245860aa5a1e6c` |
| `gate-loci.json` | `ea4c01f78151586ffa8317e43021c7bf0f94379e88da80ba5ffe0abe4a31402b` |
| `audit-request.json` | `98ff9e25ef9726e3a8d389d4189e1fc5508e4592a39d0ba4a380750305d74f76` |
| `snapshot-manifest.json` | `0fe767fd9b8988411874485a7307bd5e09d4fbfca37fbb8878b95dc4d4ba3fdf` |
| `rubric.md` | `9fd138c8b0fc35c4d0d7d285460c8e1ea22de3921ea1e9e611cae8afb45ddc66` |
| `lc7-protocol.json` | `25d9bc1afca4a9f94a3daf75119cb5ab2a588eb2d5e8bd2845a76ec69f58702c` |
| `advisory-input.md` | `5c7c45ca3878773771887dc40c23e8360246bdf55f1ccee6f26d58b236c65103` |
| `authority-skeletons.json` | `b918a203c62ab4f37bd8eceb6bc45ef1adbf4237750a00bbf03edd18b1d1373f` |
| `completeness.md` | `78a9bc50dcbdbae25b6e3fa52551400a5826e334569aff69a776c453205f24cc` |
| `check.py` | `6657f8e018982d90ab7e14b384fcdda056d80497264aa8f625a77014923a67a3` |

`check.py` recomputes every digest from git at the named commit, re-reads every gate locus, and checks that every slot names one of D1–D9. After the review repair it passes under `custody.py run` (18 digest entries, 47 loci, 48 slots; records `O/010-s2-repair/custody/`), and fails as expected on a corrupted digest, a slot naming a nonexistent decision and a digest entry with `sha256` removed.

## What the operator should know first

- **Production prepare and adopt are refused by design, beyond the plan's section 1 table.** The host's installed-research dispatch accepts only `research.invalidate.v1`; production basis verification and Preparer/Adopter installation do not exist; the delivery consumer is `controlled-proof`-only (`gate-trace.md` G4, G5). Production startup also still refuses at `native_startup::publish` (G1) and the non-controlled protected-surface observation (G2).
- A production installation route exists in the task host (`--operator-research-installed`); what it lacks is real operator evidence and a non-controlled native hold (G3). That route is also source-fixed to the ChatGPT access-lease provider route (`research_installation.rs:1349-1362`), which narrows the D6 provider choice (G3, G7).
- The H2 passage that the register decision binds (`git:1a1cb443`, lines 88–101 of `context-boundary.md`) has moved: the levels text is now in `context-measurements.md`. The snapshot proposal pins both, and the register-pinned probe `metrics/probes/compaction-points-20261105.md` (HEAD blob equals the cited `d80dd242` blob).
- Every authority-bearing field is a named slot for a decision, listed in `completeness.md`. Reading the slots does not pre-decide them.
- **Deviation from plan 4.1.** `completeness.md` classifies five rows as a third state, `runtime` (the admission-packet records for consumer delivery, recommendation, prepared basis and adoption, plus the realized-startup/caller-evidence row). They bind bytes that do not exist until execution and none is operator-approved, so they are neither `exact` nor `slot`; plan 4.4's "satisfied or slotted" is not met literally. Root either accepts the classification or amends 4.1.

## Not established

No native run, build, test or inference took place. Whether the config knobs for the seven native utilities actually remove them from the tool registry is untested. No A4 card preview was produced. The `request_user_input` handling is a source read of the WE client and the native handler only.
