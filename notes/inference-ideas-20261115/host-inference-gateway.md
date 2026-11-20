# Trusted host inference gateway

Status: idea extraction, 2026-11-15. Source: [scratch notes](../scratch.md),
lines 46–301, especially the development-model passage at 195–217. Provider
and implementation observations are reported by the source, not newly verified.
See the [collection index](README.md) for provenance and attribution.

## Idea

Run inference through a trusted host boundary while the Pi worker constructs
requests and performs engineering work without possessing provider secrets.
The gateway would attach credentials, refresh OAuth, dispatch authorized
requests, stream responses, and publish observations to existing accounting
and lifecycle owners.

The source first sketches an application-level inference proxy with job-scoped
worker authorization. It then refines the idea: retain the native Pi framed IPC
boundary and inherited channel initially. That could avoid adding container
networking, TLS interception, or another bearer credential inside Pi. A generic
HTTPS CONNECT proxy is distinguished from a gateway capable of inspecting
requests and attaching upstream authentication.

The proposed boundary keeps API keys, access tokens, and refresh tokens with the
host. It validates requests against the admitted job, removes worker-supplied
authorization, and constructs permitted upstream destinations rather than
accepting arbitrary worker URLs.

## Ownership and integration

The gateway is the shared execution point for credential access, admission,
routing, streaming, and observations. It composes their semantic owners rather
than creating a second budget ledger or deciding independently whether an
attempt is settled.

The source distinguishes three decisions:

| Decision | Proposed responsibility |
| --- | --- |
| Credential selection and refresh | Host mechanics within the selected connection and authorization |
| Billing-route selection | Authorized policy, resource availability, limits, and compatibility |
| Model or provider substitution | Execution contract and the owner of affected quality, capability, or independence requirements |

Changing who pays is therefore a different operation from changing which model
does the work. Even a billing-only switch needs compatible request features and
continuation state; matching model names alone is insufficient.

## Reported starting point and missing work

The source reports that the newer `pi-native.mjs` / `pi_native.rs` path runs Pi
without container networking, intercepts SDK fetch calls, and uses framed IPC.
It describes `pi_native_probe.rs` attaching synthetic host-only authentication
before forwarding to a synthetic endpoint. It explicitly treats this as an
offline, buffered demonstration rather than production streaming qualification.

It contrasts an older live Pi coding profile that stages a short-lived
credential and permits inference-container networking. These paths must not be
treated as interchangeable evidence of isolation or readiness.

The proposed additional work includes real credential custody and refresh,
request authorization, streaming with cancellation and backpressure, and
authoritative attempt accounting. The conversation describes both a synthetic
foundation and a narrow live route before broader automatic failover; it does
not settle a new implementation plan.

The reported possibility of using OpenAI's public Responses API for both API
billing and eligible plan usage could simplify the transport. The source
qualifies this with registration, granted permissions, model availability,
request-field restrictions, and uncertain interchangeability with existing
Codex tokens. Commercial or remotely hosted eligibility is also left for
qualification.

Related: [resource management](inference-resource-management.md),
[attempt evidence](usage-and-attempt-evidence.md), and
[qualification experiments](qualification-experiments.md).
