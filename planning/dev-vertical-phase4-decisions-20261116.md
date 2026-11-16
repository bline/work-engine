# Phase 4 operator decisions, 2026-11-16

Decisions on the [phase 4 design](dev-vertical-phase4-hosted-transport-20261116.md),
received from the operator in the Claude takeover session.

1. **Profile strategy:** the design is approved as proposed. A new versioned
   hosted profile is added; existing versioned profiles are not changed in place.
2. **Retirement:** the old credential-mounted execution paths are removed after
   the hosted profile is qualified. The operator prefers removal to permanent
   legacy support.
3. **Logical request versus funding attempt (binding requirement):** a failover
   must not create a second coding request or duplicate its tool effects. Each
   provider attempt has its own identity and financial reservation underneath
   the original logical request. The financial owner must authorize any
   release, replacement or new grant.
4. **Raw terminal event versus raw stream (binding requirement):** capturing the
   terminal event on the host satisfies the existing settlement contract only.
   It does not satisfy the earlier requirement to preserve the complete
   screened SSE stream. That requirement needs protected evidence or an
   explicit acceptance decision narrowing it.
5. **Phase 3 standing:** its results stand. Two items remain qualification
   work, not reasons to discard them: an authenticated registry/sealed-reader
   join for recovery, and cleanup of containers orphaned by a crash.
6. **Order of work:**
   - (a) Build a host stream state machine as an event-driven component,
     including cancel while awaiting response headers, and verify it
     independently.
   - (b) Integrate the hosted profile with the Pi coding lifecycle and
     financial admission. Run the offline Docker coding workflow with no
     network and no credential mount in the inference container.
   - (c) Qualify the integration: failover, crash recovery, cancellation,
     accounting, checked exports and independent evidence on the exact
     integrated source.

   Hold a focused design review before (b).
