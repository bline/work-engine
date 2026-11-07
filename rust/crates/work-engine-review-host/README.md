# Review and completion host

`work-engine-review-host` is the installed executable for both the qualified
review workflow and publication-only completion. `completion-bootstrap` and
`completion-serve` accept the same `--startup ABSOLUTE_JSON` and optional
`--job-request ABSOLUTE_JSON` as review startup. Completion always reopens the
existing installed admission; it never admits a new job or launches review.

The production build has no feature flags. A build with `test-support` or
`test-faults` cannot activate installed completion. The older
`work-engine-completion-host --fixture-config` executable requires its explicit
`fixture` feature and retains fixture standing.

## Installed completion selection

The deployment owner installs the exact reviewed JSON at
`/etc/work-engine/review-host/completion.json`. The file and every ancestor
must be root-owned with no group/other write permission, and the path must be
canonical. JSON is strict, with no request override or fixture selector:

| Field | Meaning |
| --- | --- |
| `schemaVersion` | `1` |
| `startupDigest` | SHA-256 of the exact qualified startup document bytes |
| `runtimeSha256` | Exact installed review-host executable SHA-256, also selected by startup and custody |
| `completionRoot` | The existing campaign owner's root plus `/publication` |
| `workspaceRoot` | Exact publication workspace owner root selected by custody |
| `repository` | Exact managed canonical bare repository selected by custody; immutable candidate objects must be available there |
| `custodySha256` | SHA-256 of the exact fixed installed custody selection bytes for this phase |
| `socket` | Absolute Unix socket under a canonical runtime-owned private directory |
| `operatorUid` | Independently selected operator UID checked from the connected Unix peer |
| `authoritySourceDigest` | Exact claims authority-source file SHA-256 already pinned by qualified startup |

Constructing a `CompletionSelection` or validating its proposal shape supplies
no installed standing. The workflow also opens the existing private qualified
acceptance, campaign, evidence reader, claims and episode owners. The socket
directory must be runtime-owned with no group/other permission; ancestors
must be non-writable by group/other and owned by root or that runtime UID.
Untrusted reviewer and validator descendants must have no grant to the socket,
owner roots, canonical repository or deployment selections. An unrestricted
same-UID agent does not establish operator isolation.

## Bootstrap, activation and cold serving

These commands describe the implementation interface, not an installation
grant. Before installation, bind the exact production binary/source/build,
qualified startup/admission, claims/episode/evidence inputs, genuine candidate,
managed repository/ref, validator policy and runtime/operator identities in an
operator-approved packet. Existing preparation output is data until the owning
authority issues its grants. Credentials and live-provider qualification need
their own authority.

1. The deployment owner installs protected startup/authority inputs and the
   exact binary, allocates the approved managed repository, owner roots and
   private socket directory, and observes their filesystem identities. The
   review owner initializes the campaign through the existing installed
   acceptance or installed job policy. Completion does not replace that step.
2. Install `/etc/work-engine/workspace-publication/custody.json` with
   `phase: "initialize"`, exact runtime/repository/root/permanent-lock identities,
   validator and ref policy. Install completion JSON bound to that custody hash
   and exact startup. As the selected runtime UID, run
   `work-engine-review-host completion-bootstrap --startup ABSOLUTE_JSON`
   (and `--job-request ABSOLUTE_JSON` for a derived job admission). This allocates
   publication-owner state only. Its output includes owner root/repository IDs
   and `active: false`.
3. Observe the resulting marker/database identities. The deployment owner
   separately approves and installs custody `phase: "active"`, sealing those
   identities. Install matching completion selection bytes with the active
   custody SHA-256. Preserve the initialize bytes and epoch history.
4. After the review owner has committed the selected initial outcome, run
   `work-engine-review-host completion-serve --startup ABSOLUTE_JSON` with the
   same optional job request. It derives the registered result transition and
   evidence reference from durable campaign state, opens the exact owners once,
   opens active managed custody and the publication store, and binds the socket.
   It does not write/synthesize an episode result or dispatch another reviewer.
   Missing review outcome requires review-owner recovery before serving.

Every decoded socket request revalidates qualified startup and the exact
installed completion selection before execution. The workspace owner retains
its installed custody/currentness checks and operation/CAS/unknown-effect
recovery. A pre-existing socket is refused and never automatically removed.
Stop/collect the old host and establish that it is no longer serving before an
operator removes a stale socket and starts a cold process.

Use the existing `work-engine-completionctl` to inspect the package. The
operator's `accept-checkpoint` decision binds its displayed digest and grants
publication-only eligibility. Preparation/offer display does not authorize
creation; the operator separately chooses `create` over the exact displayed
offer digest. Neither decision accepts or settles the coding job. Published
state must be confirmed from the publication owner and cold reopened, with
actual descendant confinement independently observed; source fixtures are
insufficient for installed production qualification.

## Rollback and unresolved effects

Before an admitted publication attempt, the deployment owner may stop/collect
the host and revoke/remove its protected selection. Preserve all source/binary,
selection bytes, owner roots, permanent lock and epoch history. After admission
or uncertain effects, do not retry creation, reset refs, regenerate markers,
replace the permanent lock or delete state. Reopen the identical approved owner
configuration and use the existing operation inspection/reconciliation route;
retain unresolved or unconfirmed standing when the evidence cannot settle it.
