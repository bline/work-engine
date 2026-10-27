#!/usr/bin/env python3
"""Run registry: run directory -> Codex threads / Claude sessions -> cost and recorded outcomes
(F2, metrics/experiment-plan-20261026.md). Reads the frozen F1 ledger; never re-counts calls.

Usage
  python3 -I run_registry.py --ledger DIR --runs RUN_DIR [RUN_DIR ...] --out OUT

Outputs (OUT/)
  threads.jsonl   one Codex thread per line: rollout files, spawn parent (rollout and
                  state_5 thread_spawn_edges, with agreement), forked_from, inherited
                  starting counter, model/effort segments, resume gaps, cost.
  sessions.jsonl  one Claude session per line: transcript files across project
                  directories (relocations merged by sessionId), cwd, worktree,
                  continued-in links, fork refs, fallback model changes, model
                  segments, cost two ways (transcript-derived and CLI counter).
  runs.json       one entry per run directory: every link with its evidence, the
                  attribution class of each linked entity, cost buckets, recorded
                  outcome fields with sources.
  summary.json    counts and coverage.

Evidence classes for a link, strongest first
  declared    the id sits in a structured record of the run directory next to a
              role-like key (role, agent_path, actor, assignment) - role captured
  stream      a stream-json file in the run directory names the Claude session
  cwd         a Claude session's recorded cwd is inside the run directory
  inferred    a Codex thread descends (recorded parent links) from a non-top-level
              thread with a declared/stream link claimed by this run alone
  agent_path  a quoted Codex agent_path that exactly one thread carries
  listed      the id sits in a role-bearing record of a snapshot-like file (board,
              lineage, context, inventory, census, ...), or a non-assignment file
              names more than 20 agent paths: a listing, not a membership claim
  mentioned   the id appears in a run file without role context (also
              agent_path_ambiguous: a path several threads carry)
Claude entities are sessions (main-thread calls only) and subagents (agent id
`a` + 16 hex, their own calls), so no call is counted at both levels.
Directories holding a repository checkout (AGENTS.md, .git, Cargo.toml,
ARCHITECTURE.md) and copied metrics/ trees are not scanned: they carry other
runs' evidence.

Claude cost is reported two ways and never merged: transcript-derived (main thread, and
with the session's subagents) and the CLI counter (max over the session's cost-state
records). For interactive sessions the counter can exceed the transcripts; that gap is
open (F1 record) and is not treated as established spend.

Attribution of an entity's cost - never forced
  attributed   declared/stream/cwd links to exactly one run
  windowed     a declared record gives a time window; only calls inside any of the
               run's windows count, the rest of the entity stays unattributed
               (windowed_shared when another run also claims the entity)
  shared       declared/stream/cwd links to more than one run: listed in each, summed
               in none
  inferred     only inferred links, to exactly one run: listed separately, not summed
               into attributed
  candidate    only mentioned links: listed, not summed
Each run also carries its observed environment: Codex CLI version and repository commit at
thread start, Claude Code versions, over its attributed/windowed/shared entities. Which
instruction files the agents loaded was not recorded and stays not established.
Outcome fields are copied verbatim with their source path and sha256; nothing is
adjudicated, and a run without an outcome record says `not established`.
"""
import argparse, collections, datetime as dt, glob, hashlib, json, os, re, sqlite3, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import usage_ledger as L   # frozen F1 readers; imported, not modified

UUID = re.compile(r'\b[0-9a-f]{8}-[0-9a-f]{4}-[47][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b')
AGENT_ID = re.compile(r'\ba[0-9a-f]{16}\b')                 # Claude Code subagent id
AGENT_PATH = re.compile(r'"(/root(?:/[A-Za-z0-9_.-]+)+)"')   # quoted Codex agent_path
ROLE_KEYS = ('role', 'agent_path', 'actor', 'assignment', 'agent_role')
# subject keys only: relational fields (parent_thread_id, root_thread) describe the subject's neighbours
ID_KEYS = ('session', 'session_id', 'sessionId', 'thread', 'thread_id', 'threadId', 'id', 'child_thread_id',
           'native_session', 'codex_thread', 'agent', 'agent_id', 'agentId')
LISTING_FILE = re.compile(r'(board|lineage|context|inventory|census|snapshot|observation|status|agents|tree|watch|'
                          r'registry|manifest)', re.I)
