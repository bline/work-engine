#!/usr/bin/env python3
"""Verify the exact accepted S3 U0 manifest and every published input byte."""
import hashlib
import json
import subprocess
from pathlib import Path

WIRE = Path(__file__).resolve().parents[2] / "lifecycle-wire"
MANIFEST = WIRE / "schemas/v1/manifest.json"
PIN = "f4c73a45602284989fc352f5529f421a5e2c73cdaece2c4678035825032727be"
S3_COMMIT = "634161c2"
REPO = Path(__file__).resolve().parents[4]
S4_PIN = Path(__file__).resolve().parents[1] / "docs/s4-client-pin.json"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    actual = sha(MANIFEST)
    if actual != PIN:
        raise SystemExit(f"S3 manifest changed: {actual}; expected {PIN} ({MANIFEST})")
    manifest = json.loads(MANIFEST.read_text())
    groups = (
        ("schema_sha256", WIRE / "schemas/v1"),
        ("fixture_sha256", WIRE / "tests/fixtures"),
    )
    checked = 1
    for key, root in groups:
        for name, expected in manifest[key].items():
            path = root / name
            actual = sha(path)
            if actual != expected:
                raise SystemExit(f"S3 {key} changed: {path}: {actual}; expected {expected}")
            checked += 1
    for name, expected in manifest["source_sha256"].items():
        path = (WIRE / name).resolve().relative_to(REPO)
        historical = subprocess.check_output(
            ["git", "show", f"{S3_COMMIT}:{path}"], cwd=REPO
        )
        actual = hashlib.sha256(historical).hexdigest()
        if actual != expected:
            raise SystemExit(f"historical S3 source changed: {path}: {actual}; expected {expected}")
        checked += 1
    s4 = json.loads(S4_PIN.read_text())
    for name, expected in s4["sha256"].items():
        path = REPO / name
        actual = sha(path)
        if actual != expected:
            raise SystemExit(f"current S4 source changed: {path}: {actual}; expected {expected}")
        published = subprocess.check_output(
            ["git", "show", f"{s4['published_commit']}:{name}"], cwd=REPO
        )
        if hashlib.sha256(published).hexdigest() != expected:
            raise SystemExit(f"S4 receipt does not match published commit: {name}")
    print(f"S3 U0 historical pin verified: manifest {PIN}; {checked} files")
    print(f"S4 current public-source pin verified: {s4['published_commit']}; {len(s4['sha256'])} files")


if __name__ == "__main__":
    main()
