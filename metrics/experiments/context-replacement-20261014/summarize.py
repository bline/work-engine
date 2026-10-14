#!/usr/bin/env python3
"""Recompute every figure quoted in README.md from observations.json.

Run with the observations path. The README states no number this script does not
print, so a later extraction can be compared against the frozen one.
"""
import collections
import json
import pathlib
import statistics
import sys

def quantile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[min(int(fraction * len(ordered)), len(ordered) - 1)]

def main() -> int:
    if len(sys.argv) != 2:
        print("usage: summarize.py <observations.json>", file=sys.stderr)
        return 2
    document = json.loads(pathlib.Path(sys.argv[1]).read_text())
    sessions = document["sessions"]
    before, retained, by_date = [], [], collections.defaultdict(list)
    cold, tails = [], []
    for session in sessions:
        for replacement in session["replacements"]:
            if replacement["context_before_tokens"]:
                before.append(replacement["context_before_tokens"])
            size = replacement["retained_context_bytes"]
            if size is not None:
                retained.append(size)
                by_date[replacement["timestamp"][:10]].append(size)
        tail = session["climb_after_last_replacement"]
        if len(tail) >= 14:
            cold.append(tail[0]["units"])
            tails.append((tail[0]["context_tokens"], tail[-1]["context_tokens"],
                          sum(entry["units"] for entry in tail)))
    print("corpus: %d sessions, %d replacements, %d model calls, %.0f estimated Plus-units" % (
        document["session_count"], document["replacement_count"],
        sum(item["model_calls"] for item in sessions),
        sum(item["estimated_units"] for item in sessions)))
    print("sessions offering a new_context tool: %d of %d" % (
        sum(1 for item in sessions if item["new_context_tool_present"]), len(sessions)))
    print()
    print("context before replacement (n=%d): min %.0fK p10 %.0fK median %.0fK p90 %.0fK max %.0fK" % (
        len(before), min(before) / 1000, quantile(before, 0.1) / 1000,
        statistics.median(before) / 1000, quantile(before, 0.9) / 1000, max(before) / 1000))
    print("  at or above 200K: %.0f%%   below 150K: %.0f%%" % (
        100 * sum(1 for value in before if value >= 200000) / len(before),
        100 * sum(1 for value in before if value < 150000) / len(before)))
    print()
    print("retained_context bytes (n=%d): min %d median %.0f max %d; blank (<=200 bytes) %d" % (
        len(retained), min(retained), statistics.median(retained), max(retained),
        sum(1 for value in retained if value <= 200)))
    blank_days = sorted(day for day, sizes in by_date.items() if max(sizes) <= 200)
    mixed_days = sorted(day for day, sizes in by_date.items() if max(sizes) > 200)
    print("  last all-blank date %s; first date with retention %s" % (
        blank_days[-1] if blank_days else "-", mixed_days[0] if mixed_days else "-"))
    print()
    print("first call after a replacement (n=%d sessions with a 14-call tail): median %.3f u p90 %.3f max %.3f" % (
        len(cold), statistics.median(cold), quantile(cold, 0.9), max(cold)))
    print("14-call tail: floor median %.0fK reaching %.0fK, median cost %.2f u" % (
        statistics.median([item[0] for item in tails]) / 1000,
        statistics.median([item[1] for item in tails]) / 1000,
        statistics.median([item[2] for item in tails])))
    print()
    print("cost per call by context band:")
    for band, value in document["combined_cost_by_context_band"].items():
        print("   %-10s calls %6d  units/call %.4f" % (band, value["calls"], value["units_per_call"]))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
