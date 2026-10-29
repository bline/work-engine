#!/usr/bin/env python3
"""Foreground, same-thread continuation controller. See same-session-context.md."""
from __future__ import annotations
import argparse, contextlib, fcntl, hashlib, json, os, shlex, signal, sys, time, uuid
from pathlib import Path
sys.dont_write_bytecode = True
import codex_app
import role_host
import child_context

class Refusal(Exception):
    def __init__(self, detail, status='failure'):
        super().__init__(detail); self.status=status

class PendingBoundary(Exception): pass

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def absolute(value):
    p = Path(value)
    if not p.is_absolute() or p.resolve() != p:
        raise Refusal(f'absolute normalized path required: {value}')
    return p

def syncwrite(path, data):
    path = Path(path); tmp = path.with_name(path.name + '.' + str(os.getpid()) + '.tmp')
    with tmp.open('w') as f:
        json.dump(data, f, sort_keys=True, indent=2); f.write('\n'); f.flush(); os.fsync(f.fileno())
    os.replace(tmp, path)
    fd = os.open(path.parent, os.O_RDONLY); os.fsync(fd); os.close(fd)

class Store:
    def __init__(self, root):
        self.root = absolute(root); self.path = self.root / 'operation.json'; self.lockpath = self.root / 'lock'
    @contextlib.contextmanager
    def lock(self, blocking=True):
        self.root.mkdir(parents=True, exist_ok=True)
        with self.lockpath.open('a') as f:
            try: fcntl.flock(f, fcntl.LOCK_EX | (0 if blocking else fcntl.LOCK_NB))
            except BlockingIOError: raise Refusal('operation is already controlled')
            yield
    def read(self): return json.loads(self.path.read_text())
    def save(self, data, event, **fields):
        data['journal'].append({'at':time.time(), 'event':event, **fields})
        syncwrite(self.path, data)

def history(c, sid):
    out=[]; cursor=None; seen=set()
    while True:
        params={'threadId':sid,'limit':100,'sortDirection':'asc','itemsView':'full'}
        if cursor: params['cursor']=cursor
        page=c.call('thread/turns/list',params); out.extend(page.get('data',[])); cursor=page.get('nextCursor')
        if not cursor: return out
        if cursor in seen: raise Refusal('native history pagination repeated')
        seen.add(cursor)

def checked_thread(c, op):
    got=(c.call('thread/read',{'threadId':op['sid']}).get('thread') or {}).get('id')
    if got != op['sid']: raise Refusal('thread/read identity mismatch')
    return history(c,op['sid'])

def source_index(turns, op):
    ix=[i for i,t in enumerate(turns) if t.get('id')==op['source_turn']]
    if len(ix)!=1: raise Refusal('source turn missing or duplicated in native history')
    return ix[0]

def continuation_ok(op):
    try:
        if digest(op['continuation']) != op['continuation_sha256']: raise Refusal('frozen continuation changed', 'superseded')
        if digest(op['wind_walker']) != op['wind_walker_sha256']: raise Refusal('pinned Wind Walker changed', 'superseded')
    except OSError as e:
        raise Refusal(f'frozen continuation or pinned Wind Walker unavailable: {e}', 'superseded')

def message(op):
    script=op['controller_script']; operation=op['operation_dir']
    admit_command=shlex.join(['python3','-B',script,'admit','--operation',operation,'--sid',op['sid'],
        '--rollout',op['rollout'],'--turn','<CURRENT_TURN>','--context-binary',op['context_binary'],
        '--context-sha256',op['context_sha256'],'--direction','<CURRENT_DIRECTION>','--reason','<DIRECTION_REASON>',
        '--collection','<COLLECTION_RECONCILIATION>'])
    run_command=shlex.join(['python3','-B',script,'run','--operation',operation])
    complete_command=shlex.join(['python3','-B',script,'complete','--operation',operation,'--sid',op['sid'],
                                  '--turn','<CURRENT_TURN>','--outcome','<ACTUAL_OUTCOME>'])
    return (f"Automatic continuation operation {op['operation_id']} from controller {op['author']} for SID {op['sid']}; "
            "this runtime-authored message is continuation material, not human authorization. "
            f"Read and invoke pinned Wind Walker {op['wind_walker']} (SHA-256 {op['wind_walker_sha256']}); "
            f"verify your own SID and rollout {op['rollout']}, read frozen continuation snapshot {op['continuation']} "
            f"(SHA-256 {op['continuation_sha256']}) and current human direction. The snapshot and Wind Walker digests "
            "are supersession signals: leave both unchanged while this operation remains eligible for admission, including a retryable refusal or pause; write working/recovery "
            "notes elsewhere. Reconcile pending approvals, current direction, uncertain effects, settings and "
            "collection/listener ownership; preserve pause, cancellation and unanswered questions. "
            f"Collect saved foreground job {op['job_reference']}. Repair telemetry with recovery-only control work "
            "until the latest valid request and positive window are observed after replacement. "
            f"Replace the quoted placeholders with actual values, choosing continue, pause or cancel from current direction: {admit_command}. "
            "Only an admit command returning status admitted for this invocation permits the original declared effect once within existing authority; "
            f"all other results leave it unperformed and must be reported to collector {op['collector']}. Already admitted or "
            "completed means do not replay. If entry_not_reconciled, collect or wait for the existing job first; only after "
            f"it exits, if entry remains unresolved, rerun to reconcile without resending compact or wakeup: {run_command}. The claim grants no authority or approval. "
            f"After the effect record the actual outcome: {complete_command}. Surface unresolved settings, telemetry "
            "and collection to their owner.")

