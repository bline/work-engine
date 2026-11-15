# Selected task presentation

`work-engine.selected-task.v2` is a private read-only presentation produced by
Rust from independently qualified TO2 components. It owns presentation and exact
task → input → native session matching; task, lifecycle, native delivery and
Events remain the semantic owners. The existing session-v1 projection is unchanged.

Select an absolute Unix endpoint and explicit task ID:

```sh
work-engine-observe selected-task /private/host.sock task:chosen \
  --delivery delivery:chosen --profile /private/events-profile.json \
  --output /private/selected.json --interval 4 --human
work-engine-terminal-ui --selected-task /private/selected.json /private/host.sock task:chosen
```

The task read has one five-second deadline. Optional session reads are sequential
and use the existing bounded Events client. Polling starts after the previous
collection completes. The collector rereads the protected profile each cycle;
changed bytes clear retained task anchors before the new source is used. The
source ID hashes exact profile bytes, or `no_events_profile` when omitted.
Invalid configuration stops collection. A failed task read retains compatible
anchors with their original identity and time, while activity linkage becomes
unestablished. Output replacement is atomic and mode 0600; it is not an owner
transaction. No mutation, provider request or retry of an effect is available.

`--replay` reads a protected captured TO2 envelope. It always emits
`fixture_replay`, with unknown task read time (zero), even when parsed today.
Socket reads emit `new_endpoint_read`; that label establishes neither successful
native execution nor production qualification. U2's first installed-host
attempt failed before native spawn because the installed code-mode companion
did not match the lifecycle owner's pin. Historical captured success remains
historical. A later finite test with the exact privately retrieved qualified
0.163 package reached native completion and the CLI/TUI question/answer/result
journey. Its wrapper killed four leftover Git descendants; destination and
external contact were not captured, so natural descendant closure and network
confinement remain unestablished. These observations do not establish the full
three-client outcome. Browser authentication and transport require separate
qualification.

The [generated schema](selected-task.schema.json) describes DTO structure.
Regenerate it with `cargo run --locked -p operator-observation --example
selected_task_schema`. Rust `SelectedTask::validate` and the JavaScript envelope
validator additionally enforce version, bounds, timestamps and scope digests;
schema validity alone establishes no authority, provenance or valid owner join.
The checked fixture and schema-generation test bind the consumers to these DTOs.

The closed envelope includes endpoint/request, source and scope IDs, host
identity, standing, confidentiality, assembly time, transport, five independent
components, activity and linkage. Scope ID hashes compact sorted-key JSON
`[endpoint,request,source_id,host_identity]`. It changes on established endpoint,
task/target, source/account-profile or host identity changes. It is a presentation
identity, not an authenticity signature or grant. The browser recomputes it.

Each component preserves its TO2 state, scope, revision, completeness and local
read-completion time. `present`, `absent`, `not_requested`, `not_applicable`,
`unavailable` and `refused` remain distinct. Rows are bounded, sanitized excerpts
of owner data, not a second owner payload. A stale row retains original
`last_known_scope`, `last_known_revision`, `last_known_completeness` and
`last_known_at_ms`; latest failure scope and time never replace that provenance.
Native zero-row completeness differs from a 65-row capacity refusal.

Read time means completion of that source read, not event time, latest owner
mutation or an atomic cross-owner cutoff. Assembly or mailbox fetch time cannot
renew it. Consumers reevaluate age with their own clocks; zero/future timestamps
are unknown/stale and age over 120 seconds is a display warning. That warning
grants no authority and does not expire a domain record. CLI prints age, the TUI
redraws continuously and reconciles the file every second, and the browser
reevaluates age with its own clock. Mailbox cadence belongs to the dashboard and
is independent of the collector.

Activity requires exact owner task/input membership, matching selected delivery
when supplied, and a native thread ID accepted by the existing session contract.
The source profile must match. Subject equality, timestamps, transcript prose
and selected command text cannot create edges. Entries are session-wide; no
per-entry turn or task-command association is inferred. Campaign, review,
limits and scheduler associations stay explicitly unestablished.

