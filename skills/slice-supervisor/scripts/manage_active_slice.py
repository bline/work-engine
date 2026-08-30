#!/usr/bin/env python3
"""Publish supervisor-decided lifecycle transitions for one active slice."""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path


def load_module(path: Path):
    spec = importlib.util.spec_from_file_location("manage_active_live_state", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


LIVE = load_module(Path(__file__).with_name("live_slice_state.py"))


def json_value(raw: str, label: str):
    try:
        return json.loads(raw)
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid {label}: {error}") from error


def current_at_revision(store, identity: dict, expected_revision: str) -> dict:
    current = LIVE.load(store, identity)
    if current is None:
        raise ValueError("active slice attempt state not found")
    if current["durable_revision"] != expected_revision:
        raise ValueError("expected durable revision does not match current state")
    return current


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True, type=Path)
    commands = parser.add_subparsers(dest="command", required=True)

    begin = commands.add_parser("begin")
    begin.add_argument("--identity-json", required=True)
    begin.add_argument("--actor-binding-json", required=True)
    begin.add_argument("--phase", required=True)
    begin.add_argument("--obligation-json", required=True)
    begin.add_argument("--authoritative-refs-json", default="[]")
    begin.add_argument("--accepted-boundary-json")

    publish_phase = commands.add_parser("publish-phase")
    publish_phase.add_argument("--identity-json", required=True)
    publish_phase.add_argument("--expected-revision", required=True)
    publish_phase.add_argument("--phase", required=True)
    publish_phase.add_argument("--consequence-json", required=True)

    bind_actor = commands.add_parser("bind-actor")
    bind_actor.add_argument("--identity-json", required=True)
    bind_actor.add_argument("--expected-revision", required=True)
    bind_actor.add_argument("--event-id", required=True)
    bind_actor.add_argument("--actor-binding-json", required=True)

    wait = commands.add_parser("wait")
    wait.add_argument("--identity-json", required=True)
    wait.add_argument("--expected-revision", required=True)
    wait.add_argument("--event-id", required=True)
    wait.add_argument("--capability", required=True)
    wait.add_argument("--provider")
    wait.add_argument("--reason", required=True)

    retire = commands.add_parser("retire")
    retire.add_argument("--identity-json", required=True)
    retire.add_argument("--expected-revision", required=True)
    retire.add_argument("--event-id", required=True)
    retire.add_argument("--outcome", required=True)
    retire.add_argument("--reason", required=True)

    args = parser.parse_args()
    try:
        store = LIVE.DURABLE.GitRefDurableStateStore(args.repository)
        identity = json_value(args.identity_json, "identity JSON")
        if args.command == "begin":
            result = LIVE.create(store, identity=identity,
                actor_binding=json_value(args.actor_binding_json, "actor binding JSON"),
                phase=args.phase,
                pending_obligation=json_value(args.obligation_json, "obligation JSON"),
                authoritative_refs=json_value(args.authoritative_refs_json,
                                              "authoritative refs JSON"),
                accepted_boundary=(json_value(args.accepted_boundary_json,
                                              "accepted boundary JSON")
                                   if args.accepted_boundary_json else None))
        else:
            current = current_at_revision(store, identity, args.expected_revision)
            if args.command == "wait":
                result = LIVE.wait_on_capability(store, current, event_id=args.event_id,
                    capability=args.capability, provider=args.provider, reason=args.reason)
            elif args.command == "retire":
                result = LIVE.retire(store, current, event_id=args.event_id,
                    outcome=args.outcome, reason=args.reason)
            elif args.command == "bind-actor":
                result = LIVE.bind_actor(store, current, event_id=args.event_id,
                    actor_binding=json_value(args.actor_binding_json, "actor binding JSON"))
            else:
                result = LIVE.publish_phase_consequence(store, current, phase=args.phase,
                    consequence=json_value(args.consequence_json, "consequence JSON"))
    except (json.JSONDecodeError, OSError, RuntimeError, ValueError) as error:
        print(f"manage_active_slice: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
