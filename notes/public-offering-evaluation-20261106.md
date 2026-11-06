# Public offering: evaluation and deferral

Recorded 2026-11-06 UTC. Claude evaluated the
[Devwits + Work Engine public offering proposal](devwits-public-offering-proposal-20261106.md)
against the repository, and ChatGPT reviewed that evaluation. The user, ChatGPT
and Claude agreed on the direction below.

**Status: a decided deferral and direction, not authorization.** Moving to a
product is deferred until Work Engine provides a full development environment
(milestone 2 below), and the user will revisit it then. This note authorizes no
implementation, provider use, deployment, publication or spending. It does not
change the status of any owning document.

Source baseline: work-engine `78f00c15` (local branch `rescue-detached-20260911`)
and devwits-auth `2856388`. The standings below were read from source and records
at those commits.

## Decision

Build the engine before the platform. The proposal's long-term Devwits
architecture stays as a reference. The immediate work is to finish and
generalize the live Rust vertical. Accounts, a public gateway, metering and
cloud hosting do not compete with that work for development time.

| Milestone | Content | Standing |
| --- | --- | --- |
| 1. Governed execution proof | A real coding task through Events MCP → Rust server → Pi/Docker → receipt | Reached, n=1 through the full path (2026-11-06) |
| 2. General-purpose local development | Multi-file changes in real checkouts, configurable tools, existing review/acceptance/publication/claims owners attached, repeatable runs | Next; this is the revisit trigger |
| 3. Limited public preview and Dot demonstration | Reproducible install, one supported provider, a matched-task benchmark, remotely signed approvals over the **existing** OpenAI Secure MCP Tunnel | After 2; no multi-tenant service needed |
| 4. Commercial Devwits platform | Accounts, GitHub App, public OAuth MCP gateway, runtime enrollment, hosted workers, storage plans, metering, payments | Deferred |

Agreed scope cuts for the first preview: Linux and Docker only, a Rust-only coding
profile, one provider, and limitations stated plainly.

The proposal's efficiency figure (around 50% less inference per accepted unit)
is the user's estimate, not evidence. Hard data is to come from a working
environment, so the figure is not evaluated here.

## Current state against the proposal