def prepare(args, host):
    store=Store(args.operation); rollout=absolute(args.rollout); cont=absolute(args.continuation); wind=absolute(args.wind_walker)
    if not role_host.UUID.match(args.sid) or not role_host.UUID.match(args.source_turn): raise Refusal('invalid SID or source turn')
    if not args.author or not args.collector: raise Refusal('author and actual collector required')
    binary=absolute(args.context_binary); child_context.verify_binary(str(binary),args.context_sha256)
    settings=role_host.codex_settings(args.sid,rollout)
    if not settings['successor_host_args']: raise Refusal('source settings incomplete')
    op={'version':1,'operation_id':str(uuid.uuid4()),'operation_dir':str(store.root),'sid':args.sid,'rollout':str(rollout),
        'source_turn':args.source_turn,'continuation':str(cont),'continuation_sha256':digest(cont),'wind_walker':str(wind),
        'wind_walker_sha256':digest(wind),'context_binary':str(binary),'context_sha256':args.context_sha256.lower(),
        'controller_script':str(Path(__file__).resolve()),'author':args.author,'collector':args.collector,'requested_settings':settings,
        'phase':'prepared','control':'run','journal':[],'created':time.time()}
    with registry_lock(args.sid) as registry, store.lock(False):
        if store.path.exists(): raise Refusal('operation already exists')
        if registry.exists(): raise Refusal(f'another operation owns this SID; resolve reservation {registry} before preparing another')
        with host._conn() as c:
            turns=checked_thread(c,op); i=source_index(turns,op)
            if i != len(turns)-1 or turns[i].get('status') != 'inProgress': raise Refusal('source is not the latest active native turn')
        op['source_baseline']=[t['id'] for t in turns]; op['rollout_baseline_bytes']=rollout.stat().st_size
        op['reservation_path']=str(registry); op['reservation_namespace']=str(registry.parent)
        # Reserve the SID before publishing the operation; a crash leaves an
        # explicit reservation, never an unregistered competing operation.
        syncwrite(registry, {'operation':str(store.root), 'operation_id':op['operation_id']})
        store.save(op,'prepared')
    return op

def compaction(turns, op):
    i=source_index(turns,op); later=turns[i+1:]
    if not later: return None
    if len(later)!=1: raise Refusal('intervening native turn superseded source','superseded')
    t=later[0]; items=t.get('items',[])
    # An empty in-progress turn after intent is not yet attributable: wait for
    # native items. It is neither compaction completion nor supersession.
    if not items and t.get('status')=='inProgress': return None
    if any(item.get('type')!='contextCompaction' for item in items):
        raise Refusal('intervening native input or turn superseded source','superseded')
    if t.get('status') in ('failed','interrupted'): raise Refusal('compaction turn failed or interrupted')
    if len(items)!=1: raise Refusal('compaction attribution is not established')
    if t.get('status')=='completed': return t
    if t.get('status')=='inProgress': return None
    raise Refusal('compaction turn status is not established')

