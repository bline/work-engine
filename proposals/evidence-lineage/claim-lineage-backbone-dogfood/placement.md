# Placement: Claim-Lineage Backbone Dogfood

## Experimental placement

The probable first placement is a reversible Git-backed dogfood surface adjacent
to the evidence-lineage proposal family. That location keeps the experiment
near its semantic hypothesis and makes immutable repository and review subjects
easy to reference. It is not a permanent shared-evidence placement decision.

The canonical dogfood records and the query projection remain distinct. The
records preserve the exercised claim revisions, impact nominations, refresh
judgments, reliance, evidence, provenance, and authority references. A local
in-memory, SQLite, or similarly replaceable projection may answer the bounded
proof queries if it can rebuild deterministically and expose its source and
completeness boundary.

The executing planner may choose the smallest faithful physical layout within
this experimental boundary. The proposal does not mandate a database, a graph
store, a service, or a production package merely because those are possible
realizations.

The current proposal-packet capability recursively treats every `packet.json`
under its repository root as a proposal manifest. Dogfood records placed near
the proposal family must therefore use an unmistakably experimental namespace
and filenames that cannot enter proposal-packet discovery accidentally. An
authorized execution should demonstrate in a disposable checkout that adding
and removing the experimental machinery does not change proposal-packet or
review-artifact meaning or validation. Historical evidence records may remain
when their owners and non-production status are explicit.

## Ownership boundaries

- The proposal or review workflow owns the original semantic judgment.
- Git, packets, reviews, checkpoints, and completion receipts retain their
  existing canonical facts.
- Dogfood-local lineage records reference those facts and own only the
  experimental representation under test.
- An authorized refresh judgment owns unchanged or changed semantic
  reconciliation for its exact fixture claim.
- The projection owns no claim meaning, causality, authority, or completeness
  beyond its declared inputs and watermarks.
- The downstream proposal or review decision owner retains reliance and
  reopening authority.

## Rejected assumptions

### Put the history inside `packet.json`

Rejected as an assumed route. Proposal packets own proposal identity and current
lifecycle metadata. Expanding the version 1 manifest would conflate packet
state with independently revisioned evidence history before the experiment has
shown a shared contract.

### Make review artifacts the shared owner

Rejected as a permanent conclusion. A real review finding is a required
dogfood subject, but review-specific episode and synthesis semantics do not
therefore own proposal-research claims.

### Select a production graph or database now

Rejected as premature. The first query set is deliberately small. A replaceable
projection is enough to expose whether graph-shaped relationships are useful
without promoting an index into canonical authority.

### Integrate scheduler, control plane, or role state

Rejected from this experiment's boundary. Those systems may later deliver or
rely on claim consequences, but they neither own epistemic history nor need to
be changed to prove the four backbone behaviors.

## Placement evidence to preserve

The dogfood report should distinguish:

- fields and edges used identically by research and review;
- fields that remained domain-specific;
- queries that required projection support;
- any canonical-history behavior that Git-backed local artifacts could not
  preserve truthfully; and
- any pressure that would justify a different semantic owner or physical
  adapter.

Those observations may reopen the parent placement question. They do not settle
it automatically.