| Capability | Standing at the baseline | Evidence |
| --- | --- | --- |
| End-to-end coding path | Live through the full MCP path once (`gpt-6-luna`, accepted). Its transport identity was a random-token fixture, not real OAuth onboarding. The run was driven by a private operator script with no tracked copy in the repository | [pilot README](../records/local-development-pilot-20261106/README.md) "Transport identity" paragraph; `LIVE-CUSTODY.json` argv names `~/.local/state/work-engine/local-development-pilot-20261106T0004/operator/run_pilot.py`; `git ls-files` has no `run_pilot` |
| Other real-provider runs | 2026-11-04 direct host run: completed. 2026-11-05 continuity trial: four requests and the reset worked, but the edit matched zero lines and the validator never ran. 2026-11-05 seeded trial: completed in 50.083 s. 2026-11-06 Sol/high: refused at admission, no provider entered | [11-04](../records/rust-lifecycle-s10-pi-docker-real-coding-result-20261104.md); [11-05 decision](../records/local-development-system-qualification-20261105/S10-OWNER-DECISION.json); [11-05 seeded](../records/local-development-prerequisite-repairs-20261105/README.md); [Sol](../records/sol-high-live-pilot-20261106/README.md) |
| Job scope | Only `src/lib.rs` is writable, in a disposable workspace, with one fixed tool image | [`pi-coding.md:19,152`](../rust/crates/work-engine-manual-host/docs/pi-coding.md) |
| Isolation | Three containers: inference (network only on live jobs), tool (no network), validator (no network). Images are created with `--pull never` | pilot README; [`pi_docker.rs:849-850`](../rust/crates/work-engine-manual-host/src/pi_docker.rs) |
| Admission, limits, receipts | On the live path. The receipt keeps execution status separate from acceptance, which is hard-coded `owner_pending` | [`pi_docker.rs:1276`](../rust/crates/work-engine-manual-host/src/pi_docker.rs) |
| Review, acceptance, publication | Separate binaries, qualified with fixtures only. No Rust code consumes a coding receipt. Production publication refuses: "production repository custody is not installed". The review host's protected `/etc` policy needs an interactive `sudo` install and is not installed | [review entry record](../records/rust-review-runtime-entry-20261106/README.md); [`store.rs:663`](../rust/crates/workspace-publication/src/store.rs); board 4757 |
| Claims in coding jobs | The claims bridge profile is a provider-disabled fixture | [`pi_coding.rs:26-30`](../rust/crates/work-engine-manual-host/src/pi_coding.rs) |
| Context pressure | The live job's pressure is a fixed 9500 basis points written by job preparation, not a measurement | [`coding-job-prepare.rs:361`](../rust/crates/work-engine-server/src/bin/coding-job-prepare.rs) |
| Model provider | Only `openai-codex` at `chatgpt.com/backend-api`, using a ChatGPT-mode credential read from the Codex CLI's auth file. API keys are refused and there is no token refresh. A case-insensitive search for `anthropic` over `rust/crates/**/*.rs` has 0 hits | [`pi_coding_provider.rs:77-81`](../rust/crates/work-engine-manual-host/src/pi_coding_provider.rs); [`pi_coding_auth.rs:106-112`](../rust/crates/work-engine-manual-host/src/pi_coding_auth.rs) |
| Signed approvals | Live for the owner: auth.devwits.ai → Redis mailbox → laptop Python verifier → board reply. A Rust WebAuthn verifier exists, but plain coding admission refuses signed profiles, and live coding jobs are authorized by an operator-pinned credential | [`proof.rs:87-102`](../rust/crates/session-task-admission/src/proof.rs); [`pi_docker.rs:310-319`](../rust/crates/work-engine-manual-host/src/pi_docker.rs) |
| Remote MCP | Stytch **test** project plus the Rust `mcp-auth` verifier, which accepts exactly one subject and one client. Served on loopback through the OpenAI Secure MCP Tunnel, currently observation-only. There is no request queue of our own: an offline laptop means a failed call | [`mcp-auth/src/main.rs:166,187`](../rust/crates/mcp-auth/src/main.rs); [`session_events.py:3655`](../skills/session-events/scripts/session_events.py) |
| Multi-user identity | None. devwits-auth hard-codes `OWNER_USER_ID` and uses Stytch test tokens | Search below; devwits-auth `api/_lib.js:36`, `src/config.js:5` |
| Distribution | None: no init, doctor, installer, packaging or Rust CI. Linux-only | Search below |
| Hosting, metering, billing, GitHub App, credential encryption | None | Search below |
| Public repository | GitHub `bline/work-engine` is public. Its default branch `main` was last updated 2026-09-16 (`a69efc44`) and is 802 commits behind local HEAD. The Rust tree is public only on `origin/rescue-detached-20260911`, 9 commits behind local HEAD | `git rev-list --count`; `gh repo view` |

**Absence basis.**
- **Searches:**
  - `tenant|org_id|workspace_id|customer` over `rust/crates/*/src/**/*.rs`, which is all first-party Rust source: 0 hits.
  - `fly.io|flyctl|stripe|octokit|octocrab|installation_token|installation access token|metering` over both repositories, excluding `node_modules`, `target`, `.git`, `dist`, `reviews` and `.claude`: one hit, `docs/agent-experiment-live-tests.md:283`. It concerns metering model calls in experiments, not billing.
  - `Dockerfile*`, `install*.sh`, `Makefile` and `justfile` across both repositories: none.
  - `.github/workflows/`: only `proposal-packets.yml`.
- **Coverage:** these cover source, configuration and packaging in both repositories. They do not cover private operator state under `~/.local/state`, which is not product code.
- **Earlier pass:** a read-only agent search also found no init/doctor/install/upgrade subcommands in non-test Rust.

## Gaps the proposal under-weighted

1. **Job scope.** One writable file and a fixed toolchain prevent general
   repository work. This is the main product gap. It is also what keeps Work
   Engine from doing its own development: the planned S10 full-loop job keeps the
   same narrow scope.
2. **Model access.** The proposal makes bring-your-own API key the default. The
   only built lane is the ChatGPT subscription login. The proposal itself says
   that lane cannot be assumed to be permitted for third-party or hosted use.
