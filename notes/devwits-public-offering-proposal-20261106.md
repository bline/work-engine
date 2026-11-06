<!--
Recorded 2026-11-06. The user supplied this document from a ChatGPT
conversation (original filename `devwits-work-engine-public-offering.md`); it
is kept here as written, including its own status line and links. It is a
product-direction proposal, not an implementation authorization. The
repository-side evaluation and the decision to defer it are in
public-offering-evaluation-20261106.md in this directory.
-->

# Devwits + Work Engine: Public Offering and Deployment Architecture

Status: Product direction and architecture proposal — not an implementation authorization\
Date: November 6, 2026\
Scope: Open-source local Work Engine, optional Devwits account and integrations, and optional metered hosted execution

## 1. Executive summary

Work Engine is an open-source environment for governed, model-mediated engineering. Its purpose is to improve the amount of accepted, independently verifiable work obtainable from a given inference budget, while keeping human authority, evidence, runtime effects, and publication under explicit contracts.

Devwits would provide the optional public-facing product around that engine:

1. Self-installed Work Engine: Run on the developer's machine or infrastructure, using their repository and authorized model connection. The engine should work without a Devwits account.
2. Connected Work Engine: Link an installation to a Devwits workspace for a Web interface, remote job visibility, signed approvals, and integrations such as ChatGPT/Dot through one public OAuth-protected MCP server.
3. Hosted Work Engine: Run the same engine in isolated, metered cloud execution environments with persistent workspace storage. Customers bring their own permitted model credentials; Devwits charges for allocated storage and consumed infrastructure, not AI inference.

Public value proposition: Get more accepted engineering work from the model access you already pay for, and supervise it remotely without giving an AI unrestricted authority.

Early internal pilots and studies motivate an efficiency hypothesis, potentially around 50% lower inference consumption per accepted unit of work or better. This is a validation target, not a verified public claim. A reproducible matched-task benchmark with independent quality evaluation must precede numeric marketing claims. At 50% savings, a fixed inference quota supports twice as much equivalent accepted work; faster wall-clock completion is a separate, plausible but unproven benefit.

The hosted option is a deployment and convenience choice, not a different execution or governance product.

## 2. Product principles

- One engine, multiple placements. Local and hosted runtimes should execute the same versioned contracts and produce comparable receipts.
- Contracts constrain what must remain true; models choose how to make it true. Do not replace Work Engine's model-centered design with a rigid workflow merely to simplify hosting.
- Identity is not authority. Stytch authenticates a person or external client. Independent Work Engine owners decide whether a specific action is authorized, admitted, accepted, or publishable.
- Deterministic mechanics, bounded judgment. Scheduling, custody, accounting, replay, deadlines, and durable state belong to deterministic owners. Models handle contextual reasoning and decisions within their grants.
- No inference resale by default. Customers supply their own provider-paid API keys or provider-authorized OAuth connections. Devwits bills only its infrastructure services.
- No account requirement for basic local use. The Devwits account enables cloud connection and optional infrastructure, rather than being a prerequisite to running open-source code.
- No quiet expansion of authority or spending. Remote clients never acquire write or publication rights from login alone, and jobs cannot silently exceed grants or approved limits.
- Truthful maturity and claims. Distinguish designed, fixture-tested, privately piloted, and publicly deployed capabilities. Stopped and unestablished outcomes stay visible.

## 3. Product surfaces and boundaries

| Surface                                  | Responsibility                                                                                                                                 | Not responsible for                                                         |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Work Engine (Rust and local packaging)   | Admission, jobs, execution host, lifecycle, claims, reviews, gates, durable receipts, acceptance and publication contracts                     | Public user registration, SaaS payments, or provider-wide identity          |
| Devwits Web application                  | Signup/login, workspaces, repository/model/installation connections, dashboards, approval UI, infrastructure plans and billing                 | Manufacturing job authority or interpreting domain acceptance               |
| Devwits integration gateway (public MCP) | Authenticate clients, bind tenant/installation/workspace, authorize routing, submit typed requests, deliver observations and approval requests | Direct unrestricted shell access or independent authority to accept/publish |
| Local installation connector             | Outbound connection, authenticated polling or streaming, local job custody and responses                                                       | Trusting cloud requests without local admission                             |
| Hosted runtime adapter                   | Provision isolated workers/storage, meter usage, secure secrets, enforce runtime custody                                                       | Altering Work Engine semantic or approval contracts                         |
| Model connections                        | Provider-approved inference access, secret handling, identity and configuration provenance                                                     | Work Engine user identity or permission to mutate a repository              |