All exported data is `private_operator_data`. Structured sensitive JSON keys and
recognized credential tokens are redacted before task rows are serialized;
Events owns transcript redaction. Sanitization is defense in depth and cannot
classify arbitrary work prose as safe for publication. Endpoint/source/scope
and excerpts remain private. Browser delivery is opt-in through the existing
authenticated mailbox; sign-out/account change/authentication failure clear
displayed data, and old request generations cannot overwrite a new selection.

TUI Enter inspects the selected questions, result, component or activity route; `b` returns, arrows select routes,
PageUp/PageDown scroll, `r` refreshes and `q` exits. Identity changes reset
inspection. Ordinary file failures retain original data visibly stale; a
confirmed endpoint/task mismatch clears it. Browser selection requires exact
endpoint/task and resets on scope changes. U3 notifications, replay/recovery
and a five-second source-to-screen target remain unqualified.

## Bounded repair after U2 review

The private v2 presentation envelope now requires `activity_coverage`: state,
exact sorted owner-linked session IDs and Events limit 20. This is an Events
read disposition, independent of the native complete-for-scope limit 64.
`refused_capacity` publishes no activity records and lists the entire linked
scope; no sessions are silently selected or omitted into a successful read.
`read_completed` covers every linked session, each of which may independently
be unavailable. `not_requested`, `unlinked` and `unavailable` do not assert an
empty transcript. Task components keep publishing during activity refusal.
No deployed compatibility contract is claimed for the amended private envelope.

Human CLI latest-read diagnostics and retained rows are separate: retained
rows show original read time, scope, revision and completeness. Repeated owner
failures do not refresh those anchors. Question identity comes from owning
`Question.id`; the owner-shaped fresh completed capture is a regression control.


## Compact owner summary (S1)

The producer and CLI/TUI/browser/mailbox readers use v2 together. Old or unknown
presentation versions refuse explicitly; there is no downgrade or v1 emission
flag. Historical v1 records remain evidence, not a supported current envelope.
TO2 itself remains protocol 2.

Rust derives `summary` from the chosen typed owner observation, independently of
native/session completion. The owner has no generic task status. Objective,
engineering question, pending questions, recorded answers and attributed result
are independent facts. Pending questions may coexist with a published result.
Question identity is owning `Question.id`; explicit `result:null` means no result
published in that observation, while a missing or malformed result makes the
compact summary unavailable. No transcript parsing or browser-side owner join
establishes these facts.

`summary.basis` identifies `latest_owner`, `last_known_owner`, or `none`. Current
values use owner scope/revision/completeness/read time; retained values use only
the original last-known metadata. A whole transport failure does not observe a
new component read time. Neither consumer age nor mailbox fetch renews anchors.
Malformed, missing, unsupported, or over-capacity summary fields leave all five
component dispositions available for inspection, never inferred empty.

There are at most 64 questions. Text excerpts cap at 1024 Unicode scalars,
result body at 4096, and IDs/references/attribution at 512. Recognized secrets
and controls are sanitized across the full string before clipping; every clipped
excerpt has `truncated:true` and a visible cue. Overview previews have separate
visual continuation cues. Sanitization remains private defense in depth.
The complete envelope retains its 1 MiB publication bound and fails visibly if
too large; bounded evidence does not promise omitted source content.

CLI `--human` defaults to a compact summary. For example:

```sh
work-engine-observe selected-task /private/host.sock task:chosen --human --inspect questions
```

Inspectors are `questions`, `result`, `owner`, `lifecycle`,
`native_delivery_status`, `command_result`, `control_result`, and `activity`.
Raw mode remains the v2 DTO. TUI arrows/Enter and browser Inspect buttons expose
the same typed facts and existing sanitized evidence. Back retains selection;
owner basis identity/revision changes reset inspection. Activity is separately
qualified and session-wide, never task success. S1 changes no source polling,
read lifecycle, authentication, or runtime owner.

