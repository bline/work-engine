#!/usr/bin/env python3
"""Where engineering inference goes by role, and why the phase split does not survive.

Usage:
  python3 -I rust_programme_roles.py ~/.codex/sessions/2026/10/*/rollout-*.jsonl \
                                     ~/.codex/archived_sessions/*.jsonl

Two outputs.

1. Role split. Threads are classified by `session_meta.agent_path` into build,
   review+remediation, planning+contract and root, and root is allocated pro
   rata over the rest. This is measurement.

2. Phase split, which FAILS, and is kept because the failure is the finding.
   Segmenting each builder thread at the first non-plan review of its own
   ordinal is an attempt to separate initial implementation from correction.
   It does not work: ordinals split into two populations by campaign style,
   not by correction rate. Where review runs concurrently with ongoing planned
   work, "after the first review" captures the whole build and says nothing
   about rework. Chronology does not recover purpose; only attribution at
   dispatch time would.

Per-call usage is info.last_token_usage; info.total_token_usage is
account-lifetime cumulative. Threads are deduplicated by session_meta.id,
keeping the copy with the most calls.
"""
import json, sys, re, collections

MWEIGHTED_PER_WEEK = 162.3
SINCE = "2026-10-01"


def role(agent_path):
    s = agent_path.lower()
    if s == "/root":
        return "root orchestration"
    if re.search(r'review|reviewer|_g\d$|remediation|repair|gate_fix|fix$', s):
        return "review + remediation"
    if re.search(r'plan|characterization|contract|disposition|readiness|inventory', s):
        return "planning + contract"
    if re.search(r'build|implementation|_sol$|integration|evidence|proof|recover|prepare|vertical', s):
        return "build"
    return "other"


def ordinal(agent_path):
    m = re.search(r'/(?:root/)?([a-z]+\d+[a-z]?)[_/]', agent_path + "_")
    return m.group(1) if m else None


def load(paths):
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
        if tid and rows and min(r[0] for r in rows) >= SINCE:
            if tid not in best or len(rows) > len(best[tid][1]):
                best[tid] = (ap, sorted(rows))
    return best


def main(argv):
    best = load(argv)
    if not best:
        print("no threads matched"); return 1

    mwt = collections.Counter()
    for ap, rows in best.values():
        mwt[role(ap)] += sum(r[1] for r in rows)
    total = sum(mwt.values())
    root = mwt["root orchestration"]
    non_root = total - root
    oh = 1 + root / non_root

    print("== MEASURED: role split ==")
    print("%d threads since %s, %.1f Mweighted; root %.1f = %.1f%% of non-root, allocated x%.3f\n"
          % (len(best), SINCE, total / 1e6, root / 1e6, 100 * root / non_root, oh))
    print("%-24s %9s %9s %9s %8s" % ("role", "Mwt", "+root", "Pro200wk", "share"))
    for k, v in mwt.most_common():
        if k == "root orchestration":
            print("%-24s %9.1f %9s %9s %7.1f%%" % (k, v / 1e6, "-", "-", 100 * v / total))
            continue
        l = v * oh
        print("%-24s %9.1f %9.1f %9.3f %7.1f%%"
              % (k, v / 1e6, l / 1e6, l / 1e6 / MWEIGHTED_PER_WEEK, 100 * v / non_root))
    b, r = mwt["build"] * oh, mwt["review + remediation"] * oh
    print("\nreviewer threads are %.0f%% of builder spend" % (100 * r / b))

    # phase split -- the negative result
    groups = collections.defaultdict(lambda: {"build": [], "review": []})
    for ap, rows in best.values():
        o = ordinal(ap)
        if not o:
            continue
        kind = ("review" if re.search(r'review|gate_fix|remediation|repair', ap.lower())
                else "build")
        groups[o][kind].append((ap, rows))

    print("\n== NEGATIVE RESULT: phase split does not survive ==")
    print("%-8s %9s %9s %s" % ("ordinal", "build Mwt", "rev Mwt", "builder spend after first non-plan review"))
    seq, inter = [], []
    for o in sorted(groups):
        g = groups[o]
        if not g["build"] or not g["review"]:
            continue
        bm = sum(r[1] for _, rs in g["build"] for r in rs)
        if bm < 3e6:
            continue
        revs = [rs for ap2, rs in g["review"] if "plan" not in ap2.lower()]
        if not revs:
            continue
        first = min(rs[0][0] for rs in revs)
        pre = sum(r[1] for _, rs in g["build"] for r in rs if r[0] < first)
        post_pct = 100 * (bm - pre) / bm
        (inter if post_pct >= 70 else seq).append((o, bm, bm - pre))
        print("%-8s %9.1f %9.1f %6.0f%%"
              % (o, bm / 1e6, sum(r[1] for _, rs in g["review"] for r in rs) / 1e6, post_pct))

    for name, pop in (("build-then-review", seq), ("review interleaved", inter)):
        if not pop:
            continue
        tb = sum(x[1] for x in pop); tp = sum(x[2] for x in pop)
        print("\n  %-20s %s" % (name, ", ".join(x[0] for x in pop)))
        print("  %-20s builder %.1f Mwt, after first review %.1f = %.1f%%"
              % ("", tb / 1e6, tp / 1e6, 100 * tp / tb))
    print("\n  These are two campaign styles, not two correction rates. The share of")
    print("  builder spend occurring after a review is NOT a rework estimate.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