3. **The loop is not closed.** Review, acceptance and publication are not yet
   attached to coding receipts. Closing that loop is the current S10 runtime-entry
   work (board 4758, 4763).
4. **Signed approval is not on the coding path.** The verifier and a disabled
   signed-Pi candidate (`examples/mcp_pi_coding.rs`) exist; connecting them is
   integration work.
5. **Installation.** Root-owned policy, unpublished images, a private pilot
   driver and pre-provisioned runtime trees make a third-party install
   impossible today.

## Reusable designs

- [A2 connector identity and scoped access](../planning/a2-connector-identity-and-policy-design-20261105.md):
  installation registry, scope intersection and revocation ordering. Includes
  bids of $30 (A2.1) and $35 (A2.2); implementation is unapproved.
- [Connector adapters around existing owners](../planning/connector-adapter-architecture-plan-20261104.md):
  stages A0–A5, including the shared Rust application server.
- [Named provider connections and auth fallback](provider-auth-connections-and-fallback-direction-20261106.md):
  the user's direction for API-key/OAuth connections and bounded fallback.

## Relative sizing (unverified estimates)

Claude's first-pass sizes per proposal stage are below, assuming the pace of
early November 2026. ChatGPT judged them useful for **relative** sizing but not
as a calendar forecast. The user expects faster progress now (next section).
None of these figures has been checked against an outcome.

| Stage | Rough size |
| --- | --- |
| 0 Baseline and loop closure | 1–3 weeks |
| Job scope (not in the proposal's table) | 3–6 weeks; less for a Rust-only preview, since the tool image already carries a Rust toolchain |
| 1 Local distribution, including an API-key lane | 4–8 weeks |
| 2 Multi-user identity | 3–5 weeks |
| 3 Public MCP gateway, queue and outbound connector | 4–8 weeks |
| 4 Signed approval into coding admission, delegation grants | 2–4 weeks |
| 5 GitHub App, secret custody, multiple installations | 3–6 weeks |
| 6 Hosted runtime, metering, payments | 8–16 weeks, plus ongoing operations |
| 7 Public beta, security review, benchmark | 4–8 weeks |

**Calibration points used:**
- devwits-auth: 31 commits from 2026-11-01 to 2026-11-05, about 3.8k lines, single-owner.
- The MCP → Pi vertical: Docker probes started 2026-11-03 and the full-path pilot ran 2026-11-06, on top of the existing Rust foundation.
- The A2 bids above, which reach fixture level only, with nothing activated.

## Velocity hypothesis

The user's view is that progress will now be much faster. Before S10 everything
was built against contracts and fixtures, and nothing ran end to end until the
first days of November.

**Supporting data point (n=1).** Joining the review host to the running path
showed that its startup still used the historical `TrustedStartup::load` rather
than the new per-job pin path (board 4757). Fixtures had not exposed this. It was
fixed, reviewed and accepted the same day (`a8fde529`, board 4763).

**Expected shape.**
- *Fastest:* attaching existing owners, namely review, acceptance, publication,
  claims, signed approval and measured context pressure.
- *Slower:* new builds, namely job scope, installation, the API-key lane and all of
  milestone 4.
- *Unaffected:* human approvals and protected installs. The same runtime-entry
  slice spent about $8.70 of its $35 budget at checkpoint. Its live job then waited
  on a reserve-amendment answer and a `sudo` install (board 4763).

**Measurement.** Measure retrospectively from transcripts, the usage ledger, the
board and records after the next two or three slices. Record three things per
slice: time from start to a live-qualified result, actual cost against the bid,
and how many live-only mismatches surfaced. No prospective tracking is to be
added.

## Open decisions for the revisit

1. **Preview provider.** Either confirm that a third-party local tool may use a
   person's ChatGPT login (the proposal cites OpenAI's "Sign in with ChatGPT"
   program; Work Engine's eligibility is not established), or build an OpenAI
   API-key lane. Reading another application's credential file is fragile in
   either case.
2. **Public default branch.** Bringing `main` up to date is the user's
   publication decision. An open-source preview needs it.
3. **Proposal §14** (gateway pricing, workspace roles, storage meaning, billing
   meter, hosting provider, credential custody, distribution, repository
   relationship, operations). All of these remain open and belong to milestone 4.
