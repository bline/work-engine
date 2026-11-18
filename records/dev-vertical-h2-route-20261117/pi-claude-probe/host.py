#!/usr/bin/env python3
"""Host side of the probe: runs the container, serves its framed fetches over
HTTPS to api.anthropic.com with the real OAuth bearer injected, retains exact
request bodies and rate-limit headers. Never writes the real token.
Usage: host.py <run-dir> <model>"""
import base64, hashlib, http.client, json, os, ssl, subprocess, sys, time

run, model = sys.argv[1], sys.argv[2]
P = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.makedirs(run + "/out", exist_ok=True)
os.makedirs(run + "/requests", exist_ok=True)
real = json.load(open(os.path.expanduser("~/.claude/.credentials.json")))["claudeAiOauth"]["accessToken"]
DUMMY = "sk-ant-oat01-" + "D" * 95
ledger = open(run + "/requests/ledger.jsonl", "a")
cmd = ["docker", "run", "-i", "--rm", "--pull", "never", "--network", "none", "--read-only",
       "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
       *(["--pids-limit", "64", "--memory", "512m", "--cpus", "1", "--tmpfs", "/tmp:rw,noexec,nosuid,size=64m"]
         if os.environ.get("PROBE_STRICT") else
         ["--pids-limit", "256", "--memory", "2g", "--cpus", "2", "--tmpfs", "/tmp:rw,nosuid,size=512m"]),
       "--user", "1000:1000",
       "--mount", "type=bind,src=/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi,dst=/pi,readonly",
       "--mount", f"type=bind,src={os.path.expanduser('~/.pi/agent/npm')},dst=/ext,readonly",
       "--mount", f"type=bind,src={P}/scripts,dst=/probe,readonly",
       "--mount", f"type=bind,src={run}/in,dst=/probe-in,readonly",
       "--mount", f"type=bind,src={P}/fixture,dst=/workspace,readonly",
       "--mount", f"type=bind,src={run}/out,dst=/out",
       "--env", f"PROBE_MODEL={model}",
       "node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da",
       "node", "/probe/forwarder.mjs"]
p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0)
ctx = ssl.create_default_context()
def reply(o):
    p.stdin.write((json.dumps(o) + "\n").encode()); p.stdin.flush()
done = None
for line in p.stdout:
    m = json.loads(line)
    if m["t"] == "done":
        done = m; break
    body = base64.b64decode(m["body_b64"])
    n = m["id"]
    open(f"{run}/requests/{n:04d}.body", "wb").write(body)
    auth = m["headers"].get("authorization", "")
    rec = {"n": n, "method": m["method"], "path": m["path"], "body_sha256": hashlib.sha256(body).hexdigest(),
           "len": len(body), "auth_from_container": "dummy" if auth == "Bearer " + DUMMY else ("absent" if not auth else "OTHER"),
           "header_names": sorted(m["headers"].keys())}
    if m["method"] != "POST" or not m["path"].startswith("/v1/messages") or rec["auth_from_container"] != "dummy":
        rec["refused"] = True
        ledger.write(json.dumps(rec) + "\n"); ledger.flush()
        reply({"t": "head", "id": n, "status": 403, "headers": {"content-type": "application/json"}})
        reply({"t": "chunk", "id": n, "b64": base64.b64encode(b'{"type":"error","error":{"type":"permission_error","message":"probe host refused"}}').decode()})
        reply({"t": "end", "id": n}); continue
    hop = {"host", "connection", "content-length", "accept-encoding", "transfer-encoding", "authorization"}
    headers = {k: v for k, v in m["headers"].items() if k.lower() not in hop}
    headers.update({"Host": "api.anthropic.com", "Authorization": "Bearer " + real, "Accept-Encoding": "identity"})
    conn = http.client.HTTPSConnection("api.anthropic.com", 443, context=ctx, timeout=600)
    conn.request("POST", m["path"], body=body, headers=headers)
    r = conn.getresponse()
    rh = {k: v for k, v in r.getheaders() if k.lower() not in {"transfer-encoding", "connection", "content-length", "set-cookie"}}
    rec["status"] = r.status
    rec["ratelimit"] = {k: v for k, v in r.getheaders() if "ratelimit-unified-status" in k.lower() or "overage" in k.lower()}
    reply({"t": "head", "id": n, "status": r.status, "headers": rh})
    resp = open(f"{run}/requests/{n:04d}.response", "wb")
    while True:
        c = r.read1(65536)
        if not c: break
        resp.write(c)
        reply({"t": "chunk", "id": n, "b64": base64.b64encode(c).decode()})
    resp.close(); conn.close()
    reply({"t": "end", "id": n})
    ledger.write(json.dumps(rec) + "\n"); ledger.flush()
p.wait(timeout=60)
print(json.dumps({"done": done, "exit": p.returncode}))
