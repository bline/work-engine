"""Exact owned Present / ExactAbsent / Unknown; facts retain their acting owner."""
import json, pathlib, re
from realization import sha, write, read, require
LABEL='u2.private.owner'
def removal_ok(r,cid):
    return all(r.get(k)==v for k,v in dict(returncode=0,stdout=cid+'\n',stderr='').items())
def decode(r,ref,sid,label=LABEL,labels=None):
    if r.get('returncode')==1 and r.get('stdout')=='[]\n' and r.get('stderr')==f'Error response from daemon: No such container: {ref}\n':return {'kind':'ExactAbsent','reference':ref}
    if r.get('returncode')!=0 or r.get('stderr')!='':return {'kind':'Unknown','reason':'RPC failure'}
    try:
        rows=json.loads(r['stdout']); assert isinstance(rows,list) and len(rows)==1
        v=rows[0]; cid=v['Id']; assert isinstance(cid,str) and re.fullmatch('[0-9a-f]{64}',cid)
        assert (cid==ref if re.fullmatch('[0-9a-f]{64}',ref) else v['Name']=='/'+ref)
        assert v['Config']['Labels'][label]==sid
        assert all(v['Config']['Labels'].get(k)==val for k,val in (labels or {}).items())
        st=v['State']; assert type(st['Running']) is bool and type(st['Pid']) is int and st['Pid']>=0
        assert (st['Running'] and st['Pid']>0) or (not st['Running'] and st['Pid']==0)
        return {'kind':'Present','object':v}
    except (ValueError,KeyError,TypeError,AssertionError):return {'kind':'Unknown','reason':'invalid/mismatched response'}
def inspect(ref,sid,run,label=LABEL,labels=None):
    r=run('container','inspect',ref);return decode(r,ref,sid,label,labels),r
def valid_record(path,out,binding,actor=None):
    p=pathlib.Path(path); require(p.parent==out and p.resolve()==p,'fact path escaped')
    v=read(p); require(v['binding']==binding and v['record_path']==str(p),'fact binding mismatch')
    require(v['actor'] in ['controller','watchdog'] and (actor is None or v['actor']==actor),'fact actor mismatch')
    require(type(v['actor_pid']) is int and v['actor_pid']>0,'fact actor pid'); return v
def fact(path,out,b,cid,kind):
    v=valid_record(path,out,b);require(v['container']==cid,'fact container mismatch')
    require(pathlib.Path(path).name==v['actor']+'-'+kind+'-fact.json','fact actor/path mismatch')
    if kind=='terminal':require(v['state']['Running'] is False and v['state']['Pid']==0,'terminal state')
    else:require(removal_ok(v['response'],cid),'removal response')
    return v
def cleanup(cid,out,prefix,sid,run,context):
    out=pathlib.Path(out); attempts=[]; b=context.b; label=context.s.r['owner_label_key']
    def save(n,v):return context.save(out/(prefix+'-'+n+'.json'),dict(v,container=cid))
    def observe(stage):
        o,r=inspect(cid,sid,run,label,context.labels());attempts.append(dict(stage=stage,observation=o,response=r));return o
    def unresolved(reason):
        save('unresolved',dict(status='container_unknown',reason=reason,attempts=attempts,custody_owner=context.s.r['root_session_id'],closed=False));return False
    o=observe('initial')
    if o['kind']=='Unknown':return unresolved('initial inspection unknown')
    if o['kind']=='Present':
        if o['object']['State']['Running']:
            r=run('kill',cid);attempts.append(dict(stage='kill',response=r));o=observe('after-kill')
            if o['kind']=='Unknown':return unresolved('unknown after kill')
            if o['kind']=='Present' and o['object']['State']['Running']:return unresolved('still running')
        if o['kind']=='Present':
            save('terminal-fact',dict(state=o['object']['State'],observation=o))
            r=run('rm',cid);attempts.append(dict(stage='remove',response=r))
            if removal_ok(r,cid):save('removal-fact',dict(response=r))
    o=observe('final')
    if o['kind']!='ExactAbsent':return unresolved('final absence unestablished')
    absent_saved=save('absent',dict(absent=True,observation=o,attempts=attempts))
    facts={}
    for kind in ['terminal','removal']:
        facts[kind]=[]
        for p in out.glob('*-'+kind+'-fact.json'):
            try:
                v=fact(p,out,b,cid,kind);facts[kind].append(dict(path=str(p),sha256=sha(p),actor=v['actor'],actor_pid=v['actor_pid']))
            except (OSError,ValueError,KeyError,TypeError):continue
    if not absent_saved or not facts['terminal'] or not facts['removal']:return unresolved('exact absence observed; terminal/removal attribution missing')
    ap=out/(prefix+'-absent.json')
    return save('closure',dict(closed=True,terminal_facts=facts['terminal'],removal_facts=facts['removal'],absence=dict(path=str(ap),sha256=sha(ap))))

def confirmation(out,cid,binding):
    try:
        out=pathlib.Path(out); v=valid_record(out/'cleanup-confirmed.json',out,binding,'controller')
        require(v['container']==cid and v['terminal_removed'] is True,'confirmation identity')
        cp=out/'controller-closure.json'; require(v['closure']==dict(path=str(cp),sha256=sha(cp)),'confirmation closure reference')
        r=valid_record(cp,out,binding,'controller');require(r['container']==cid and r['closed'] is True,'closure identity')
        ap=out/'controller-absent.json';require(r['absence']==dict(path=str(ap),sha256=sha(ap)),'absence path/digest')
        a=valid_record(ap,out,binding,'controller'); require(a['container']==cid and a['absent'] is True and a['observation']==dict(kind='ExactAbsent',reference=cid),'absence fact')
        for kind in ['terminal','removal']:
            require(bool(r[kind+'_facts']),'empty fact list')
            for f in r[kind+'_facts']:
                require(sha(f['path'])==f['sha256'],'fact digest');fv=fact(f['path'],out,binding,cid,kind)
                require(fv['actor']==f['actor'] and fv['actor_pid']==f['actor_pid'],'actor attribution')
        return True
    except (OSError,ValueError,KeyError,TypeError):return False