These boundaries could be maintained in two repositories or reorganized later; ownership contracts matter more than repository layout. The current `bline/devwits-auth` project is a useful foundation for the Web application, but it is a personal pilot, not yet a multi-user SaaS identity service.

## 4. Reference architecture

```
ChatGPT / Dot         Other MCP assistants       Browser / CLI / IDE
      \                      |                     /
       \                     |                    /
        +---- public HTTPS OAuth MCP + Web API ---+
                               |
                    Devwits integration gateway
                 identity / installation routing /
                  scoped request and event delivery
                               |
                 +-------------+-------------+
                 |                           |
         Local installation             Hosted runtime
        outbound connector             cloud worker pool
       (poll/stream; no inbound)        (isolated execution)
                 |                           |
                 +-------------+-------------+
                               |
                     Work Engine Rust owners
        admission / grants / lifecycle / job identity / receipts
                               |
                 deterministic Pi/Docker runtime host
                    model | tool | validation
                               |
            evidence -> review -> acceptance -> publication
                     (only under valid authority)
```

The diagram is a target composition, not a claim that the complete chain is deployed. The existing MCP-to-Pi vertical has executed a real bounded coding task with Docker and a receipt. The always-on Dot Events service is currently observation-only, and the hosted multi-tenant system does not exist.

### Proposed addresses

- `devwits.ai`: Public website, installation, documentation, benchmarks and pricing.
- `app.devwits.ai`: Workspaces, connections, jobs, receipts, approvals and billing.
- `auth.devwits.ai`: Stable sign-in and OAuth authorization origin, potentially retained during expansion.
- `mcp.devwits.ai/mcp`: Public OAuth-protected MCP gateway for Dot and future compatible clients.

These names are proposed, not deployed endpoints. Services need not be separate repositories or hosting deployments. Do not move `auth.devwits.ai` without validating passkey relying-party/origin configuration, OAuth metadata, callbacks, refresh behavior and active integrations.

## 5. Account, workspace, repository, model and runtime mapping

Create explicit stable identities rather than using repository names, email addresses, OAuth bearer tokens or physical machine names as the primary keys.

| Object                     | Suggested durable identity and relations                                                                               |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| User                       | Internal user ID; linked verified Stytch identity (issuer + subject)                                                   |
| Workspace                  | Internal workspace ID; owner/memberships and permission policies                                                       |
| Repository binding         | Workspace + provider + stable repository ID + provider installation/permission grant; local-only repository option     |
| Model connection           | Workspace/user + provider + credential type + owner + allowed runtime location + models/scopes + status                |
| Runtime installation       | Stable installation ID, local or hosted, keypair/credential, capabilities, version, health and workspace association   |
| External client connection | OAuth issuer, subject, client ID, scopes, resource/audience, workspace policy and revocation state                     |
| Job                        | Stable operation ID binding objective/plan revision, repo, installation, model connection, grants, limits and deadline |
| Approval                   | Exact decision bytes, subject/revision, signer, authority scope, freshness, consumed-once record and proof             |
| Receipt                    | Actual execution/configuration/evidence identity, terminal state and separate acceptance/publication state             |

Repository access: For GitHub, evaluate a GitHub App installed with explicitly selected repositories and minimum required permissions. Bind the stable repository and installation IDs; issue short-lived installation access tokens scoped to the job. A repository authorization is not equivalent to permission to publish a branch. GitLab, other Git hosts, and user-supplied local repositories can be subsequent adapters.

Model access: Distinguish the model provider connection from the OAuth connection that authenticates ChatGPT/Dot to Devwits. These are unrelated authorizations even if both involve OpenAI. Provider secrets should not be embedded in browser state, public MCP responses or receipts. Hosted execution needs encrypted credential custody, runtime-scoped delivery and revocation.

## 6. Dot and future external assistant integrations

### Primary route: public OAuth MCP gateway