LISTING_FANOUT = 20   # loose agent_path mentions: a non-assignment file naming more paths than this is a listing
SKIP_DIRS = {'target', '.git', 'node_modules', 'source', 'worktree', 'candidate-src', 'review-root', 'repository',
             'final-review-repository', 'src', 'rust', 'app-server', 'docs', 'skills', 'source-snapshot', 'packet-src'}
TEXT_EXT = ('.json', '.jsonl', '.md', '.txt')
OUTCOME_FILE = re.compile(r'(result|acceptance|score|outcome|final-metrics|verdict|disposition|comparison|quality|'
                          r'terminal|completion|RESULT)[^/]*\.json$')
OUTCOME_KEYS = ('status', 'outcome', 'decision', 'result', 'conclusion', 'disposition', 'verdict', 'review_conclusion',
                'integration_decision', 'accepted', 'final_tests', 'tests', 'test_totals', 'findings', 'review_findings',
                'interpretation', 'limitations', 'next_action')
RESUME_GAP_S = 4 * 3600   # same threshold as the ledger's M20 resume boundary

def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as fh:
        for b in iter(lambda: fh.read(1 << 20), b''): h.update(b)
    return h.hexdigest()

def segments(rows, key):
    """Consecutive runs of the same value, with first/last timestamp and call count."""
    out = []
    for r in rows:
        v = key(r)
        if out and out[-1]['value'] == v:
            out[-1]['last'] = r['ts']; out[-1]['calls'] += 1
        else:
            out.append(dict(value=v, first=r['ts'], last=r['ts'], calls=1))
    return out

def gaps(rows):
    out = []
    for a, b in zip(rows, rows[1:]):
        d = (L.ts_of(b['ts']) - L.ts_of(a['ts'])).total_seconds()
        if d >= RESUME_GAP_S: out.append(dict(after=a['ts'], resumed_at=b['ts'], gap_hours=round(d / 3600, 2)))
    return out

def cost(rows):
    c = collections.Counter(); unpriced = collections.Counter()
    for r in rows:
        if r['usd'] is None: unpriced[r['model']] += 1
        else: c['usd'] += r['usd']
        c['calls'] += 1; c['weighted'] += r['weighted']
    return dict(calls=c['calls'], usd=round(c['usd'], 6), weighted=round(c['weighted'], 1),
                unpriced_calls=dict(unpriced))

# ------------------------------------------------------------------ entities
def load_ledger(d):
    rows = collections.defaultdict(list)
    for line in open(os.path.join(d, 'calls.jsonl')):
        r = json.loads(line)
        if r['provider'] == 'codex': rows[('codex', r['thread'])].append(r)
        elif r.get('agent'): rows[('anthropic-agent', r['agent'][len('agent-'):])].append(r)
        else: rows[('anthropic', r['session'])].append(r)
    for v in rows.values(): v.sort(key=lambda r: r['ts'] or '')
    l1 = json.load(open(os.path.join(d, 'level1.json')))
    return rows, l1

