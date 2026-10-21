#!/usr/bin/env bash
# Build work-engine-context-status outside the shared Cargo target and pin it
# by digest, so observation never takes a workspace build lock.
#
# Usage: provision_context_status.sh [REPOSITORY]
# Prints the pinned binary path, its SHA-256, the source commit and whether
# the crate had uncommitted edits; a new pin also gets provenance.json. An
# existing pin with the same digest is reused, never overwritten.
set -euo pipefail

repo="${1:-$(git rev-parse --show-toplevel)}"
state="${WORK_ENGINE_STATE:-$HOME/.local/state/work-engine}"
target="${CONTEXT_STATUS_TARGET:-$state/tools/context-status/target}"

cargo build --locked --release \
  --manifest-path "$repo/rust/Cargo.toml" \
  --target-dir "$target" \
  -p work-engine-context-status >&2

built="$target/release/work-engine-context-status"
digest="$(sha256sum "$built" | cut -c1-64)"
pin="$state/tools/context-status/$digest"

commit="$(git -C "$repo" rev-parse HEAD)"
crate="rust/crates/work-engine-context-status"
if [ -n "$(git -C "$repo" status --porcelain -- "$crate" rust/Cargo.lock)" ]; then dirty=true; else dirty=false; fi

if [ ! -x "$pin/work-engine-context-status" ]; then
  mkdir -p "$pin"
  cp "$built" "$pin/work-engine-context-status"
  chmod 0555 "$pin/work-engine-context-status"
  printf '{"sha256":"%s","source_commit":"%s","crate_dirty":%s,"built_at":"%s"}\n' \
    "$digest" "$commit" "$dirty" "$(date -u +%FT%TZ)" > "$pin/provenance.json"
fi

printf 'binary=%s\nsha256=%s\nsource_commit=%s\ncrate_dirty=%s\n' \
  "$pin/work-engine-context-status" "$digest" "$commit" "$dirty"