ChatGPT can connect to a public HTTPS MCP server using OAuth. Consequently the standard Devwits integration would not require each customer to configure an OpenAI Secure MCP Tunnel.

A typical request path:

1. User installs or connects the Devwits MCP integration in a supported ChatGPT workspace.
2. ChatGPT/Dot completes Stytch OAuth authorization for the Devwits resource.
3. Gateway validates token issuer, audience/resource, subject, client and scopes, then resolves the authorized workspace and selected runtime.
4. Gateway records a typed request, binds idempotency and delivers it to the local outbound connector or a hosted worker.
5. The Rust Host independently checks the job profile, authority grant, limits and expiry before any effect.
6. Gateway returns a job identity/status immediately; long coding jobs execute outside the MCP request lifecycle.
7. Status, blockers, receipts and pending decisions become available through MCP and the Web UI.

Avoid a cross-tenant confused-deputy problem: user-supplied workspace or installation IDs are selectors, not proof of authority. The resolved connection principal and installation/repository binding must authorize the action at both routing and consumption.

### Local delivery without inbound ports

The local installation would initiate an outbound authenticated connection to the Devwits gateway. An initial durable queue with bounded polling is easier to recover and audit than a new persistent streaming protocol. Streaming or WebSocket delivery can be added for low-latency updates while preserving request identity and queue semantics.

If a machine is offline, the request remains pending, expires according to its policy, or is canceled; it must not be reported as running. Reconnection must not create duplicate execution. A runtime can be revoked, and queued work is rechecked against current authorization before disclosure or execution.

### Direct-private alternative

Keep OpenAI Secure MCP Tunnel as an optional route for installations that choose not to connect to Devwits. It supports a privately hosted MCP server without public ingress, but involves OpenAI-specific tunnel configuration and is not the default for a distributable public integration. A private-only installation could also expose its own HTTPS MCP server if the operator chooses.

### Integration interface contracts

Expose a small, stable set of capabilities rather than every internal Rust operation. Candidate functions include discovering authorized workspaces/runtimes, reading fleet/job status, submitting a bounded objective, querying blockers, retrieving evidence/receipts, and proposing/forwarding decisions. Separate observation and mutation scopes; require explicit current job authority for consequential writes.

An integration adapter translates, while semantic owners decide. The same gateway can serve future MCP assistants, editors, CLI clients and automation systems without giving each one a custom execution pathway.

## 7. Human approval and delegated operation

The standout remote experience is Dot supervising governed work while the human is away.

Two modes should be supported:

- Available: Dot reports observations and proposals. A consequential decision is presented in the Web app and the human signs the exact approved answer. Work Engine verifies it when consumed; successful mailbox delivery is not itself execution authorization.
- Unavailable, bounded delegation: The human grants a specific scope, budget, time limit and decision class. Dot may make permitted low-consequence decisions; decisions beyond the grant become pending escalations and block only the dependent work.

Examples of decisions that should remain separate in policy: ordinary route revision, investigation, budget increase, scope expansion, architectural change, acceptance, and publication. Human acceptance and publication authority remain the default. Any narrower delegated action needs an explicit owning-contract basis; neither OAuth login nor a model's confidence creates it.

Every decision should be attributable and revision-bound, with reasons, alternatives where appropriate, signature/grant provenance, and consumption result. Revocation must fence queued and retried effects as well as new requests. Never assume an approval signature automatically transfers to a different artifact revision.

## 8. Business model: charge only for infrastructure

Core pricing principle: The customer pays the model provider for inference. Devwits charges for optional persistent storage and container/worker execution, with a transparent operating margin in those infrastructure rates.

| Category                      | Customer billing concept                      | Operational note                                                                                                     |
| ----------------------------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Local/self-hosted Work Engine | No Devwits compute or storage bill            | User supplies their own hardware, storage and model access                                                           |
| Connected-only local use      | Free, fair-use or included tier — not decided | Public gateway, authentication, delivery and support still have real cost                                            |
| Hosted persistent storage     | Monthly rate per allocated GB/tier            | Covers provisioned capacity plus backups, metadata, retention and support margin; allocation changes may be prorated |
| Hosted execution              | Metered worker duration and resources         | Include CPU, RAM, startup/minimum billing, temporary storage and expected infrastructure overhead in published rate  |
| Model inference               | Paid directly by customer                     | No resale or hidden inference markup                                                                                 |