def codex_threads(rows, l1):
    db = {}
    try:
        con = sqlite3.connect(f'file:{L.STATE_DB}?mode=ro', uri=True)
        edges = collections.defaultdict(set)
        for p, c, _ in con.execute('select parent_thread_id, child_thread_id, status from thread_spawn_edges'): edges[c].add(p)
        for tid, ap, role, model, eff in con.execute('select id, agent_path, agent_role, model, reasoning_effort from threads'):
            db[tid] = dict(agent_path=ap, agent_role=role, index_model=model, index_effort=eff, spawn_edge_parents=sorted(edges.get(tid, ())))
    except sqlite3.Error as e:
        L.issue('thread_index_unreadable', error=str(e))
    offsets = {x['thread']: x.get('starting_offset') for x in l1.get('codex_threads', [])}
    metas = collections.defaultdict(dict); files = collections.defaultdict(list)
    for f in L.codex_files(None):
        for line in L.open_lines(f):
            if '"session_meta"' not in line[:200]: break
            p = json.loads(line).get('payload') or {}
            tid = p.get('id'); files[tid].append(f)
            if not metas[tid]:
                src = p.get('source')
                spawn = (src or {}).get('subagent', {}).get('thread_spawn', {}) if isinstance(src, dict) else {}
                metas[tid] = dict(parent_thread_id=p.get('parent_thread_id'), forked_from_id=p.get('forked_from_id'),
                                  agent_path=p.get('agent_path'), originator=p.get('originator'), thread_source=p.get('thread_source'),
                                  created=p.get('timestamp'), spawn_depth=spawn.get('depth'), cli_version=p.get('cli_version'),
                                  repo_commit=(p.get('git') or {}).get('commit_hash'), git_branch=(p.get('git') or {}).get('branch'))
            break
    out = {}
    for tid in set(metas) | {k[1] for k in rows if k[0] == 'codex'}:
        m = metas.get(tid, {}); r = rows.get(('codex', tid), []); d = db.get(tid, {})
        par = m.get('parent_thread_id'); ep = d.get('spawn_edge_parents', [])
        out[tid] = dict(thread=tid, rollouts=sorted(files.get(tid, [])), created=m.get('created'), agent_path=m.get('agent_path'),
                        cli_version=m.get('cli_version'), repo_commit_at_start=m.get('repo_commit'), git_branch_at_start=m.get('git_branch'),
                        originator=m.get('originator'), thread_source=m.get('thread_source'), spawn_depth=m.get('spawn_depth'),
                        parent_thread_id=par, spawn_edge_parents=ep,
                        parent_agreement=('no_parent' if not par and not ep else 'agree' if par in ep else 'rollout_only' if par and not ep
                                          else 'index_only' if ep and not par else 'disagree'),
                        forked_from_id=m.get('forked_from_id'), inherited_starting_counter=offsets.get(tid),
                        index_role=d.get('agent_role'), index_model_effort=[d.get('index_model'), d.get('index_effort')],
                        model_effort_segments=segments(r, lambda x: [x['model'], x['effort']]),
                        resume_gaps=gaps(r), cost=cost(r), first_call=r[0]['ts'] if r else None, last_call=r[-1]['ts'] if r else None)
    return out

def claude_sessions(rows, l1):
    sub_rows = collections.defaultdict(list)   # a session's subagent calls, for the two-way comparison
    for (prov, aid), r in rows.items():
        if prov == 'anthropic-agent': sub_rows[r[0]['session']].extend(r)
    sess = collections.defaultdict(lambda: dict(files=set(), cwds=set(), continued_in=set(), continued_from=set(),
                                                versions=collections.Counter(), branches=set(),
                                                fork_parent_of_subagent=set(), worktrees=set(), relocated_cwds=set(),
                                                model_fallbacks=[], cost_state={}))
    for f in sorted(glob.glob(L.CLAUDE_GLOB, recursive=True)):
        sub = '/subagents/' in f
        for line in L.open_lines(f):
            if not any(k in line for k in ('"sessionId"', 'fork-context-ref', 'continued-in')): continue
            try: o = json.loads(line)
            except ValueError: continue
            sid = o.get('sessionId')
            if not sid: continue
            s = sess[sid]; s['files'].add(f)
            if o.get('cwd'): s['cwds'].add(o['cwd'])
            if o.get('version'): s['versions'][o['version']] += 1
            if o.get('gitBranch'): s['branches'].add(o['gitBranch'])
            t = o.get('type')
            if t == 'continued-in' and o.get('continuedInSessionId'):
                s['continued_in'].add(o['continuedInSessionId']); sess[o['continuedInSessionId']]['continued_from'].add(sid)
            elif t == 'fork-context-ref' and o.get('parentSessionId'):
                s['fork_parent_of_subagent'].add(o['parentSessionId'])
            elif t == 'relocated' and o.get('relocatedCwd'):
                s['relocated_cwds'].add(o['relocatedCwd'])
            elif t == 'worktree-state':
                w = o.get('worktreeSession') or {}
                if w.get('worktreePath'): s['worktrees'].add(w['worktreePath'])
            elif t == 'system' and o.get('subtype') == 'model_refusal_fallback':
                s['model_fallbacks'].append(dict(ts=o.get('timestamp'), original=o.get('originalModel'),
                                                 fallback=o.get('fallbackModel'), trigger=o.get('trigger'), subagent_file=sub))
            elif t == 'cost-state':
                s['cost_state'][f] = {m: v.get('costUSD') for m, v in (o.get('modelUsage') or {}).items()}
    l1c = collections.defaultdict(list)
    for x in l1.get('claude_sessions', []): l1c[x['session']].append(x['cls'])
    out = {}
    for sid in set(sess) | {k[1] for k in rows if k[0] == 'anthropic'}:
        s = sess.get(sid) or sess[sid]; r = rows.get(('anthropic', sid), [])
        dirs = sorted({f.split('/.claude/projects/')[1].split('/')[0] for f in s['files']})
        # the CLI counter: the last cost-state per transcript file; files of one session are not summed when
        # one continues another, so both the per-file values and their max are reported
        cs_files = {f: round(sum(v or 0 for v in m.values()), 6) for f, m in s['cost_state'].items()}
        out[sid] = dict(session=sid, project_dirs=dirs, transcripts=sorted(s['files']), cwds=sorted(s['cwds']),
                        claude_code_versions=dict(s['versions']), git_branches=sorted(s['branches']),
                        relocated_cwds=sorted(s['relocated_cwds']), worktrees=sorted(s['worktrees']),
                        continued_in=sorted(s['continued_in']), continued_from=sorted(s['continued_from']),
                        fork_parent_of_subagent=sorted(s['fork_parent_of_subagent']), model_fallbacks=s['model_fallbacks'],
                        model_segments=segments(r, lambda x: [x['model'], x.get('effort')]), resume_gaps=gaps(r),
                        cost_transcript=cost(r), cost_transcript_with_subagents=cost(r + sub_rows.get(sid, [])),
                        cost_cli_counter_by_file=cs_files,
                        cost_cli_counter_max=max(cs_files.values()) if cs_files else None,
                        level1_classes=sorted(set(l1c.get(sid, []))) or None,
                        first_call=r[0]['ts'] if r else None, last_call=r[-1]['ts'] if r else None)
    return out