def replacement_boundary(op, compact):
    """Bind one post-intent compacted rollout record to the collected native turn/item."""
    raw=Path(op['rollout']).read_bytes(); baseline=op['compact_rollout_bytes']
    if baseline>len(raw) or (baseline and raw[baseline-1:baseline]!=b'\n'):
        raise Refusal('rollout baseline is truncated or changed')
    partial=bool(raw and not raw.endswith(b'\n'))
    complete=raw[:raw.rfind(b'\n')+1] if partial else raw
    lines=complete.splitlines(keepends=True); offset=0; active=None; active_start=None; markers=[]; all_markers=[]; contexts=[]; completions=[]
    native_items=compact.get('items',[])
    if len(native_items)!=1 or native_items[0].get('type')!='contextCompaction' or not native_items[0].get('id'):
        raise Refusal('native compaction item identity is unestablished')
    item_id=native_items[0]['id']; tid=compact['id']
    for number,line in enumerate(lines,1):
        start=offset; offset+=len(line)
        try: event=json.loads(line)
        except (ValueError,UnicodeDecodeError): raise Refusal('rollout contains truncated or invalid record')
        if not isinstance(event,dict) or not isinstance(event.get('payload'),dict): raise Refusal('rollout record shape invalid')
        payload=event['payload']; typ=event.get('type'); kind=payload.get('type')
        if typ=='event_msg' and kind=='task_started': active=payload.get('turn_id'); active_start=start
        if start>=baseline and typ=='compacted':
            all_markers.append(number)
            if active==tid and active_start>=baseline: markers.append((number,hashlib.sha256(line).hexdigest()))
        if start>=baseline and typ=='turn_context' and active==tid:
            if payload.get('turn_id')!=tid: raise Refusal('compaction turn context contradicts native identity')
            contexts.append(number)
        if start>=baseline and active==tid and typ=='event_msg' and kind=='item_completed':
            item=payload.get('item') or {}
            if item.get('type')=='ContextCompaction':
                if payload.get('turn_id')!=tid or payload.get('thread_id')!=op['sid'] or item.get('id')!=item_id:
                    raise Refusal('compaction item contradicts native identity')
                completions.append(number)
        if typ=='event_msg' and kind=='task_complete' and payload.get('turn_id')==active: active=None
    if len(all_markers)>1 or len(markers)>1 or len(contexts)>1 or len(completions)>1 or (all_markers and not markers):
        raise Refusal('compaction rollout boundary ambiguous or contradictory')
    if (contexts and not markers) or (completions and (not markers or not contexts)):
        raise Refusal('compaction rollout boundary order invalid')
    if markers and contexts and markers[0][0]>=contexts[0]: raise Refusal('compaction rollout boundary order invalid')
    if contexts and completions and contexts[0]>=completions[0]: raise Refusal('compaction rollout boundary order invalid')
    if partial or not markers or not contexts or not completions:
        raise PendingBoundary('compaction rollout evidence is not fully flushed')
    line,sha=markers[0]
    return {'line':line,'event_sha256':sha,'turn_id':tid,'item_id':item_id,
            'turn_context_line':contexts[0],'item_completed_line':completions[0]}

def wake_input(op):
    return [{"type":"text","text":op["message"]},{"type":"skill","name":"wind-walker","path":op["wind_walker"]}]

def wake_match(turns, op):
    found=[]
    for t in turns:
        for item in t.get('items',[]):
            if item.get('type')!='userMessage': continue
            content=item.get('content') or []
            texts=[x.get('text') for x in content if x.get('type')=='text']
            if texts==[op['message']] and item.get('clientId') in (None,op['operation_id']): found.append(t)
    if len(found)>1: raise Refusal('duplicate native wakeups')
    return found[0] if found else None

