#!/usr/bin/env python3
"""Recover one active slice attempt from durable state in a fresh process."""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path


def load_module(path: Path):
    spec = importlib.util.spec_from_file_location("resume_active_live_state", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


LIVE = load_module(Path(__file__).with_name("live_slice_state.py"))


def resume(repository: Path, identity: dict, *, capability: str | None = None,
           recovery_event_id: str | None = None) -> dict:
    store = LIVE.DURABLE.GitRefDurableStateStore(repository)
    state = LIVE.load(store, identity)
    if state is None:
        raise ValueError("active slice attempt state not found")
    if state["identity"] != identity:
        raise ValueError("active slice attempt identity mismatch")
    if capability is None and recovery_event_id is None:
        return state
    if not capability or not recovery_event_id:
        raise ValueError("capability and recovery_event_id must be supplied together")
    return LIVE.resume_capability(store, state, event_id=recovery_event_id, capability=capability)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True, type=Path)
    parser.add_argument("--identity-json", required=True)
    parser.add_argument("--capability")
    parser.add_argument("--recovery-event-id")
    args = parser.parse_args()
    try:
        result = resume(args.repository, json.loads(args.identity_json),
                        capability=args.capability, recovery_event_id=args.recovery_event_id)
    except (json.JSONDecodeError, OSError, RuntimeError, ValueError) as error:
        print(f"resume_active_slice: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

