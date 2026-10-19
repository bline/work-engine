#!/usr/bin/env python3
"""Inference cost of the Rust programme, by campaign slice, in weekly quota.

Usage:
  python3 -I rust_programme_cost.py ~/.codex/sessions/2026/10/*/rollout-*.jsonl \
                                    ~/.codex/archived_sessions/*.jsonl

Attribution is by session_meta.agent_path, which names the campaign ordinal
directly. Interval attribution does not work on this corpus because several
campaigns ran concurrently.

Threads are deduplicated by session_meta.id, keeping the copy with the most
calls: the archived store holds partial duplicates of live Oct 7-8 sessions and
summing files double counts them. Per-call usage is info.last_token_usage;
info.total_token_usage is account-lifetime cumulative.

Root orchestration is reported separately and allocated back over all non-root
work pro rata, so the per-slice column is fully loaded and the total reconciles
with measured spend.
"""
import json, sys, re, collections

MWEIGHTED_PER_WEEK = 162.3   # Pro 200 = 10x; 100 points at 6.16 Plus-units/Mweighted
SINCE = "2026-10-01"

RULES = [
    (r'/s10_', 'S10 replacement qualification'),
    (r'/s9_', 'S9 fresh start + deferred migration'),
    (r's8_', 'S8 retained workflow'),
    (r's67_|/s7_|/s6_', 'S6+S7 semantic + admin'),
    (r'_s5_|/s5_', 'S5 native provider'),
    (r'_s4_', 'S4 first service'),
    (r'_s3_|ui_s3', 'S3 wire + client'),
    (r'_s2_', 'S2 async execution'),
    (r'_s1_', 'S1 core + store'),
    (r'_s0_|s0_recovery', 'S0 foundation'),
    (r'rp\d|review_provider', 'RP review provider'),
    (r'sc\d|campaign', 'SC slice campaign'),
    (r'ce\d|claim', 'CE claim evidence'),
    (r'hp\d|h0_|h1_|host', 'HP host'),
    (r'/c\d_|compiler|rust_c\d', 'C compiler'),
    (r'/r\d_|episode|_r0', 'R review episode'),
    (r'j1|j2|completion_boundary', 'J owner join + completion'),
    (r'/w\d_|workspace', 'W workspace'),
    (r'p1_|replay', 'P replay regression'),
    (r'er1', 'ER evidence read'),
    (r'e0_', 'E0 evidence'),
    (r'/ui_', 'UI terminal'),
    (r'schema14', 'schema14 (not Rust)'),
    (r'rust_native_lifecycle_vertical|rust_lifecycle_implementation|rust_lifecycle_offline',
     'S lifecycle shared'),
    (r'astra_activation|astra_inventory|model_adapter', 'coordination / readiness'),
]
NON_RUST = {'schema14 (not Rust)', 'coordination / readiness', 'unclassified'}


def slot(agent_path):
    s = agent_path.lower()
    for pat, name in RULES:
        if re.search(pat, s):
            return name
    return 'ROOT orchestration' if s == '/root' else 'unclassified'


def load(paths):
    """tid -> (ncalls, agent_path, mweighted, first_ts) keeping the longest copy."""
    best = {}
    for f in paths:
        tid = ap = None
        rows = []
        for line in open(f, errors="ignore"):
            if '"token_count"' not in line and '"session_meta"' not in line:
                continue
            try:
                d = json.loads(line)
            except Exception:
                continue
            p = d.get("payload")
            if not isinstance(p, dict):
                continue
            if d.get("type") == "session_meta":
                tid, ap = p.get("id"), p.get("agent_path") or "/root"
            elif p.get("type") == "token_count":
                l = (p.get("info") or {}).get("last_token_usage") or {}
                i_ = l.get("input_tokens", 0)
                c = l.get("cached_input_tokens", 0)
                o = l.get("output_tokens", 0)
                if not (i_ or o):
                    continue
                rows.append((d.get("timestamp"), (i_ - c) + c / 10.0 + 5 * o))
        if not (tid and rows):
            continue
        rec = (len(rows), ap, sum(r[1] for r in rows), min(r[0] for r in rows))
        if tid not in best or rec[0] > best[tid][0]:
            best[tid] = rec
    return [v for v in best.values() if v[3] >= SINCE]


def main(argv):
    threads = load(argv)
    if not threads:
        print("no threads matched"); return 1
    total = sum(t[2] for t in threads)
    mwt = collections.Counter()
    calls = collections.Counter()
    first = {}
    for n, ap, w, f in threads:
        k = slot(ap)
        mwt[k] += w
        calls[k] += n
        first[k] = min(first.get(k, "z"), f)

    root = mwt['ROOT orchestration']
    non_root = total - root
    oh = 1 + root / non_root

    print("%d threads since %s, %.1f Mweighted = %.2f Pro-200 weeks" % (
        len(threads), SINCE, total / 1e6, total / 1e6 / MWEIGHTED_PER_WEEK))
    print("root orchestration %.1f Mweighted = %.1f%% of non-root work, "
          "allocated pro rata at x%.3f\n" % (root / 1e6, 100 * root / non_root, oh))

    print("%-38s %9s %9s %10s %8s %s" % (
        "slice / track", "Mwt", "+root", "Pro200wk", "calls", "started"))
    rust = 0.0
    for k, w in mwt.most_common():
        if k == 'ROOT orchestration':
            continue
        loaded = w * oh
        if k not in NON_RUST:
            rust += loaded
        print("%-38s %9.1f %9.1f %10.3f %8d %s" % (
            k, w / 1e6, loaded / 1e6, loaded / 1e6 / MWEIGHTED_PER_WEEK,
            calls[k], first[k][:10]))
    print("\n%-38s %9s %9.1f %10.3f" % (
        "RUST PROGRAMME TOTAL", "", rust / 1e6, rust / 1e6 / MWEIGHTED_PER_WEEK))
    print("%-38s %9s %9.1f %10.3f" % (
        "check: all work incl. non-Rust", "", non_root * oh / 1e6,
        non_root * oh / 1e6 / MWEIGHTED_PER_WEEK))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