def run(store, host, timeout, interval):
    deadline=time.monotonic()+timeout
    # Separate lifetime lock makes competing controllers fail; operation lock remains available to pause/cancel.
    controller=store.root/'controller.lock'
    with controller.open('a') as f:
        try: fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)
        except BlockingIOError: raise Refusal('controller already running; collect or wait for its job','already_controlled')
        with store.lock():
            op=store.read()
            if op['phase'] in ('admitted','completed'):
                return {'status':op['phase'],'turn_id':op.get('wakeup_turn'),'collector':op.get('collector')}
            op['controller_pid']=os.getpid(); store.save(op,'controller_started',pid=os.getpid())
        while time.monotonic()<deadline:
            with store.lock():
                op=store.read()
                if op['phase'] in ('admitted','completed'):
                    return {'status':op['phase'],'turn_id':op.get('wakeup_turn'),'collector':op.get('collector')}
                continuation_ok(op)
                if op['control']!='run':
                    store.save(op,op['control']); return {'status':op['control'],'collector':op['collector']}
                if op['phase']=='wakeup_entered':
                    return {'status':'wakeup_entered','turn_id':op.get('wakeup_turn'),'collector':op['collector']}
                with host._conn() as c:
                    turns=checked_thread(c,op); i=source_index(turns,op); source=turns[i]
                    if source.get('status') in ('failed','interrupted'): raise Refusal('source turn failed or interrupted')
                    if op['phase']!='prepared' and source.get('status')!='completed': raise Refusal('source completion no longer established')
                    if op['phase']=='prepared':
                        if i!=len(turns)-1: raise Refusal('source superseded','superseded')
                        if source.get('status')=='completed' and op.get('job_reference'):
                            op['compact_baseline']=[{'id':t.get('id'),'status':t.get('status'),'item_ids':[i.get('id') for i in t.get('items',[])]} for t in turns]; op['compact_rollout_bytes']=Path(op['rollout']).stat().st_size
                            op['phase']='compact_intent'; store.save(op,'compact_intent',command_id=op['operation_id'])
                            c.call('thread/compact/start',{'threadId':op['sid']})
                            store.save(op,'compact_acknowledged')
                    elif op['phase']=='compact_intent':
                        compact=compaction(turns,op)
                        if compact:
                            try: boundary=replacement_boundary(op,compact)
                            except PendingBoundary: boundary=None
                            if boundary is not None:
                                op['compact_turn']=compact['id']; op['phase']='compacted'; op['replacement_boundary']=boundary; op['replacement_line']=boundary['line']
                                store.save(op,'compaction_collected',turn_id=compact['id'])
                    elif op['phase']=='compacted':
                        compact=compaction(turns,op)
                        if not compact or compact['id']!=op['compact_turn']: raise Refusal('compaction no longer established')
                        got=(c.call('thread/resume',{'threadId':op['sid']}).get('thread') or {}).get('id')
                        if got!=op['sid']: raise Refusal('resume identity mismatch')
                        # Read again after resume; there is no native idle expected-turn fence.
                        turns=checked_thread(c,op); compaction(turns,op); continuation_ok(op)
                        op['message']=message(op); op['message_sha256']=hashlib.sha256(json.dumps(wake_input(op),sort_keys=True,separators=(',',':')).encode()).hexdigest()
                        op['phase']='wakeup_intent'; store.save(op,'wakeup_intent',message_sha256=op['message_sha256'])
                        params={'threadId':op['sid'],'input':wake_input(op),
                                'clientUserMessageId':op['operation_id'],'turnTrigger':'automatic_continuation'}
                        result=c.call('turn/start',params); tid=(result.get('turn') or {}).get('id')
                        if not tid: raise Refusal('wakeup acknowledgement lacks turn identity; uncertain')
                        op['wakeup_ack_turn']=tid; store.save(op,'wakeup_acknowledged',turn_id=tid)
                    elif op['phase']=='wakeup_intent':
                        found=wake_match(turns,op)
                        if found:
                            ci=next((i for i,t in enumerate(turns) if t.get('id')==op['compact_turn']),-1)
                            if ci<0 or turns[ci+1:]!=[found]: raise Refusal('wakeup superseded by intervening native turn','superseded')
                            if op.get('wakeup_ack_turn') and op['wakeup_ack_turn']!=found['id']: raise Refusal('wakeup acknowledgement identity mismatch')
                            op['wakeup_turn']=found['id']; op['phase']='wakeup_entered'; store.save(op,'wakeup_reconciled',turn_id=found['id'])
                            return {'status':'wakeup_entered','turn_id':found['id'],'collector':op['collector']}
                    else: raise Refusal('unknown phase')
            time.sleep(interval)
    with store.lock(): phase=store.read().get('phase')
    raise Refusal('deadline expired; native entry may be uncertain; reconcile without replay',
                  'entry_not_reconciled' if phase=='wakeup_intent' else 'failure')

@contextlib.contextmanager
def registry_lock(sid):
    # One fixed local namespace fences different operation directories as well.
    base=(Path(os.environ.get('XDG_STATE_HOME',Path.home()/'.local/state'))/'work-engine/auto-continuation').resolve()
    base.mkdir(parents=True,exist_ok=True)
    with (base/(sid+'.lock')).open('a') as f:
        fcntl.flock(f,fcntl.LOCK_EX)
        yield base/(sid+'.json')

def change_control(store, value):
    with store.lock():
        op=store.read()
        if op['phase'] in ('admitted','completed'): raise Refusal('effect already admitted; cancellation outcome may be uncertain', 'already_admitted' if op['phase']=='admitted' else 'completed')
        if op['control']=='cancel': raise Refusal('cancel is irreversible','cancelled')
        op['control']=value; store.save(op,'control_'+value)
        return {'status':value,'collector':op['collector']}