Conceptual bill:

`monthly Devwits bill = provisioned storage charge + sum(actual metered hosted execution charges)`

Rates should include a modest service margin that covers platform operations and the maintainer's time. Actual economics may require a small minimum hosted storage tier, fair-use limits for free gateway access, and charges built into the two published infrastructure rates for egress, backups and payment fees. Avoid pretending these costs vanish merely because customers don't run coding containers.

### Billing controls and evidence

- Customer-visible estimates and hard account/job compute caps, enforced before admission and during execution.
- Per-job usage receipts: worker class, start/stop, CPU/RAM or resource-seconds, storage tier, meter version, rates, charges, anomalies and canceled/failed outcomes.
- Clear handling for idle jobs, cold starts, retries, failures, unexpected resource use, disconnected credentials and budget exhaustion.
- No silent switch from an exhausted subscription allowance to paid inference.
- Reconcile cloud provider bills with Work Engine's independent metering before charging customers.

A failed job may still consume infrastructure, but the charge must be visible and governed by the customer's pre-authorized cap.

## 9. Provider-credential and OAuth eligibility boundaries

Bring Your Own API Key is the default portable inference path. OAuth only where that provider expressly supports the proposed use and execution environment.

- ChatGPT/Dot → Devwits OAuth: Authenticates and authorizes the external assistant to use Devwits MCP tools; it does not confer model-inference rights.
- OpenAI model access: An authorized API-key connection is straightforward in principle. OpenAI has a separate Sign in with ChatGPT / plan usage program for eligible open-source local tools and selected commercial partners. Commercial/remote hosted use must be qualified or authorized rather than inferred from a local pilot.
- Anthropic model access: API keys under the customer's provider agreement are the conservative default. Anthropic's published restrictions do not generally permit third-party applications to intermediate Claude.ai subscription OAuth tokens or route user subscription requests. Local unmodified Claude Code and explicitly permitted enterprise/provider paths have their own rules.
- Other providers: Add adapters and credential types only after verifying the provider's applicable contract, audience, scope, custody and revocation semantics.

The system should record which actual provider/auth mode a job used. Credential handling must preserve provider terms, tenant isolation, secret redaction and auditable expiration/revocation.

## 10. Hosting research before committing to a vendor

Fly.io is a plausible candidate, not a selection. Its Machines advertise usage-metered compute and provisioned storage, and Sprites may be interesting for coding environments. Evaluate it against alternatives with a real Work Engine workload.

A provider comparison should measure: cold/warm startup; effective CPU/RAM/runtime charges; persistent and ephemeral storage; snapshot/restore; network egress; multi-tenant isolation and secrets; no-privileges and filesystem protections; whether the current three-container custody model can be reproduced; cleanup and crash recovery; concurrency; region availability; observability; API quality; minimum charges; and portability.

Do not call a VM, Sprite or hosted container equivalent to the current Rust Docker Host until its actual isolation and receipt guarantees have been verified. Keep a provider-neutral runtime adapter and a provider-specific realization/qualification record.

## 11. User journeys

### A. Local developer without Devwits account

Install Work Engine; select or clone a repository; run a system doctor; connect a permitted model credential; run a bounded, governed job; inspect evidence and receipts locally. No public endpoint or registration is required.

### B. Local developer with Dot remote control

Create a Devwits account/workspace; pair the local installation using a one-time enrollment flow; link the repository; connect ChatGPT to the Devwits public MCP URL via Stytch OAuth; grant read access and bounded job permissions. Dot reads status, requests work, and escalates decisions through signed Web approvals. The laptop continues to own execution and secrets; no customer inbound port is needed.

### C. Hosted customer

Create workspace; connect authorized repository access; choose a persistent storage allocation; add a provider-approved model credential; select a hosted worker profile and spending cap; submit a job through Web, CLI or Dot; receive receipts and approval requests; pay only the defined Devwits storage and compute charges.

### D. Hybrid user

Keep a repository/workspace identity while selecting either local or hosted execution on a job-by-job basis. Migration of working state/checkpoints must be explicit, encrypted, provenance-bearing, and tested; it is not automatic simply because both runtimes exist.

