"""P1.34 derived Module A packet. Source: the frozen P1.33 packet (prompt a60ce900, 14 files). Each source file is
checked against its P1.33 manifest digest, then only the declared provenance redactions are applied, each with an
exact expected count. Line counts are unchanged, so every file:line cites the same line of the frozen file. The
normative rules are untouched; only labels that disclose the earlier review, its findings, or the author's
change log are removed.
build:  build_packet_p134.py build   -> sources/, prompt-oneshot.md, PACKET-MANIFEST.json, REDACTION-MAP.json, redaction.diff
verify: build_packet_p134.py verify  -> re-derives every file from the frozen source and checks digests, delimiters, sentinel"""
import hashlib, json, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
P133 = '/home/bline/cbm-work/p133-s1r5'; P133PK = f'{P133}/packet'; P133_PROMPT = 'a60ce90061d165b66680ab9c6ddc06310a30beae116e1a08aa1c780eaf2bbf9d'
W, W_MANIFEST = f'{P133}/s1', 'c20174532aa0731656a7a749b63000a79f377b97ddab638c420801d34e4af31b'
P4, P4_MANIFEST = '/home/bline/cbm-work/p132-s1r4/s1', '3af24453dd509e8d00d2975c8501a3f0ed7c61fc9b569e2248a25fe80da61d78'
LABEL = (re.compile(r'\bA-0([1-9])\b'), r'R5-0\1')   # neutral revision-item labels in place of review-finding ids
SUBS = {   # path -> ordered [(old, new, expected count)] applied before the label rename
 'contract/CONTRACT.md': [
   ('(revision 5, P1.33: repairs of the P1.32 Module A findings A-01 to A-09', '(revision 5, P1.33: revision items A-01 to A-09,', 1),
   ('and owner dispositions A-01, A-02, A-03, A-06; exact change log in `CHANGELOG-1.4.md`).',
    'of which A-01, A-02, A-03 and A-06 are owner dispositions; see `DECISION-REGISTER.md`).', 1)],
 'contract/DECISION-REGISTER.md': [
   ('## P1.33 owner dispositions (revision 5; P1.32 Module A findings)', '## P1.33 owner dispositions (revision 5)', 1),
   ('The other Module A findings (A-04, A-05, A-07, A-08, A-09) are repaired as P2 corrections without owner decisions. See',
    'Revision items A-04, A-05, A-07, A-08 and A-09 are contract changes made without owner decisions. The author\'s', 1),
   ('`CHANGELOG-1.4.md`.', 'change log is not part of this review set.', 1)],
 'contract/INVARIANTS.md': [(' (P1.32 A-01)', '', 2)],
}
LABELLED = ('contract/CONTRACT.md', 'contract/TRANSITIONS.md', 'contract/INVARIANTS.md', 'contract/MONEY.md', 'contract/RESERVATION-BOUNDS.md',
            'contract/DECISION-REGISTER.md', 'contract/S2A-ENTRY-CRITERIA.md', 'corpus/SCHEMA.md')
CORPUS = 'corpus/sample-cases.jsonl'; DROP = ('findings', 'group')
ORIGIN = ('contract::P1.33 Module A repair (from ', 'contract::targeted case (from ')
LEAK = re.compile(r'Module A (finding|repair)|P1\.32[- ]A-0|\bA-0[1-9]\b|CHANGELOG|repaired|P1\.32 Module|findings A-0')
BEGIN = '===== BEGIN FILE {p} | sha256 {h} | {n} bytes | {l} lines | encoding {e} ====='
END = '===== END FILE {p} ====='
SENT = '===== END OF REVIEW SET: {n} files ====='
REMINDER = ('Reminder: the review set is complete only if every file has its END line and the END OF REVIEW SET line is present. '
            'Answer now, in this single response, following the output contract (Part 1 JSON, then Part 2 prose). No tool calls are available.\n')
