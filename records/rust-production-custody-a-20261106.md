# Workspace publication custody A

This source candidate implements the workspace-publication owner seam for an
independently selected managed repository and a confined untrusted process
launcher. It does not compose the production review/completion startup (B),
settle a coding job (C), install policy, or establish live product qualification.

`ManagedCustody::load_installed()` reads only
`/etc/work-engine/workspace-publication/custody.json`. All selection ancestry
must be canonical, root-owned and protected against group/other writes. The
selection binds the nonroot runtime UID, executing image digest, epoch, private
bare repository and ledger-root identities, permanent lock identity, canonical
refs, exact validator argv/program digest, validation policy and time limit.
The running executable must itself have protected root-owned ancestry. Bwrap
and Git have independently reviewed compiled-in pins; request JSON cannot
choose their executable authority. A build with `test-faults` cannot obtain
production installed custody.

An `initialize` selection authorizes only trusted owner allocation through
`Workspace::initialize_managed`. It cannot prepare, publish or execute untrusted
code. After allocation, the operator must independently issue an `active`
selection sealing the resulting marker and database device/inode/UID/mode.
`Workspace::open_managed` verifies those identities; it does not recapture a
replacement as current. Selection removal/replacement/content change, runtime
replacement, root/common-dir/lock identity changes or unsafe Git configuration
refuse further authority. An epoch change cannot authorize a new physical
attempt from an older preparation. An unfinalized attempt observed after an
epoch change remains unconfirmed or unresolved, preserving the original
one-shot boundary. Cached receipts retain their historical meaning.

The trusted host may retain the operator UID. It must launch every untrusted
inference/validator process through the enforced supported boundary; an
unrestricted same-UID agent outside that boundary is unsupported. The current
Codex development worker is not such a confined production agent. Trusted
operator administration is outside the agent grant and must revoke/replace the
installation epoch before altering managed custody. No claim of detecting an
adversarial trusted operator's erase-and-restore history is made.

The Linux boundary uses pinned bwrap, user/PID/network/mount namespaces,
disabled nested user namespaces, capability drop, no-new-privileges, new
session, isolated proc/dev/tmp, read-only system tools and selected source,
and private writable scratch. The parent explicitly marks all descriptors
above stdio close-on-exec before bwrap: bubblewrap alone preserves deliberately
inherited descriptors. This is distinct from intentional guard inheritance
into trusted Git topology helpers. Unix control sockets are omitted, not
merely mounted read-only; selected source rejects sockets, devices, FIFOs,
hard links and nested filesystem mounts. Candidate `.git` links confer no
canonical access because the target is absent from the namespace.

This realization supports a private bare canonical repository, local files
without alternate/grafted/shallow object topology, and executables beneath
`/usr` inside the untrusted namespace. The trusted host selects source below
its `inputs/<selection>` or `operations/<operation>` namespace. A validator
reads `/candidate`, receives the actual integrated tree identity, and may write
`/scratch`; it cannot write candidate or canonical state. A physical checkout
check rejects byte/type/executable-mode transforms before validation. Git
helpers reject executable-capable repository configuration and disable hooks,
fsmonitor, global attributes and external protocol execution. No native model
program, credential or provider network grant is added by A.

The test-owner selection constructor exists only under `test-faults`. It runs
the same namespace, descriptor, filesystem and owner publication mechanics but
labels receipts `fixture_managed_custody` with an explicit absence of installed
production authority. These fixtures qualify mechanics, not operator approval,
actual deployed agent confinement or a complete genuine coding/review loop.

The inert installation proposal and exact source/test custody are owned by
`/home/bline/.local/state/work-engine/rust-production-custody-20261106T2144/builder`.
The selected production executable/caller is unissued: B must compose these
library APIs and Root must collect a concrete reviewed installation selection
before any protected action or full production-mode qualification. No fixture
selection is installation material.