## 12. Integration and release plan

The existing working MCP-to-Pi vertical is the reference implementation for integration work. Avoid a large rewrite and qualify each graft onto the real path.

| Stage                             | Deliverable                                                                                       | Must prove before advancing                                                                                      |
| --------------------------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 0. Baseline                       | Pin current pilot, evidence, Rust Host and current Devwits Web boundaries                         | Repeatable local coding job and truthful receipts; known gaps recorded                                           |
| 1. Local distribution             | `work-engine init`, doctor, configuration profiles, provider setup, installer and upgrade path    | New-machine first successful job without manual service/token surgery; no Devwits login required                 |
| 2. Multi-user identity            | Stytch production identity, accounts/workspaces, installation enrollment, connection-policy owner | User/tenant isolation, token expiry/refresh and revocation; no cross-tenant access                               |
| 3. Public MCP gateway             | Stable HTTPS MCP endpoint, OAuth discovery, scoped tool catalog, job queue and runtime routing    | Real ChatGPT OAuth, identity-bound requests, offline/reconnect behavior, deduplication, admission and revocation |
| 4. Governed Dot pilot             | Read-only remote control, bounded submission, Web signed approvals and receipts                   | Signed approval verified at consumption, unauthorized actions rejected, no silent retries or scope growth        |
| 5. Account/repo/model connections | GitHub App, model secret custody and multiple installations/workspaces                            | Correct provider binding, limited GitHub permissions, token rotation and no credential leakage                   |
| 6. Hosted runtime qualification   | Portable cloud custody adapter, storage plans, metering, hard caps                                | Equivalent effect isolation, cancellation, cleanup, durable recovery and reconciled charges                      |
| 7. Public beta and benchmarking   | Documented installation, pricing, transparent benchmark suite, support path                       | Reproducible quality-equivalent savings, security review and realistic operating margin                          |

These stages show dependency and validation boundaries, not a mandatory serial delivery schedule. The Rust integrations already planned—claims, real context continuity, review, acceptance and publication—should continue landing in the execution path in parallel where dependencies permit.

## 13. Public benchmark and product claim standards

The leading marketing claim should be accepted work per unit of inference, not raw token reduction or a faster-looking demo.

For a meaningful comparison, hold task set, repository revision, model identity, acceptance criteria, independent evaluation and repair accounting as consistent as practical. Compare against a defined native/traditional harness, not a weak strawman. Measure:

- per-task and aggregate inference use, including all planner, builder, reviewer, supervisor and retry turns;
- accepted completion rate, independent findings, escaped defects and rework;
- wall-clock time, distinguishing waiting, inference, deterministic execution and human approval;
- quota use where observable, separately from API-equivalent USD and cloud infrastructure billing;
- failure classes, distribution/variance, confidence intervals and task complexity;
- reproducibility on third-party machines and repositories.

Plan IR and model-aware execution strategy are future optimization hypotheses: the canonical contract may be rendered at different levels of explicitness for different demonstrated executor capabilities. Do not attribute their projected benefits to a pilot that hasn't exercised them.

Only publish a numeric percentage when supported by complete evidence and a stated benchmark scope. Quality equivalence means equivalent independently accepted outcomes and downstream obligations—not merely passing a selected test command.

## 14. Open product decisions

These should remain explicit questions for product/authority owners, not assumptions quietly inserted into implementation:

1. Free gateway policy: Is connected-only remote Dot access free under fair-use limits, included with storage, or a small paid tier? How are abuse and operating costs covered while retaining a simple two-charge public model?
2. Multi-user permissions: What are the minimal workspace roles and who may grant/revoke installation, repository, model and approval authority?
3. Storage meaning: What persists (checkpoints, worktrees, claims, logs), for how long, under which privacy/encryption and deletion/backup policies? Is storage charged on allocation or actual retained GB?
4. Billing meter: CPU/RAM time versus worker class × wall-clock, startup minima, concurrency, retries, data transfer and snapshot overhead.
5. Hosted provider: Fly Machines, Sprites or alternatives after empirical workload qualification.
6. Provider credentials: Approved OAuth availability for local versus hosted environments; secret custody and exposure in model/tool containers.
7. Integration distribution: Public ChatGPT plugin review and workspace eligibility; other clients' MCP/OAuth quirks; stable versioning and tool-discovery changes.
8. Repository relationship: Preserve `work-engine` and `devwits-auth` as bounded projects or consolidate once interfaces stabilize.
9. Operational responsibility: Support, abuse prevention, incident response, credential revocation, backup recovery, jurisdiction/data residency, payment disputes and compliance.