DERIVE = ['VC01-openai-usage-settlement', 'VC02-anthropic-usage-settlement', 'P130-R02-rounded-up-charge-admitted', 'VC04-upstream-error-keeps-reservation',
          'S02-control-error-status-after-reservation', 'P133-A02-error-status-with-priceable-usage', 'U08-missing-observed-tier', 'P130-R01-usage-incomplete-terminal',
          'P133-A01-incomplete-usage-above-reservation', 'P2-usage-settlement-unpriced-identity-keeps-charge', 'N01-identity-refusal-dearer-served',
          'Ha-inconsistent-cache-split-keeps-supported-low', 'N06-null-cache-counts', 'P133-A04-cache-write-absent-legacy', 'P133-A04-cache-counts-both-absent',
          'P133-A04-cached-malformed', 'P133-A04-mixed-observations', 'P133-A05-model-case-variant', 'S06-string-http-status', 'A06-reservation-usage-null-status-no-id',
          'P133-A06-usage-settlement-status-absent', 'P133-A06-error-status-null', 'R04-reused-reservation-id', 'P133-A09-conflicting-reservations-with-settlement']
sha = lambda b: hashlib.sha256(b).hexdigest()


def frozen():
    for d, want in ((W, W_MANIFEST), (P4, P4_MANIFEST)):
        assert sha(open(f'{d}/WORKSPACE.MANIFEST.sha256', 'rb').read()) == want, d
        assert subprocess.run(['sha256sum', '-c', '--quiet', 'WORKSPACE.MANIFEST.sha256'], cwd=d, capture_output=True).returncode == 0, d
    assert sha(open(f'{P133PK}/prompt-oneshot.md', 'rb').read()) == P133_PROMPT
    v = subprocess.run(['/usr/bin/python3', '-I', f'{P133PK}/build_packet_p133.py', 'verify'], capture_output=True, text=True, env={})
    assert v.returncode == 0 and 'VERIFY PASS' in v.stdout, v.stdout[-500:]
    return json.load(open(f'{P133PK}/PACKET-MANIFEST.json'))


def derive(path, src, log):
    if path == CORPUS:
        out, dropped, origins = [], 0, 0
        for line in src.decode().split('\n')[:-1]:
            c = json.loads(line)
            for k in DROP: dropped += k in c; c.pop(k, None)
            o = (c.get('provenance') or {}).get('origin', '')
            if o.startswith(ORIGIN[0]): c['provenance']['origin'] = ORIGIN[1] + o[len(ORIGIN[0]):]; origins += 1
            out.append(json.dumps(c, sort_keys=True, separators=(',', ':')))
        log.append(dict(op='drop fields ' + '+'.join(DROP), count=dropped)); log.append(dict(op='rewrite origin prefix', old=ORIGIN[0], new=ORIGIN[1], count=origins))
        data = ('\n'.join(out) + '\n').encode()
        assert origins == 14, origins
    else:
        t = src.decode()
        for old, new, n in SUBS.get(path, []):
            assert t.count(old) == n, (path, old, t.count(old)); t = t.replace(old, new); log.append(dict(op='replace', old=old, new=new, count=n))
        if path in LABELLED:
            t, k = LABEL[0].subn(LABEL[1], t); log.append(dict(op='relabel A-0N -> R5-0N', count=k))
        data = t.encode()
    assert data.count(b'\n') == src.count(b'\n'), (path, 'line count changed')
    probe = data.decode().replace('repro_unit_a.py::A-06', '')   # P1.9 Unit A test name (older reader review), kept as provenance
    assert not LEAK.search(probe), (path, LEAK.search(probe).group(0))
    return data


def encode(data, enc):
    text = data.decode(); fn = text.endswith('\n'); ls = text.split('\n')[:-1] if fn else text.split('\n')
    return '\n'.join(('%5d| %s' % (i + 1, l)) if enc == 'numbered' else l for i, l in enumerate(ls)), len(ls), fn


def materials():
    m133 = frozen(); res = []
    for f in m133['files']:
        src = open(f'{P133PK}/sources/{f["path"]}', 'rb').read(); assert sha(src) == f['sha256'], f['path']
        log = []; data = derive(f['path'], src, log); res.append((f, src, data, log))
    return m133, res