## OwnerBasisCoherence and finite credential syntax

Rust construction/parsing and JavaScript DTO verification enforce the same
within-envelope relation, including unavailable summaries. A fresh present owner
requires completed transport, latest-owner basis and a full retained tuple equal
to its scope, revision, completeness and original read time. A stale present
owner requires unavailable transport, last-known basis and the same equal tuple.
A latest refused/unavailable observation may retain a qualified older present
anchor with equal scope, or with the same task and omitted latest subject ID.
Its failure revision/time need not equal the anchor. An explicit different
subject cannot retain it. With no usable owner/anchor, basis is none, summary
is unavailable/no_owner_value, retained metadata is null and owner rows empty.
Absence clears anchors; later transport failure cannot restore them. Zero read
time remains unknown. This relation cannot attest historical continuity, owner
authenticity or summary derivation; producer transition tests cover continuity.

The shared Rust sanitizer recognizes ASCII identifier keys (letters, digits,
underscore and hyphen), optionally single/double quoted, followed by optional
whitespace and colon/equal. Matching is ASCII case-insensitive with hyphen
normalized to underscore and sensitive-family substring matching: password,
secret, token, api_key, authorization, credential. Spaces within key names and
encoded keys are outside this finite syntax. Structured JSON sensitive-key
redaction remains unchanged. A recognized assignment consumes a complete
single/double quoted value, with backslash escaping the following character;
an unterminated quoted value consumes the rest of that source string. Unquoted
values end at whitespace, comma, semicolon, ampersand, closing brace or bracket.
Bearer plus its quoted/unquoted credential is consumed together, including
standalone Bearer. Existing provider-token recognition remains. Sanitization
precedes clipping for typed excerpts and inspection rows. Arbitrary confidential
prose and exotic encodings remain outside this defense-in-depth claim.

The checked consistency expectation matrix is independent of validator results;
Rust emits decoder/Selection-derived positives and single-relation mutations
against it, and JavaScript consumes those exact serialized vectors.

## Owner metadata in coordinated v2

Only owner metadata uses canonical ASCII decimal strings for the full unsigned 64-bit domain (0 through 18446744073709551615). Present and retained revisions are closed objects containing `kind: "task"`, `owner_revision`, and `cursor_revision`. Zero is `"0"`; leading zeroes, signs, whitespace, fractions, exponents, non-ASCII digits, overflow, numeric JSON and additional fields are refused. TO2 decoding and storage stay numeric; other components retain their distinct metadata. Old numeric owner metadata is explicitly unsupported in this coordinated v2 contract.

Owner scope is a closed object with the requested string `task_id` and optional `subject_id` (string or null). Omitted and explicit-null subjects remain distinct: omission may retain a same-task older anchor; an incompatible explicit subject clears it. Retained present anchors require a string subject. Completeness is null. Latest absent/refused/unavailable revision is null, independently of a retained older revision. Extra scope/revision metadata is refused rather than silently discarded. An owner-approved extension requires revisiting its lossless representation.

The sole missing-task exception is the exact unobserved sentinel: empty scope, unavailable/stale owner, `transport_unavailable`, zero read time, null revision/completeness, no anchor or rows, `none/no_owner_value`, and unavailable transport on completed publication. Initial selection has no host identity. Explicit task identity is checked even without a value or anchor. These checks establish within-envelope consistency, not historical authenticity.

TO2 uses private strict task decoding to retain exact numeric task integers, including nested owner data. It rejects duplicate decoded names and trailing input. Only present owner revision/value use the task integer domain; all other fields retain lifecycle interoperable numeric qualification. The lifecycle default parser is unchanged. Presentation converts owner metadata to canonical full-u64 decimal strings before JavaScript parses JSON. Read preservation does not prove mutation reachability or advancement at u64::MAX.