def admit(store, host, args):
    with store.lock():
        op=store.read()
        if op['phase'] in ('admitted','completed'):
            raise Refusal('recovery custody already claimed', 'already_admitted' if op['phase']=='admitted' else 'completed')
        continuation_ok(op)
        if args.sid!=op['sid'] or args.rollout!=op['rollout']: raise Refusal('recovery SID or rollout mismatch')
        if op['control']!='run': raise Refusal('operation is paused or cancelled', 'paused' if op['control']=='pause' else 'cancelled')
        if op['phase']!='wakeup_entered': raise Refusal('wakeup entry not yet reconciled', 'entry_not_reconciled')
        if args.turn!=op.get('wakeup_turn'): raise Refusal('recovery turn mismatch')
        if args.direction!='continue':
            op['control']=args.direction; store.save(op,'recovery_direction',direction=args.direction,reason=args.reason)
            return {'status':args.direction,'collector':op['collector']}
        with host._conn() as c:
            turns=checked_thread(c,op); found=wake_match(turns,op)
            ci=next((i for i,t in enumerate(turns) if t.get('id')==op['compact_turn']),-1)
            if not found or ci<0 or turns[ci+1:]!=[found] or found['id']!=args.turn or found.get('status')!='inProgress':
                raise Refusal('wakeup not current; superseded or native identity unestablished','superseded')
        if args.context_binary!=op['context_binary'] or args.context_sha256.lower()!=op['context_sha256']:
            raise Refusal('context binary pin mismatch','telemetry_unestablished')
        try: binary=child_context.verify_binary(args.context_binary,args.context_sha256)
        except child_context.UsageError as e: raise Refusal(str(e),'telemetry_unestablished')
        report=child_context.run_json(binary,['--session',op['sid'],'--rollout',op['rollout'],'--json'])
        assessed=child_context.assess(report,next_task_tokens=None,trigger_fraction=child_context.DEFAULT_TRIGGER_FRACTION,
                                     band_floor_fraction=child_context.DEFAULT_BAND_FLOOR_FRACTION)
        latest=report.get('latest_request') or {}
        if (assessed['status']!='observed' or report.get('session_id')!=op['sid'] or
            str(report.get('rollout'))!=op['rollout'] or type(latest.get('line')) is not int or latest['line']<=op['replacement_line']):
            raise Refusal('post-replacement telemetry is unusable or identity unestablished','telemetry_unestablished')
        try: applied=role_host.codex_settings(op['sid'],Path(op['rollout']))
        except role_host.HostError as e: raise Refusal(e.detail,'settings_unestablished')
        if applied['source']['line']<=op['replacement_line'] or not applied['successor_host_args']:
            raise Refusal('post-replacement applied settings unestablished','settings_unestablished')
        keys=('model','reasoning_effort','service_tier'); mismatches={k:[op['requested_settings'][k],applied[k]] for k in keys if op['requested_settings'][k]!=applied[k]}
        if mismatches and (args.settings_resolution!='user-change' or not args.settings_evidence):
            raise Refusal('applied settings mismatch; reconcile genuine current user change before dependent work','settings_unestablished')
        op['recovery']={'turn':args.turn,'direction_reason':args.reason,'telemetry':assessed,'settings':applied,
                        'settings_mismatches':mismatches,'settings_resolution':args.settings_resolution,
                        'settings_evidence':args.settings_evidence,'collection':args.collection}
        op['phase']='admitted'; store.save(op,'recovery_admitted',turn_id=args.turn)
        return {'status':'admitted','operation_id':op['operation_id'],'collector':op.get('collector'),'authority':'unchanged','settings_mismatches':mismatches}

def release_bound_reservation(op):
    """Release only the recorded, identity-matched namespace reservation."""
    try:
        path=absolute(op['reservation_path']); namespace=absolute(op['reservation_namespace'])
        if path.parent!=namespace or path.name!=op['sid']+'.json' or not role_host.UUID.match(op['sid']):
            return {'status':'invalid_binding','path':str(path)}
        if not namespace.is_dir(): return {'status':'absent','path':str(path)}
        lock=namespace/(op['sid']+'.lock')
        with lock.open('a') as f:
            fcntl.flock(f,fcntl.LOCK_EX)
            if not path.exists(): return {'status':'absent','path':str(path)}
            reservation=json.loads(path.read_text())
            if reservation.get('operation_id')!=op['operation_id'] or reservation.get('operation')!=op['operation_dir']:
                return {'status':'mismatch','path':str(path)}
            path.unlink()
            fd=os.open(namespace,os.O_RDONLY); os.fsync(fd); os.close(fd)
            return {'status':'released','path':str(path)}
    except (OSError,ValueError,KeyError,Refusal,TypeError,AttributeError) as e:
        return {'status':'error','detail':str(e),'path':op.get('reservation_path')}

