# Repository claims: local operator guide

Work Engine's native repository facts keep reusable assertions and exact
evidence under `claim-evidence`. The local deployment supplies identity and
authority; MCP provides reads. A fact's visibility is separate from a
consumer's decision to rely on it.

## Deployment inputs

A deployment selects a stable repository ID, decision scope, private canonical
root and CBM artifact directory. Its externally provisioned bootstrap v3
config/source files use `repository-claims-v1`; producer grants select
`repository-fact-v1` permissions. The producer startup file fixes the actor,
grant, allowed actions and expected bootstrap hashes. The read profile fixes
the same root/repository/scope and can additionally select evidence custody and
a current checkout for byte comparisons.

These private files stay outside Git. The native host's README and Rust
`Startup` type define the closed producer shape; the server's `ReadProfile`
defines the read shape. The host's initialization consumes the already
provisioned grant set. Changing this adapter's grants requires a quiesced,
externally authorized deployment change; a publication cannot admit itself.
Hashes protect selected bytes, while trusted launcher and OS custody supply
authentication.

## Available controls

The installed native executables can be pinned by digest independently of the
working checkout. With an operator-selected startup and expected digest:

```text
<claims-host> initialize <producer-startup> <startup-sha256>
<claims-host> execute <producer-startup> <startup-sha256> < operation.json
<claims-host> inspect <producer-startup> <startup-sha256> < operation.json
```

The host accepts claim creation, exact-predecessor revision publication and
exact-revision reliance. Persistent operation identity distinguishes replay,
conflict, no effect and unknown outcomes. Inspection reconciles uncertainty;
an unknown outcome is not evidence that another publication is safe.

CBM captures select source paths and tool requests under a pinned repository
binding. They retain raw observations, source bytes, publication identity,
coverage limitations and documentation-reference pages:

```text
<cbm-capture> capture --binding <binding> --request <capture-request> --artifacts <artifact-directory>
```

An immutable capture reference fits the native claim's evidence fields. The
producer supplies the bounded proposition, assumptions, support qualification
and limitations; capture integrity does not supply semantic judgment.

## Session access

The claims-only server entry shares a private read profile across concurrent
stdio clients and starts no HTTP listener or legacy session tools:

Its profile loader is
`rust/crates/work-engine-server/src/main.rs::load_claims_stdio`; its stdio
lifecycle is `rust/crates/work-engine-server/src/main.rs::run_claims_stdio`.
These explicit documentation references identify implementation targets for
CBM. Whether the targets implement the surrounding assertion is a separate
evidence-backed judgment.

```text
<server> claims-stdio /absolute/read-profile.json
codex mcp add work-engine-claims -- <server> claims-stdio /absolute/read-profile.json
```

The tool catalog advertises read-only, nondestructive hints. A Codex deployment
can explicitly restrict this server to its four read operations and use its
server-specific approval mode, while leaving the general approval policy in
place:

```toml
[mcp_servers.work-engine-claims]
command = "/absolute/pinned/work-engine-server"
args = ["claims-stdio", "/absolute/read-profile.json"]
enabled_tools = ["discover_repository_facts", "resolve_repository_fact", "read_repository_evidence", "check_repository_evidence_currency"]
default_tools_approval_mode = "writes"
```

Codex documents these server and per-tool controls in its
[configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference).
Hints describe tool behavior; the native owner and startup binding enforce
scope and authority, independently of client approval choices.

The resulting tools discover candidates, resolve exact revisions, read captures
and check selected-source currency. Large results retain complete provenance
through digest-bound byte pages. Tool callers cannot initialize roots, publish,
record reliance, choose authority or change startup scope. Separate filesystem
or process capabilities remain governed by their own host authority.

A consumer can inspect a candidate and its evidence, decide its applicability
to its current work, and name the exact revision in a reliance decision. The
trusted producer can record that decision for the consumer/revision/scope;
this is producer attribution, not authenticated consumer attestation.

For the local Work Engine deployment, stable configuration and artifacts live
under `~/.local/state/work-engine/repository-claims/work-engine/`; the setup
receipt identifies the actual binary pins, bootstrap hashes and exact seeded
claim references. These paths select one deployment, not a repository-wide
requirement for other operators.

Source skills describe the profile, but active agents discover frozen skill
releases. Editing the source does not deploy a release. A skill release has its
own concrete plan and operator decision.

## Reassessment

Source-byte change or an unresolved documentation reference is evidence for
reassessment, not an automatic semantic verdict. A model or other authorized
consumer determines whether the assertion remains supported. The producer can
publish a manual successor, while historical revisions and existing exact
reliances remain resolvable. Change-driven refresh, claim invalidation and
downstream reopening require separately owned lifecycle behavior.

## Documentation/code qualification

The `claims_documentation` example creates a private fixture, separate claims
root and CBM runtime. Select the four installed binaries explicitly through
`WORK_ENGINE_CBM_BINARY`, `WORK_ENGINE_CLAIMS_HOST`,
`WORK_ENGINE_CLAIMS_SERVER` and `WORK_ENGINE_CLAIMS_CAPTURE`, then run:

```text
cargo run -p work-engine-server --example claims_documentation -- prepare --state /absolute/fixture-state.json
cargo run -p work-engine-server --example claims_documentation -- advance --state /absolute/fixture-state.json --case code
cargo run -p work-engine-server --example claims_documentation -- advance --state /absolute/fixture-state.json --case docs
cargo run -p work-engine-server --example claims_documentation -- advance --state /absolute/fixture-state.json --case unresolved
cargo run -p work-engine-server --example claims_documentation -- cleanup --state /absolute/fixture-state.json
```

Run these commands from `rust/`. Preparation publishes a fixture-producer claim
about one observed sample. Each advance changes and captures evidence without
revising that claim. The manifest exposes its read profile and exact capture
references for a consumer's reassessment; the trusted producer can subsequently
record reliance or publish a manual successor. The ignored
`documentation_claims` integration test exercises the mechanical sequence with
explicit fixture judgments. A real model consumer's applicability decision is
separate evidence.

The fixture distinguishes changed implementation, contradictory documentation,
and unresolved navigation. Its compiled output covers only the recorded
sample. An unresolved reference does not establish absent implementation or
false behavior, and documentation-reference change rows do not provide general
code-impact analysis. Cleanup stops only the fixture's private CBM daemon and
retains its evidence.