## 15. Proposed public positioning

Headline: Do more with the AI coding access you already have.

Subheading: Work Engine is an open-source, governed environment for AI-assisted engineering. Run it locally for free, supervise jobs remotely through ChatGPT/Dot with explicit approvals, or pay Devwits only for optional storage and hosted execution. Bring your own model access.

Three reasons to try it:

- Efficiency: A harness built to minimize unnecessary inference and recover useful state. Quantitative savings are published only when independently reproducible.
- Remote control without unbounded autonomy: Dot and future assistants can inspect, request and coordinate work while Work Engine enforces grants, spending limits, reviews and signed human decisions.
- Infrastructure choice: Same governed engine locally or in metered cloud workers. No mandatory hosted compute and no bundled inference resale.

The two strongest demonstrations are a matched-task inference/quality benchmark and a phone-to-Dot-to-Work-Engine approval flow that resumes a real job while the owner's laptop is elsewhere.

## 16. Current-state qualifications (November 6, 2026)

- A real bounded Events MCP → Rust CLI/server → Pi/Docker → receipt coding pilot exists; it is not a deployed multi-user service.
- The always-on Dot Events connection has been observation-only. Live public Dot coding submission, genuine context-pressure-based continued coding, integrated claims consumption, and automated receipt acceptance remain to be qualified.
- `devwits-auth` already has a Stytch-backed sign-in/consent UI, separate WebAuthn decision signing, a mailbox and laptop sync. Its README describes sign-ups disabled in the Stytch test project, with a single-owner design.
- The public OAuth MCP gateway, installation-routing service, general customer registration, payment system, and hosted job service are proposed work.
- The public GitHub `main` architecture may trail the latest local Rust development; use the current integration pilot's pinned receipts and executable checks for actual qualification.

## 17. References and external validation

Work Engine source material

- [Work Engine architecture](https://github.com/bline/work-engine/blob/main/ARCHITECTURE.md?utm_source=chatgpt.com)
- [Work Engine governing design](https://github.com/bline/work-engine/blob/main/DESIGN.md?utm_source=chatgpt.com)
- [Work Engine philosophy](https://github.com/bline/work-engine/blob/main/PHILOSOPHY.md?utm_source=chatgpt.com)
- [Connected Web/auth project](https://github.com/bline/devwits-auth?utm_source=chatgpt.com) (private)
- `MCP-to-Pi Vertical.md`, November 6, 2026, supplied in this conversation
- [Structural Plan IR hypothesis](https://github.com/bline/work-engine/blob/main/app-server/ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md?utm_source=chatgpt.com)

External documentation to recheck before implementation and launch

- [OpenAI: Add custom MCP server](https://developers.openai.com/api/docs/guides/custom-mcp-server?utm_source=chatgpt.com) — public HTTPS, OAuth and write-capable tools
- [OpenAI: Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels?utm_source=chatgpt.com) — private alternative; not needed for public MCP ingress
- [OpenAI: Sign in with ChatGPT for open-source apps](https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt?utm_source=chatgpt.com) — local open-source usage and commercial-access limitations
- [Anthropic: Claude Code legal and compliance](https://code.claude.com/docs/en/legal-and-compliance?utm_source=chatgpt.com) — restrictions on third-party subscription token intermediation
- [GitHub: Installation access tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-an-installation-access-token-for-a-github-app?utm_source=chatgpt.com) — repository/permission-scoped short-lived tokens
- [Fly.io current pricing](https://fly.io/pricing/?utm_source=chatgpt.com) — candidate infrastructure economics, not a vendor decision

One-sentence product definition: Work Engine is the open-source governed execution environment; Devwits is its optional account, integration and infrastructure service, connecting Dot and future assistants to local or hosted runtimes while users retain control over model access, spending and consequential decisions.