def claude_agents(rows):
    out = {}
    for (prov, aid), r in rows.items():
        if prov != 'anthropic-agent': continue
        out[aid] = dict(agent=aid, session=r[0]['session'], agent_type=r[0].get('agent_type'), project=r[0].get('project'),
                        model_segments=segments(r, lambda x: [x['model'], x.get('effort')]), cost=cost(r),
                        first_call=r[0]['ts'], last_call=r[-1]['ts'])
    return out

# ------------------------------------------------------------------ run links
REPO_MARKERS = ('AGENTS.md', '.git', 'Cargo.toml', 'ARCHITECTURE.md')
def repo_copy(path):
    """A directory that is a repository checkout, or a copied metrics/ tree, holds other runs' evidence."""
    if any(os.path.exists(os.path.join(path, m)) for m in REPO_MARKERS): return True
    return os.path.basename(path) == 'metrics' and any(os.path.isdir(os.path.join(path, d)) for d in ('probes', 'experiments', 'scripts'))

def walk_run(run):
    for root, dirs, fnames in os.walk(run):
        depth = root[len(run):].count(os.sep)
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS and depth < 4 and not repo_copy(os.path.join(root, d))]
        for fn in fnames:
            p = os.path.join(root, fn)
            if fn.endswith('.stream.jsonl'): yield p, 'stream'
            elif fn.endswith(TEXT_EXT):
                try:
                    if os.path.getsize(p) <= 2_000_000: yield p, 'text'
                except OSError: pass

def json_records(obj, path=''):
    if isinstance(obj, dict):
        yield path, obj
        for k, v in obj.items(): yield from json_records(v, f'{path}/{k}')
    elif isinstance(obj, list):
        for i, v in enumerate(obj): yield from json_records(v, f'{path}/{i}')

