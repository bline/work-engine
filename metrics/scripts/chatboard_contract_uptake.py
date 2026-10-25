#!/usr/bin/env python3
"""Board-only uptake of the message envelope: presence of each envelope field, resolvable references,
prose effect restatement and `effects` presence.

Usage: python3 -I metrics/scripts/chatboard_contract_uptake.py [--freeze F] [--after-sequence K] [--out O]

The frozen board prefix (sequence <= freeze.board.max_sequence) is always read and verified (exits non-zero on
digest mismatch). Default window = that prefix. With --after-sequence K the window is the live board's
messages with sequence > K (use after S1 lands; K is the sequence to start from). The two windows are
reported with their sizes; they cover different periods and are observational, not a matched comparison.

Metrics per window, over messages M:
  envelope.<field>      messages carrying the field (kind, role, to, in_reply_to, supersedes, effects)
  any_envelope_field    messages carrying at least one
  resolvable            messages whose in_reply_to/supersedes names an existing earlier sequence
                        (see chatboard_baselines.resolvable for the prose-candidate denominators)
  restatement           messages whose body matches chatboard_baselines.RE_NO_EFFECT
  effects_presence      among kind in {status, availability}: share carrying `effects`
  restatement_without_effects   restatement messages that carry no `effects` (lost-information check)
"""
import json, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import chatboard_board_use as bu
import chatboard_baselines as bl


def uptake(msgs):
    n = len(msgs)
    has = lambda m, f: m.get(f) is not None
    restate = [m for m in msgs if bl.RE_NO_EFFECT.search(m["body"])]
    sa = [m for m in msgs if m.get("kind") in ("status", "availability")]
    return {
        "messages": n,
        "envelope": {f: sum(1 for m in msgs if has(m, f)) for f in bl.ENVELOPE_FIELDS},
        "any_envelope_field": sum(1 for m in msgs if any(has(m, f) for f in bl.ENVELOPE_FIELDS)),
        "resolvable": bl.resolvable(msgs),
        "restatement": len(restate),
        "restatement_without_effects": sum(1 for m in restate if not has(m, "effects")),
        "effects_presence_on_status_availability": {"of": len(sa), "with_effects": sum(1 for m in sa if has(m, "effects"))},
    }


def main(argv):
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--freeze", default=str(bu.default_freeze()))
    ap.add_argument("--after-sequence", type=int)
    ap.add_argument("--out")
    a = ap.parse_args(argv)
    fr = bu.load_freeze(a.freeze)
    frozen = bl.frozen_board(fr)
    result = {"artifact_type": "chatboard_contract_uptake_v1",
              "frozen_window": {"max_sequence": fr["board"]["max_sequence"], **uptake(frozen)}}
    if a.after_sequence is not None:
        live = bl.board_messages(a.after_sequence)
        result["live_window"] = {"after_sequence": a.after_sequence, **uptake(live)}
    text = json.dumps(result, indent=1, sort_keys=True) + "\n"
    if a.out:
        Path(a.out).write_text(text)
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main(sys.argv[1:])
