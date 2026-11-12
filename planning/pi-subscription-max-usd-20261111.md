# Pi subscription container `--max-usd`

Status: implementation plan, November 11, 2026. The user requested this feature after the subscription-container release at `45155c02`.

The existing API-container dollar limit reserves a request's bounded worst case before forwarding. The native Codex subscription transport has no enforceable output-token limit. Its new opt-in `--max-usd` behavior therefore uses a clearly labelled **API-equivalent cost-proxy stopping threshold**. This does not establish subscription billing, quota consumption, or a hard dollar bound on an in-flight response.

Only one request may be in flight under the subscription threshold. After its usage reaches the threshold, no further request is forwarded. An admitted response can exceed the threshold. Missing required usage, uncertain completion or response identity, and incomplete admitted requests prevent further requests and retained delivery. Durable admission evidence must precede upstream effects so a crash cannot erase an unresolved request. Accounting must preserve precision across turns and refuse malformed or partial evidence.

The host adapter owns launch selection and fixed rate provenance; the host egress proxy owns request admission and observation; status projects that record; the usage ledger preserves its units. Subscription billing and quota remain unknown, and these calls stay in the Codex subscription pool. Existing API reservations retain their semantics. Subscription output-token caps, strict API-tier reservations and service tiers remain unsupported. The existing `--objective-record` API-dollar exposure admission cannot be combined with subscription containers; that combination fails before launch rather than reinterpreting its authority.

Implement within the existing Python adapter. This is a bounded change to the active owner, not a new first-party executable or a general budgeting framework. No Rust migration is needed to add this control.

Validation covers concurrency, threshold crossing, one-response overshoot, unknown usage, failures and interrupted streams, durable continuation after process loss or partial logs, invalid launch options, status/ledger unit separation and API regressions. Use synthetic credentials and local mocks; no live provider request is part of this objective. A separate read-only reviewer challenges the composed candidate before acceptance.

Root records a supervisor-set $8 speed-adjusted API-equivalent proxy ceiling, with a $5 checkpoint, for implementation and review. That engineering allowance has different units from this feature's default-tier API-equivalent valuation. Publication, skill deployment, paid API calls and R-2 integration/distribution are outside this task. Working authority, exact source, budget and collection bindings live at `/home/bline/cbm-work/pi-subscription-max-usd-20261111/assignment.json`.

The user explicitly selected “Use the cost-proxy stopping limit (recommended)” in the preference reply. This confirms the threshold semantics above.