def build():
    m133, res = materials(); parts, man, idx, rmap, diffs = [], [], [], [], []
    for f, src, data, log in res:
        dst = f'{HERE}/sources/{f["path"]}'; os.makedirs(os.path.dirname(dst), exist_ok=True); open(dst, 'wb').write(data)
        body, n, fn = encode(data, f['encoding'])
        for mk in ('===== BEGIN FILE', '===== END FILE', '===== END OF REVIEW SET'): assert mk not in body
        parts.append(BEGIN.format(p=f['path'], h=sha(data), n=len(data), l=n, e=f['encoding']) + '\n' + body + '\n' + END.format(p=f['path']))
        role = f['role'].replace('(author narrative stripped)', '(author narrative and labels stripped)')
        man.append(dict(path=f['path'], sha256=sha(data), bytes=len(data), lines=n, encoding=f['encoding'], final_newline=fn, role=role, source_sha256=f['sha256']))
        idx.append(f'- `{f["path"]}`: {role} ({len(data)} bytes, {n} lines)')
        rmap.append(dict(path=f['path'], source_sha256=f['sha256'], derived_sha256=sha(data), identical=src == data, operations=log))
        if src != data:
            open('/dev/shm/.p134a', 'wb').write(src); open('/dev/shm/.p134b', 'wb').write(data)
            d = subprocess.run(['diff', '-u', '--label', f'p133/{f["path"]}', '--label', f'p134/{f["path"]}', '/dev/shm/.p134a', '/dev/shm/.p134b'], capture_output=True).stdout
            diffs.append(d.decode()); os.remove('/dev/shm/.p134a'); os.remove('/dev/shm/.p134b')
    brief = open(f'{HERE}/brief_p134.md').read()
    text = (brief.replace('{NFILES}', str(len(res))).replace('{NSAMPLE}', '56').replace('{NCASES}', '173').replace('{INDEX}', '\n'.join(idx))
            .replace('{DERIVE}', '\n'.join(f'- `{c}`' for c in DERIVE)).rstrip('\n') + '\n\n' + '\n\n'.join(parts) + '\n\n' + SENT.format(n=len(res)) + '\n\n' + REMINDER)
    assert '{' + 'NFILES}' not in text and '{DERIVE}' not in text
    open(f'{HERE}/prompt-oneshot.md', 'w').write(text); b = text.encode()
    open(f'{HERE}/redaction.diff', 'w').write(''.join(diffs))
    json.dump(rmap, open(f'{HERE}/REDACTION-MAP.json', 'w'), indent=1)
    json.dump(dict(prompt=dict(sha256=sha(b), bytes=len(b)), source_prompt_sha256=P133_PROMPT, files=man, brief_sha256=sha(brief.encode()),
                   builder_sha256=sha(open(__file__, 'rb').read()), derive=DERIVE), open(f'{HERE}/PACKET-MANIFEST.json', 'w'), indent=1)
    print(f'prompt {len(b)} bytes, {len(res)} files, sha256 {sha(b)}; changed files {sum(not r["identical"] for r in rmap)}')


def verify():
    _, res = materials(); text = open(f'{HERE}/prompt-oneshot.md').read(); man = json.load(open(f'{HERE}/PACKET-MANIFEST.json')); ok = True
    def check(c, m):
        nonlocal ok; print(('ok   ' if c else 'FAIL ') + m); ok &= bool(c)
    check(sha(text.encode()) == man['prompt']['sha256'], 'prompt digest matches manifest')
    check(man['brief_sha256'] == sha(open(f'{HERE}/brief_p134.md', 'rb').read()), 'brief digest matches manifest')
    check(len(re.findall(r'^===== BEGIN FILE ', text, re.M)) == len(res) == len(re.findall(r'^===== END FILE ', text, re.M)), f'{len(res)} delimiters')
    check(text.count(SENT.format(n=len(res))) == 1, 'sentinel once')
    for (f, src, data, log), m in zip(res, man['files']):
        mm = re.search(r'^' + re.escape(BEGIN.format(p=m['path'], h=m['sha256'], n=m['bytes'], l=m['lines'], e=m['encoding'])) + r'\n(.*?)\n' + re.escape(END.format(p=m['path'])) + '$', text, re.M | re.S)
        if not mm: check(False, f'{m["path"]}: block'); continue
        ls = mm.group(1).split('\n')
        good = True
        if m['encoding'] == 'numbered': good = all(l.startswith('%5d| ' % (i + 1)) for i, l in enumerate(ls)); ls = [l[7:] for l in ls]
        restored = ('\n'.join(ls) + ('\n' if m['final_newline'] else '')).encode()
        check(good and sha(restored) == m['sha256'] == sha(data) and m['source_sha256'] == f['sha256'], f'{m["path"]}: restores to {m["sha256"][:16]}, re-derived from {f["sha256"][:16]}')
    print('VERIFY', 'PASS' if ok else 'FAIL'); sys.exit(0 if ok else 1)


if __name__ == '__main__':
    {'build': build, 'verify': verify}[sys.argv[1]]()