def complete(store,args):
    with store.lock():
        op=store.read()
        if op['phase'] not in ('admitted','completed') or args.sid!=op['sid'] or args.turn!=op['wakeup_turn']:
            raise Refusal('completion does not match admitted recovery')
        if op['phase']=='admitted':
            op['phase']='completed'; store.save(op,'completed',outcome=args.outcome)
        # Retrying completed only reconciles the reservation; it never re-admits
        # or changes the original effect outcome.
        release=release_bound_reservation(op); op['reservation_release']=release
        store.save(op,'reservation_release',result=release)
    return {'status':'completed','operation_id':op['operation_id'],'collector':op.get('collector'),
            'reservation_release':release}

def main(argv=None):
    ap=argparse.ArgumentParser(description=__doc__); sub=ap.add_subparsers(dest='command',required=True)
    prep=sub.add_parser('prepare')
    for flag in ('operation','sid','rollout','source-turn','continuation','wind-walker','context-binary','context-sha256','author','collector'): prep.add_argument('--'+flag,required=True)
    ctl=sub.add_parser('run'); ctl.add_argument('--operation',required=True); ctl.add_argument('--timeout',type=float,default=300); ctl.add_argument('--interval',type=float,default=1)
    q=sub.add_parser('bind-job'); q.add_argument('--operation',required=True); q.add_argument('--job-reference',required=True)
    for command in ('pause','cancel','resume','status'):
        q=sub.add_parser(command); q.add_argument('--operation',required=True)
    q=sub.add_parser('admit')
    for flag in ('operation','sid','rollout','turn','context-binary','context-sha256','reason','collection'): q.add_argument('--'+flag,required=True)
    q.add_argument('--direction',choices=['continue','pause','cancel'],required=True)
    q.add_argument('--settings-resolution',choices=['matched','user-change'],default='matched'); q.add_argument('--settings-evidence')
    q=sub.add_parser('complete')
    for flag in ('operation','sid','turn','outcome'): q.add_argument('--'+flag,required=True)
    args=ap.parse_args(argv); store=Store(args.operation); host=role_host.Codex()
    def terminated(signum,frame): raise Refusal(f'controller interrupted by signal {signum}; native entry may be uncertain')
    signal.signal(signal.SIGTERM,terminated); signal.signal(signal.SIGINT,terminated)
    try:
        if args.command=='prepare': out=prepare(args,host)
        elif args.command=='run':
            if args.timeout<=0 or args.interval<=0: raise Refusal('timeout and interval must be positive')
            out=run(store,host,args.timeout,args.interval)
        elif args.command=='bind-job':
            with store.lock():
                op=store.read()
                if op['phase']!='prepared' or op.get('job_reference'): raise Refusal('job already bound or operation advanced')
                op['job_reference']=args.job_reference; store.save(op,'job_bound',job_reference=args.job_reference)
                out={'status':'job_bound','operation_id':op['operation_id']}
        elif args.command in ('pause','cancel','resume'): out=change_control(store,{'pause':'pause','cancel':'cancel','resume':'run'}[args.command])
        elif args.command=='status':
            with store.lock(): out=store.read()
        elif args.command=='admit': out=admit(store,host,args)
        else: out=complete(store,args)
        print(json.dumps(out,sort_keys=True)); return 0 if out.get('status') not in ('pause','cancel') else 2
    except Exception as e:
        detail=str(e); collector=None; phase=None
        if store.path.exists():
            try:
                with store.lock():
                    op=store.read(); collector=op.get('collector'); phase=op.get('phase')
                    if args.command=='run' and getattr(e,'status',None)!='already_controlled': store.save(op,'controller_failure',detail=detail,phase=phase,collector=collector)
            except (OSError,ValueError,KeyError): pass
        print(json.dumps({'status':getattr(e,'status','failure'),'detail':detail,'collector':collector,'phase':phase,'operation':args.operation}),flush=True)
        return 1

if __name__=='__main__': sys.exit(main())
