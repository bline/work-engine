"""Write corpus/COVERAGE.md: every independently confirmed finding P1.9-P1.27 with its corpus cases or the reason it is
outside S1. Usage: coverage.py <corpus_dir>"""
import glob, json, os, sys
corpus = sys.argv[1]
cs = [json.load(open(p)) for p in sorted(glob.glob(os.path.join(corpus, 'cases', '*.json')))]
FINDINGS = [  # (id, round, summary, scope note if not a corpus case)
 ('A-01', 'P1.9', 'forgeable owner reconciliation string zeroes a charge', 'out of S1: the reconciliation interface was removed (P1.10); same-UID trust is `same_uid`'),
 ('A-02', 'P1.9', 'last-append-wins reconciliation replay', 'out of S1: interface removed (P1.10)'),
 ('A-03', 'P1.9', 'reconciliation file without writer protocol', 'out of S1: interface removed (P1.10)'),
 ('A-04', 'P1.9', 'undated reservation settlement dropped', None),
 ('A-05', 'P1.9', 'rollback to the baseline reader reintroduces the undercount', 'out of S1: operational (installed reader identity, `unit_c`)'),
 ('A-06', 'P1.9', 'reservation-settled row with usable usage loses its low bound', None),
 ('A-07', 'P1.9', 'one truncated reconciliation line blocks every objective', 'out of S1: interface removed (P1.10)'),
 ('A1-01', 'P1.11', 'a non-request settlement erases its reservation; unknown event with usd', None),
 ('A1-02', 'P1.11', 'inverted neighbour bounds exclude an undated charge', None),
 ('A1-03', 'P1.11', 'missing turn record hides its log', None),
 ('A1-04', 'P1.11', 'halt only inside the replaceable reader', 'out of S1: `unit_c` (enforcement outside the reducer)'),
 ('A1-05', 'P1.11', 'reservation row with priceable usage and null status loses its low bound', None),
 ('A1-06', 'P1.11', 'legacy OpenAI row takes cache writes from the writable session file', None),
 ('R-01', 'P1.13', 'dated clock conflict erases a reservation', None),
 ('R-02', 'P1.13', 'string charge cancels a reservation', None),
 ('R-03', 'P1.13', 'unreferenced log ignored when another log has the turn number', None),
 ('R-04', 'P1.13', 'reused reservation id overwrites exposure', None),
 ('R-05', 'P1.13', 'valid JSON that is not an object skipped', None),
 ('R-06', 'P1.13', 'lost launch row detaches a child', None),
 ('R-07', 'P1.13', 'halt and approved reader not enforced outside the reader', 'out of S1: `unit_c`'),
 ('R-08', 'P1.13', 'shared-root objectives see spend but not commitments (XO-1)', 'partly: explicit predecessor links only (D1, 12.2); shared roots without a link remain open (owner question Q1). Related case: OWN-budgeted-session-without-admission'),
 ('R-09', 'P1.13', 'an agent can raise an objective limit', 'out of S1: `same_uid`'),
 ('S-01', 'P1.15', 'under-charged reservation/upstream settlement accepted', None),
 ('S-02', 'P1.15', 'settlement before its reservation, or with another n', None),
 ('S-03', 'P1.15', 'future-stamped settlement consumes and drops a reservation', None),
 ('S-04', 'P1.15', 'unrelated next row closes an unsettled interval', None),
 ('S-05', 'P1.15', 'issue scoping by path, not session', None),
 ('S-06', 'P1.15', 'string http_status passes validation', None),
 ('S-07', 'P1.15', 'oracle repeats the implementation assumptions', 'addressed by corpus design (C.4 independence; validator self-tests), not a case'),
 ('S-08', 'P1.15', 'negative admission-journal cap accepted', None),
 ('C1', 'P1.19', 'window placement checks endpoints, not interval overlap', None),
 ('C2', 'P1.19', 'rows ahead of the reference time excluded without blocking', None),
 ('C3', 'P1.19', 'exact reservation replay after settlement collapsed', None),
 ('C4', 'P1.19', 'missing charge_basis inferred', None),
 ('C5', 'P1.19', 'partial final line read on API logs (= U-02)', None),
 ('U-01', 'P1.21', 'malformed attempt number releases', None),
 ('U-02', 'P1.21', 'unterminated final API line read as complete', None),
 ('U-03', 'P1.21', 'retirement stamp before later proxy evidence', None),
 ('U-04', 'P1.21', 'closed window drops an interval crossing closure', None),
 ('U-05', 'P1.21', 'usage settlement without response id', None),
 ('U-06', 'P1.21', 'launch lane hides an API upstream error', None),
 ('U-07', 'P1.21', 'unrelated malformed usage aborts evaluation', None),
 ('U-08', 'P1.21', 'producer-supported missing-tier settlement rejected', None),
 ('V-01', 'P1.23', 'cheaper response model lowers the consistency floor', None),
 ('V-02', 'P1.23', 'Anthropic unsplit writes accepted at the cheaper rate', None),
 ('V-03', 'P1.23', 'subscription event with contradicting labels admitted', None),
 ('V-04', 'P1.23', 'lone header-write stream_error unaccounted', None),
 ('V-05', 'P1.23', 'reservation settlement without id loses its low bound', None),
 ('V-06', 'P1.23', 'unsettled reservation with missing n counted without issue', None),
 ('V-07', 'P1.23', 'unrelated unreadable log blocks every objective', None),
 ('V-08', 'P1.23', 'test expectations derived from the reader', 'addressed by corpus design (no reader-derived expectation); not a case'),
 ('V-09', 'P1.23', 'release test fixture invalid', 'out of S1: release-suite fixture (v2a), not accounting semantics'),
 ('N-01', 'P1.25', 'identity diagnostic trips the reservation fallback (regression in A1.3.3)', None),
 ('N-02', 'P1.25', 'non-string Anthropic tier read as standard', None),
 ('N-03', 'P1.25', 'lane check misses admission/refused/identity-refusal rows', None),
 ('N-04', 'P1.25', 'reused n hides a stream_error', None),
 ('N-05', 'P1.25', 'earlier retirement skipped in the chronology', None),
 ('N-06', 'P1.25', 'null cached count blocks a producer-supported settlement', None),
 ('N-07', 'P1.25', 'reopened session keeps a released commitment', None),
 ('N-08', 'P1.25', 'reader-only Haiku rate', None),
 ('N-09', 'P1.25', '= V-08', 'see V-08'),
 ('P1.26-P2', 'P1.26', 'unpriced usage settlement drops its charge', None),
 ('P1.26-P8', 'P1.26', 'duplicate response id drops its reservation', None),
 ('P1.26-P9', 'P1.26', 'contradicted settlement claims above the reservation', None),
 ('P1.26-P10', 'P1.26', 'pricing depends on placement', None),
 ('P1.26-E-4', 'P1.26', 'duplicated read-failure stream_error', None),
 ('H-a', 'P1.27', 'non-identity diagnostic still selects the fallback (low → 0)', None),
 ('H-b', 'P1.27', 'validation depends on the objective window', None),
]
lines = ['# Historical-reproduction coverage matrix (corpus `pi-accounting-corpus/1`)', '',
         'Every independently confirmed finding from P1.9 to P1.27, with the corpus cases that encode it (finding and control) or the reason it is outside S1. '
         'Original scripts and outputs are preserved unchanged in `historical/originals/` (`historical/ORIGINALS.sha256`). Generated by `tools/coverage.py`.', '',
         '| Finding | Round | Summary | Corpus cases | Controls | Scope |', '|---|---|---|---|---|---|']
missing = []
for fid, rnd, summ, scope in FINDINGS:
    cases = [c for c in cs if fid in c['findings'] and c['role'] == 'finding']
    ctrls = sorted({x for c in cases for x in c['controls']} | {c['case_id'] for c in cs if fid in c['findings'] and c['role'] != 'finding'})
    if not cases and not scope: missing.append(fid)
    lines.append(f"| {fid} | {rnd} | {summ} | {', '.join(c['case_id'] for c in cases) or '—'} | {', '.join(ctrls) or '—'} | {scope or 'in S1'} |")
lines += ['', f'In-scope findings without a case: {missing or "none"}.', '',
          f'Corpus: {len(cs)} cases. Owner-decision cases: ' + ', '.join(sorted({c["case_id"] for c in cs if any(f.startswith("D") and len(f) <= 3 for f in c["findings"])}))]
open(os.path.join(corpus, 'COVERAGE.md'), 'w').write('\n'.join(lines) + '\n')
print('missing in-scope:', missing)
