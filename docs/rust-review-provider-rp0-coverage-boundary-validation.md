# RP0 full-tree coverage and direct-source fallback

The RP0 boundary now separates the selected Git snapshot, the exact exposed
byte tree, the copied Codebase Memory store, and graph coverage. A schema-2
boundary can seal with incomplete graph coverage only when its checked ledger
accounts for every selected nonbinary path and records exact source hashes and
direct-source obligations for each recognized gap. The ledger preserves the
raw per-path response and reported ranges. `.bin` files remain outside graph
queries but stay inside the byte-tree digest and appear as opaque, hashed
inventory. `coverage_complete=false` remains a truthful graph observation.
The pinned CBM's `metadata_changed` freshness is recognized as a direct-source
obligation; an unrecognized freshness state still refuses sealing. Neither a
metadata state nor fallback can waive the selected byte-tree or store checks.

Sealing and reopening verify the copied SQLite store, its SHA-256, the ledger
hash and semantics, candidate bytes, profile pins, and the existing selected
Git snapshot check. Legacy schema-1 manifests retain their clean-coverage
requirement. E1 uses RP0's checked boundary admission after `verify_frozen`,
then applies its existing candidate, project, store, generation, and coverage
bindings. No ledger value alone creates an Applied entry or authorizes a
provider call.

The reviewer namespace mounts the ledger read-only at `/cbm/coverage.json`
and the selected source read-only at `/repo/candidate`. The selected compiler
role must carry the exact versioned RP0 fallback guidance in its effective
projection; startup refuses a schema-2 boundary if that instruction is absent.
The task-owned compiler document and compiled prompt are proposals for the
existing compiler and campaign owners to select and pin. Availability of
source and the ledger does not prove a reviewer inspected either. A future
review result must itself cite the required direct-source observations and
graph limitations.

The no-provider default-feature preparation used the unchanged RP3 selected
tree `e49beb9b6788c81920cd276c5e630372503a813e`. It sealed and reopened
the full 2,697-file selected snapshot with an actual copied-store digest:
2,671 paths queried, 188 explicit fallback paths, and 26 opaque `.bin` files.
Its ledger SHA-256 is
`8dd97698fb31a31050fac86453fd48a562afd4e6a7de22fe06acf345c8d54330`;
the store SHA-256 is
`d14addb6a9f276101312a7f57ef670074be3abbd82a77f9d7f06024b0970cfbc`.
The no-provider mount probe read the ledger, an unverified source file, a
partial source file, and an opaque file, while source and ledger writes were
denied. This is an offline mechanism result, not installed acceptance or a
live review. The private store was built in task-owned
`/run/user/1000/rp0-fulltree-3`; an exact verified delta against the preserved
candidate-1 archive, the raw result, and qualification receipts are
durable under `/home/bline/.local/state/work-engine/rp0-coverage-boundary-fix-20261018`.

The old RP0 relay's 4,096-request capacity and hard cleanup deadline remain
unproved. No production credential was read and no provider call, protected
installation, or Applied entry occurred in this qualification.
