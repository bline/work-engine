# Bookkeeping host: reviewer budget diagnosis, 2026-11-03

The two reviewer overruns in the coordinated phase1 run do not establish that
the host ignored a pre-execution hard spending control.

The first review started at22:11:35.806Z and completed at22:17:32.338Z. Its
$4 allowance was a planning allocation. The canonical record added a node
control only at22:22:51Z, after completion, at $4.25 with a $4 mark and an
explicit historical overrun reason. Actual reviewer inference was $4.127758.
The root inspected its cost at $3.248502 and later asked it to conclude, but
that was advisory steering. No earlier configured $4 node ceiling is established.

The fresh repair review started at22:52:52.421Z. Its canonical $0.75 node limit
and $0.50 mark were added at22:53:21Z, after native execution had begun. The
review completed at22:54:16.433Z and cost $1.418312. A configured control added
after launch does not constrain earlier calls; delayed observations and a
foreground notification do not interrupt provider inference. The host's current
contract returns observations to the root and leaves interruption judgment there.
It does not supply a hard provider billing cap. This observation does not prove
whether an earlier interruption would have met the intended review contract.

The repair exposes the recorded canonical `cost_controls` and matching child
`role_controls` in compact inspection and receipt summaries. An absent matching
control is labelled `none_in_recorded_snapshot`; it is not inferred from a
prose allowance or copied from another role. Closed snapshots may have no active
controls, so their absence does not erase historical controls. Existing budget
records remain the owner of limits, revisions, scope and cost; no second ledger,
automatic budget increase or native interruption policy is introduced.

Pre-start per-role control binding and provider-side hard caps are not implemented
by this repair. Any future mechanism must preserve admission identity, canonical
limit ownership, unknown effects, accounting delay and root authority. The three
current repairs address final acknowledgment, compact evidence consumption and
authorized output-directory preflight.

Evidence: `/home/bline/.local/state/devwits-auth-rollout-20261102/session-management-phase1/assignment.json`
(`scope_budget.cost_limits` and actor allocations),
`manual-host-review/state.json` (operations and captured controls),
`RESULT.json` (cost and overruns), and the two reviewer rollouts referenced there.
This diagnosis reads those historical records and changes none of them.
