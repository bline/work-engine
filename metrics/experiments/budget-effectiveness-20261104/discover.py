"""Walk candidate roots; list every .json file whose top-level object has a scope_budget dict. Reports counts per root and files skipped."""
import os, sys, json
roots = sys.argv[2:]
found, big, bad, scanned = [], [], 0, 0
for root in roots:
    for dp, dns, fns in os.walk(root):
        dns[:] = [d for d in dns if d not in ('node_modules', '.git', 'target', '__pycache__')]
        for fn in fns:
            if not fn.endswith('.json'): continue
            p = os.path.join(dp, fn); scanned += 1
            try:
                sz = os.path.getsize(p)
                if sz > 20_000_000: big.append(p); continue
                with open(p, 'rb') as fh: b = fh.read()
                if b'"scope_budget"' not in b: continue
                d = json.loads(b)
                if isinstance(d, dict) and isinstance(d.get('scope_budget'), dict): found.append(p)
            except Exception: bad += 1
open(sys.argv[1], 'w').write('\n'.join(found) + '\n')
print('scanned', scanned, 'found', len(found), 'oversize_skipped', len(big), 'unreadable_or_nonjson', bad)
for b in big: print('  big', b)
