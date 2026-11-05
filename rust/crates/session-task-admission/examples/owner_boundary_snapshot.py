#!/usr/bin/env python3
"""SYNTHETIC scoped-control fixture adapter; never a live publisher.

Only the trusted test backend selects this executable. It calls the existing
board owner's fixed read surface and preserves owner-answer/v1 request bytes.
"""
import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "skills/durable-state/scripts"))
import codex_chatboard as owner

parser = argparse.ArgumentParser()
parser.add_argument("--repository", required=True, type=Path)
args = parser.parse_args()
snapshot = owner.owner_boundary_io(args.repository, {"operation": "snapshot"})
print(json.dumps({**snapshot, "next_sequence": len(snapshot["rows"]) + 1}, ensure_ascii=False))