def links_for_run(run, threads, sessions, agents, by_path):
    links = collections.defaultdict(lambda: dict(evidence=[], roles=set(), windows=[]))
    known = lambda i: ('codex' if i in threads else 'anthropic' if i in sessions else 'anthropic-agent' if i in agents else None)
    scanned = 0; unknown_ids = set()
    for p, kind in walk_run(run):
        scanned += 1
        rel = os.path.relpath(p, run)
        if kind == 'stream':
            for n, line in enumerate(L.open_lines(p)):
                if n > 50: break
                m = re.search(r'"session_id"\s*:\s*"([0-9a-f-]{36})"', line)
                if m and known(m.group(1)) == 'anthropic':
                    links[('anthropic', m.group(1))]['evidence'].append(dict(cls='stream', source=rel)); break
            continue
        try: text = open(p, encoding='utf-8', errors='replace').read()
        except OSError: continue
        ids = set(UUID.findall(text)) | set(AGENT_ID.findall(text))
        paths = set(AGENT_PATH.findall(text)) & set(by_path)
        path_listing = bool(LISTING_FILE.search(os.path.basename(p))) or (len(paths) > LISTING_FANOUT
                                                                            and 'assign' not in os.path.basename(p))
        for ap in paths:   # a Codex agent_path names a thread only when exactly one thread carries it
            tids = by_path[ap]
            for t in tids:
                cls = 'listed' if path_listing else 'agent_path' if len(tids) == 1 else 'agent_path_ambiguous'
                links[('codex', t)]['evidence'].append(dict(cls=cls, source=rel, agent_path=ap, threads_with_path=len(tids)))
                if cls == 'agent_path': links[('codex', t)]['roles'].add(ap)
        if not ids: continue
        declared = set()
        if p.endswith('.json'):
            try: doc = json.loads(text)
            except ValueError: doc = None
            found = []
            for jp, rec in json_records(doc) if doc is not None else ():
                roleish = {k: rec[k] for k in ROLE_KEYS if isinstance(rec.get(k), str)}
                if not roleish: continue
                for k in ID_KEYS:   # the record's own subject: first subject key that names a known entity
                    v = rec.get(k)
                    if isinstance(v, str) and known(v):
                        found.append((v, k, jp, roleish, rec)); break
            listing = bool(LISTING_FILE.search(os.path.basename(p)))
            for v, k, jp, roleish, rec in found:
                e = links[(known(v), v)]
                e['evidence'].append(dict(cls='listed' if listing else 'declared', source=f'{rel}#{jp}', key=k))
                declared.add(v)
                if listing: continue
                e['roles'].update(roleish.values())
                if rec.get('start_boundary') or rec.get('cutoff'):
                    e['windows'].append(dict(start=rec.get('start_boundary'), end=rec.get('cutoff'), source=f'{rel}#{jp}'))
        for i in ids - declared:
            k = known(i)
            if k: links[(k, i)]['evidence'].append(dict(cls='mentioned', source=rel))
            else: unknown_ids.add(i)
    # cwd links: Claude sessions whose recorded cwd lies inside the run directory
    for sid, s in sessions.items():
        if any(c == run or c.startswith(run + '/') for c in s['cwds'] + s['relocated_cwds']):
            links[('anthropic', sid)]['evidence'].append(dict(cls='cwd', source=next(c for c in s['cwds'] + s['relocated_cwds']
                                                                                         if c == run or c.startswith(run + '/'))))
    return links, scanned, len(unknown_ids)

