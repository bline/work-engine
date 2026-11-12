# Bounded research read profile v1

M3a produces a complete structural batch report or a refusal. It establishes no
research qualification, applicability, adoption or execution authority. Required
null anchor fields remain structural observations. Full cards and required-null
card refusal belong to M3b.

`work-engine-research-read CGROUP_ROOT TRUSTED_PROFILE_JSON` reads one JSON
`ReadRequest` from stdin (at most 16 KiB). The operator installs the profile and
repository scope separately; requests cannot broaden either. Repository scope
covers that repository's object closure and history. After scope admission the worker opens the admitted canonical repository path, so
retargeting a request alias cannot redirect opening. This does not freeze
repository storage. The source commit must be
an exact lowercase SHA-1 id. The bundle retains M1's original byte identities.
Inside one worker, snapshot creation, register validation, joined bundle
validation and whole-register anchors share the same object reader and pin.
An invalid register or failed bundle cannot yield a complete batch.

The initial maximums are 16 decisions, 1 MiB and 10,000 lines per blob, 8 MiB
aggregate distinct decoded object payload, 60 seconds process CPU, 60 seconds
parent wall deadline, 1 GiB cgroup memory, and 8 MiB combined stdout/stderr.
Objects other than blobs also have a conservative 1 MiB allocation admission.
Smaller positive profiles are permitted; larger values refuse. Internal limits
are 512 MiB aggregate compressed source copies, 32 MiB index bytes plus reverse
offset inventory and distinct index generations, 16 indexed packs, delta depth
64, 32,768 dependency/ancestry and directory steps, and 8,192 distinct decoded
objects. Repository configuration files are individually bounded to 1 MiB.
These are qualification bounds, not universal product or scientific policy.

The bounded reader supports ordinary worktrees and bare SHA-1 repositories. It
refuses linked common directories, alternates and multi-pack indices rather
than reading outside its implemented source inventory. Existing ordinary APIs
retain their existing Git access path. Bounded access uses the existing config
checks, object parsers and hash verification, without opening an unrestricted
gix repository. It checks physical source size before allocating an exact
private copy. Loose headers are streamed into a bounded prefix before payload
admission. Every packed OFS/REF base is recursively admitted and hash verified
before target decoding. OFS and same-pack REF admission follows physical pack
locations even when another pack or the OID cache contains a duplicate. Every
intermediate declared and result size is bounded; distinct verified payload
identities are charged once. Preflight and decoding use the same immutable owned
bytes; decoder work buffers additionally use gix's allocation limit. Cgroup
containment covers decoder/runtime overhead and unexpected expansion.

A copied payload does not freeze the universe used for prefix uniqueness.
Every prefix lookup re-observes the current bounded index/loose inventory,
admits new index generations and reuses unchanged copied pack handles. Required
source/index disappearance refuses even if decoded bytes were cached. Objects
returned from surviving replaced paths still come from the immutable admitted
copy and must match their requested hashes. Source mutations can cause refusal;
there is no promise of a transactional snapshot of repository storage.

The Linux parent creates only an owned child beneath a delegated cgroup-v2
parent with memory and pids controllers already enabled. It installs memory.max,
zero swap, group OOM and a pids bound; the fork child joins that group and sets
RLIMIT_CPU and zero core-dump allowance before exec. Missing controls refuse.
CPU limits have whole-second granularity; CPU signal termination is reported as
worker exit, separately from observed cgroup OOM. No address-space limit stands
in for cgroup memory semantics. The parent checks the executable hash and runs
a sealed memfd copy, bounding the admitted executable to 128 MiB. Object-source
copies live only in worker memory and disappear with that worker.

After loading and validating the trusted profile, the public command installs
cancellation handling and starts one wall clock covering request acquisition,
parsing and worker execution. Retaining stdin open cannot extend that deadline.
The parent drains stdout and stderr under one shared cap, independently watches
wall time and cancellation, kills/drains/waits its worker and owned cgroup, and
publishes only after successful exit, exact complete framing, output bounds,
producer, profile, source/bundle pins and complete outcome checks. Crash, OOM,
CPU termination, timeout, cancellation, refusal and incomplete frames discard
staged success. Cancellation before publication refuses. A downstream consumer
losing a reply after delivery has unknown delivery; this read operation provides
no delivery receipt or authority to infer absence.

Private `WorkerInput.probe` workloads exercise the same shipping worker and host
controls; the public request schema cannot select them. They supply no research
result. The register-only probe returns schema `research-read-qualification-v1`,
outcome `control_complete` and `m1_bundle: NOT_EXERCISED`; the joined batch
consumer cannot use it as normal batch success. It supplies real-source
resource evidence only. Linux qualification requires an explicitly delegated test parent:

```sh
RESEARCH_READ_CGROUP_ROOT=/sys/fs/cgroup/<delegated-parent> \
  cargo test -p research-register --offline -- --include-ignored --nocapture
```

The host qualification tests are ignored by default because an ordinary
build environment may lack delegation. A default cargo test pass does not
establish host qualification. OFS/REF synthetic pack controls and ordinary
regressions run by default. The repository-history oracle test requires the
actual enclosing repository history.
