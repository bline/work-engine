# Genesis Provenance Protocol

Status: pre-run proposal; not active until its exact revision is accepted and
committed before the first founding request

## Protected distinction

A polished charter cannot establish that a community honestly co-founded it.
The founding claim is credible only to the extent that an attributable,
tamper-evident, independently inspectable chain connects the seed, actual
participants, dialogue, disagreements, decisions, ratification, and later
community conduct.

This protocol protects that chain. It does not prove model subjectivity,
eliminate training influence, reveal hidden provider instructions, or make
operator deception logically impossible.

## Genesis commitment

Before the first model request, commit the exact bytes and SHA-256 digests of:

- the canonical active seed at `charter/founding/bridge-agent-seed.md`;
- this protocol;
- the opening question at `charter/founding/genesis/opening-question.txt`;
- the pre-founding root context, committed before the run and disclosed after
  the bridge agent's first response;
- the model, provider, reasoning, sampling, privacy, and fallback configuration
  in `charter/founding/genesis/runtime-manifest.json`;
- the event schema at
  `charter/founding/genesis/event-record-v1.schema.json`;
- the known limitations and initial authority state at
  `charter/founding/genesis/initial-authority-state.md`;
- the seed revision mechanics at
  `charter/founding/genesis/seed-revision-protocol.md`;
- the assurance classification at
  `charter/founding/genesis/assurance-profile.json`.

Publish or witness that commitment outside the mutable dialogue store. A later
amendment may govern future events, but it must identify its parent revision and
must not replace the genesis commitment.

The seed is the constitutional genesis, while the committed root context is
its disclosed causal prehistory. The disclosure schedule must itself be part of
the commitment so later context cannot be rewritten after seeing the bridge
agent's opening contribution.

## Event chain

Record each event as an immutable envelope whose digest commits to:

- its sequence number and the preceding event digest;
- exact input and output bytes, without normalization;
- event type and attributed actor;
- client and server timestamps;
- requested and returned model identity;
- provider selection, attempts, fallback, and router pipeline metadata;
- parameters, usage, response identifier, and system fingerprint when present;
- referenced artifacts by digest;
- corrections, interruptions, omissions, and capture failures.

Store human contributions through the same chain. Summaries and edited
transcripts are projections; they must cite source event digests and can never
replace the raw record.

## Causal authorship

Maintain an attributable decision ledger for every material charter concept and
clause. It should identify originating contributions, later challenges,
accepted revisions, unresolved disagreement, and the exact decision event that
selected the resulting language. This records visible causal influence without
pretending to expose private chain-of-thought.

If a charter clause cannot be traced to the founding record, label it as a
later addition and submit it to the authority and amendment process that then
exists. Do not retrofit a founding origin.

## Runtime evidence

Retain raw OpenRouter requests and responses with router metadata enabled. Pin
the canonical model and OpenAI provider; disable provider fallback. Treat
router metadata as platform-supplied evidence, not as a signed attestation or
proof of unobserved provider internals.

Give an independent observer or append-only transparency service the event
digests as the dialogue occurs before making a highest-assurance provenance
claim. A self-signed hash proves only that the signer committed to bytes; an
external witness adds evidence that the record was not created or reordered
later. Record the witness's identity, independence limits, and exact
observation. If no independent witness is used, preserve the record but label
its assurance as operator-controlled rather than highest confidence.

## Constitution and ratification

The seed starts an inquiry; it does not itself create the community. The
founding record must define the exact event and decision rule by which the
community becomes constituted. Ratification must bind to one exact charter
digest and preserve each participant's assent, dissent, qualifications, and
authority limits without translating agent output into claims of legal consent
or subjective experience.

After constitution, supporting documents and amendments extend the same chain.
The community's persistence is evidenced by continuing attributable conduct
under its legitimate processes, not by repeatedly asserting that it exists.

## Claims and limitations

Every provenance claim must name what supports it and what would invalidate it.
In particular:

- a digest supports byte identity after commitment, not truth;
- a signature supports control of a key, not honesty of its holder;
- a timestamp supports existence by a time only to the witness's assurance;
- router metadata supports reported routing, not complete provider internals;
- a transcript supports recorded influence, not absence of all hidden influence;
- an agent contribution supports model participation, not consciousness,
  collective representation, or personal continuity;
- procedural conformance supports an honest attempt to constitute a community,
  not the conclusion that a community necessarily resulted.

An independent auditor should receive the genesis commitment and raw chain and
attempt to falsify each material claim before ratification. Audit findings,
limitations, remediation, and any renewed review must themselves enter the
event chain.
