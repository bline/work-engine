import hashlib,json,os,shutil
from pathlib import Path
R=Path(__file__).parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
m=json.loads((R/'obligation-map.json').read_text());s=json.loads((R/'source-manifest.json').read_text())
for e in s['entries']:
 p=Path(e['path']);assert sha(p)==e['sha256'];d=R/'source-closure'/str(p).lstrip('/');d.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,d);assert sha(d)==e['sha256']
root=R/'dependency-object-closure-01';(root/'objects').mkdir(parents=True);entries=[]
for name,pin in m['external_inputs'].items():
 p=Path(name);rel='objects/'+hashlib.sha256(os.fsencode(name)).hexdigest();d=root/rel
 if pin['kind']=='symlink':
  assert p.is_symlink() and os.readlink(p)==pin['link_target'];os.symlink(pin['link_target'],d);h=hashlib.sha256(os.fsencode(os.readlink(d))).hexdigest()
 else:
  assert p.is_file() and not p.is_symlink();shutil.copyfile(p,d);h=sha(d)
 assert h==pin['sha256'];entries.append({'original_path':name,'stored_path':rel,'pin':pin})
(root/'index.json').write_text(json.dumps({'entries':entries},indent=1)+'\n')
print('retained source',len(s['entries']),'external objects',len(entries))