STRONG = ('declared', 'stream', 'cwd', 'agent_path')
def strongest(ev):
    for c in ('declared', 'stream', 'cwd', 'agent_path', 'inferred', 'listed', 'agent_path_ambiguous', 'mentioned'):
        if any(e['cls'] == c for e in ev): return c

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--ledger', required=True); ap.add_argument('--runs', nargs='+', required=True); ap.add_argument('--out', required=True)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    rows, l1 = load_ledger(a.ledger)
    threads = codex_threads(rows, l1); sessions = claude_sessions(rows, l1); agents = claude_agents(rows)
    by_path = collections.defaultdict(list)
    for t in threads.values():
        if t['agent_path']: by_path[t['agent_path']].append(t['thread'])
    runs = [os.path.abspath(r) for r in a.runs if os.path.isdir(r)]
    per_run = {}
    for run in runs:
        links, scanned, unknown = links_for_run(run, threads, sessions, agents, by_path)
        per_run[run] = dict(links=links, scanned=scanned, unknown_ids=unknown)
    # inferred links: descendants of a thread with a declared/stream link, when no other run claims that ancestor
    children = collections.defaultdict(set)
    for t in threads.values():
        for p in {t['parent_thread_id'], *t['spawn_edge_parents']} - {None}: children[p].add(t['thread'])
    strong_runs = collections.defaultdict(set)
    for run, pr in per_run.items():
        for (prov, i), e in pr['links'].items():
            if any(x['cls'] in STRONG for x in e['evidence']): strong_runs[(prov, i)].add(run)
    for run, pr in per_run.items():
        # top-level threads coordinate many runs, so they never anchor inference
        anchors = [i for (prov, i), e in pr['links'].items() if prov == 'codex'
                   and any(x['cls'] in ('declared', 'stream') for x in e['evidence']) and strong_runs[('codex', i)] == {run}
                   and threads.get(i, {}).get('parent_thread_id')]
        seen = set(anchors); stack = list(anchors)
        while stack:
            cur = stack.pop()
            for c in children.get(cur, ()):
                if c in seen: continue
                seen.add(c); stack.append(c)
                pr['links'][('codex', c)]['evidence'].append(dict(cls='inferred', source=f'descendant of {cur}'))
    # attribution
    claims = collections.defaultdict(set)
    for run, pr in per_run.items():
        for key, e in pr['links'].items():
            if strongest(e['evidence']) in STRONG: claims[key].add(run)
    inferred_claims = collections.defaultdict(set)
    for run, pr in per_run.items():
        for key, e in pr['links'].items():
            if strongest(e['evidence']) == 'inferred': inferred_claims[key].add(run)
    out_runs = []
    for run, pr in per_run.items():
        buckets = collections.defaultdict(lambda: collections.Counter()); entries = []
        for (prov, i), e in sorted(pr['links'].items()):
            ent = threads.get(i) if prov == 'codex' else sessions.get(i) if prov == 'anthropic' else agents.get(i)
            sc = strongest(e['evidence'])
            if sc in STRONG:
                single = len(claims[(prov, i)]) == 1
                if e['windows']: cls = 'windowed' if single else 'windowed_shared'
                else: cls = 'attributed' if single else 'shared'
            elif sc == 'inferred': cls = 'inferred' if len(inferred_claims[(prov, i)]) == 1 else 'shared'
            else: cls = 'candidate'
            c = ent['cost_transcript'] if prov == 'anthropic' else ent['cost']
            if cls in ('windowed', 'windowed_shared'):   # calls inside any declared window, each call once
                ws = {(w['start'], w['end']) for w in e['windows']}
                rws = [r for r in rows.get((prov, i), [])
                       if any(L.in_window(r['ts'], L.ts_of(st), L.ts_of(en)) for st, en in ws)]
                c = cost(rws)
            b = buckets[f'{prov}:{cls}']; b['entities'] += 1; b['calls'] += c['calls']; b['usd'] += c['usd']
            if prov == 'anthropic':
                b['with_subagents_usd'] += ent['cost_transcript_with_subagents']['usd']
                if ent.get('cost_cli_counter_max') is not None: b['cli_counter_usd'] += ent['cost_cli_counter_max']
            entries.append(dict(provider=prov, id=i, attribution=cls, strongest_evidence=sc, roles=sorted(e['roles']),
                                windows=e['windows'], other_runs=sorted(os.path.basename(x) for x in claims[(prov, i)] - {run}),
                                evidence=e['evidence'][:12], evidence_count=len(e['evidence']), cost=c,
                                cli_counter_usd=ent.get('cost_cli_counter_max') if prov == 'anthropic' else None,
                                transcript_with_subagents_usd=ent['cost_transcript_with_subagents']['usd'] if prov == 'anthropic' else None,
                                session=ent.get('session') if prov == 'anthropic-agent' else None))
        outcomes = []
        for root, dirs, fnames in os.walk(run):
            if root[len(run):].count(os.sep) >= 2: dirs[:] = []
            dirs[:] = [d for d in dirs if d not in SKIP_DIRS and not repo_copy(os.path.join(root, d))]
            for fn in fnames:
                p = os.path.join(root, fn)
                if fn == 'experiment-result.md': outcomes.append(dict(source=os.path.relpath(p, run), sha256=sha(p), kind='markdown'))
                if not OUTCOME_FILE.search(fn) or os.path.getsize(p) > 2_000_000: continue
                try: doc = json.load(open(p))
                except (ValueError, OSError): continue
                if not isinstance(doc, dict): continue
                fields = {k: doc[k] for k in OUTCOME_KEYS if k in doc}
                if fields:
                    outcomes.append(dict(source=os.path.relpath(p, run), sha256=sha(p),
                                         fields={k: (v if len(json.dumps(v)) <= 600 else json.dumps(v)[:600] + '...') for k, v in fields.items()}))
        env = dict(codex_cli_versions=collections.Counter(), repo_commits_at_thread_start=collections.Counter(),
                   claude_code_versions=collections.Counter())
        for e in entries:   # observed environment of the entities this run owns or shares with strong evidence
            if e['attribution'] not in ('attributed', 'windowed', 'windowed_shared', 'shared'): continue
            if e['provider'] == 'codex':
                t = threads.get(e['id'], {})
                env['codex_cli_versions'][t.get('cli_version')] += 1
                env['repo_commits_at_thread_start'][(t.get('repo_commit_at_start') or 'none')[:10]] += 1
            elif e['provider'] == 'anthropic':
                for v in sessions.get(e['id'], {}).get('claude_code_versions', {}): env['claude_code_versions'][v] += 1
            else:
                ses = sessions.get(agents.get(e['id'], {}).get('session'), {})
                for v in ses.get('claude_code_versions', {}): env['claude_code_versions'][v] += 1
        environment = {k: dict(v) for k, v in env.items()}
        environment['mixed'] = any(len(v) > 1 for v in env.values())
        environment['instruction_files'] = 'not established: skills and instructions were loaded from the live tree; not recorded per run'
        out_runs.append(dict(run=os.path.basename(run), path=run, files_scanned=pr['scanned'], unknown_uuid_mentions=pr['unknown_ids'],
                             environment_observed=environment,
                             cost_buckets={k: dict(entities=v['entities'], calls=v['calls'], usd=round(v['usd'], 4),
                                                   **({'transcript_with_subagents_usd': round(v['with_subagents_usd'], 4),
                                                       'cli_counter_usd': round(v['cli_counter_usd'], 4)}
                                                      if k.startswith('anthropic:') else {}))
                                           for k, v in sorted(buckets.items())},
                             outcome='not established' if not outcomes else 'recorded', outcome_records=outcomes, links=entries))
    with open(os.path.join(a.out, 'threads.jsonl'), 'w') as fh:
        for t in sorted(threads.values(), key=lambda t: t['thread']): fh.write(json.dumps(t, sort_keys=True) + '\n')
    with open(os.path.join(a.out, 'sessions.jsonl'), 'w') as fh:
        for s in sorted(sessions.values(), key=lambda s: s['session']): fh.write(json.dumps(s, sort_keys=True) + '\n')
    with open(os.path.join(a.out, 'agents.jsonl'), 'w') as fh:
        for g in sorted(agents.values(), key=lambda g: g['agent']): fh.write(json.dumps(g, sort_keys=True) + '\n')
    json.dump(dict(ledger=os.path.abspath(a.ledger), runs=out_runs), open(os.path.join(a.out, 'runs.json'), 'w'), indent=1, sort_keys=True)
    multi_claim = sum(1 for v in claims.values() if len(v) > 1)
    summary = dict(ledger=os.path.abspath(a.ledger), runs=len(out_runs), codex_threads=len(threads), claude_sessions=len(sessions),
                   claude_subagents=len(agents), agent_paths_shared_by_threads=sum(len(v) > 1 for v in by_path.values()),
                   threads_forked=sum(bool(t['forked_from_id']) for t in threads.values()),
                   threads_with_inherited_counter=sum(bool(t['inherited_starting_counter']) for t in threads.values()),
                   threads_parent_agreement=collections.Counter(t['parent_agreement'] for t in threads.values()),
                   threads_with_model_change=sum(len(t['model_effort_segments']) > 1 for t in threads.values()),
                   threads_with_resume_gap=sum(bool(t['resume_gaps']) for t in threads.values()),
                   sessions_multi_dir=sum(len(s['project_dirs']) > 1 for s in sessions.values()),
                   sessions_continued=sum(bool(s['continued_in']) for s in sessions.values()),
                   sessions_with_fallback=sum(bool(s['model_fallbacks']) for s in sessions.values()),
                   sessions_with_model_change=sum(len({tuple(x['value'][:1]) for x in s['model_segments']}) > 1 for s in sessions.values()),
                   entities_claimed_by_multiple_runs=multi_claim,
                   runs_without_outcome=[r['run'] for r in out_runs if r['outcome'] == 'not established'],
                   runs_with_mixed_environment=sum(r['environment_observed']['mixed'] for r in out_runs),
                   attribution_totals={k: sum(1 for r in out_runs for e in r['links'] if e['attribution'] == k)
                                       for k in ('attributed', 'windowed', 'windowed_shared', 'shared', 'inferred', 'candidate')})
    json.dump(summary, open(os.path.join(a.out, 'summary.json'), 'w'), indent=1, sort_keys=True)
    print(json.dumps(summary, indent=1, sort_keys=True))

if __name__ == '__main__':
    main()
