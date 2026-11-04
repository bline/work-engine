"""Opt-in actual MCP/HTTP -> signed Rust admission -> actual Pi/Docker fixture.
Identity, signing key and provider are synthetic; protocol/runtime/tools are real.
Run through custody with PI_CODING_* executed binary pins set by the build plan.
"""
import base64
import hashlib
import http.client
import http.server
import json
import os
from pathlib import Path
import sqlite3
import shutil
import subprocess
import threading
import time
import unittest
from test_session_events import SE

REPO = Path(__file__).resolve().parents[3]
PROTOCOL = 'work-engine.mcp-pi-coding-fixture.v1'
RESOURCE = 'https://mcp-fixture.example/mcp'
ISSUER = 'https://fixture-issuer.example'

def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def write(p, value):
    p.write_text(json.dumps(value, separators=(',', ':'), ensure_ascii=False))
    p.chmod(0o600)
    return dict(path=str(p), sha256=sha(p))

def artifact(p):
    return dict(path=str(p), sha256=sha(p))

def b64(b):
    return base64.urlsafe_b64encode(b).rstrip(b'=').decode()

@unittest.skipUnless(os.environ.get('PI_CODING_ACTUAL') == '1', 'explicit actual fixture admission required')
class ActualCoding(unittest.TestCase):
    def test_authenticated_mcp_signed_pi_fake_provider_and_pure_reads(self):
        root = Path(os.environ['PI_CODING_EVIDENCE']).resolve()
        root.mkdir(mode=0o700)
        helper = Path(os.environ['PI_CODING_HELPER'])
        generator = Path(os.environ['PI_CODING_GENERATOR'])
        verifier = Path(os.environ['MCP_AUTH_TEST_BINARY'])
        for path, key in [(helper, 'PI_CODING_HELPER_SHA'), (generator, 'PI_CODING_GENERATOR_SHA'), (verifier, 'MCP_AUTH_TEST_SHA')]:
            self.assertEqual(sha(path), os.environ[key])
        # Stage owned executable bytes with private modes; preserve build output.
        staged = root/'mcp-pi-coding'
        shutil.copyfile(helper, staged); staged.chmod(0o700)
        self.assertEqual(sha(staged), os.environ['PI_CODING_HELPER_SHA'])
        write(root/'executed-helper-pin.json', dict(path=str(staged),sha256=sha(staged),build_output=str(helper)))
        helper = staged
        state = root/'native'; state.mkdir(mode=0o700)
        workspace = root/'workspace'; (workspace/'src').mkdir(parents=True)
        before = 'pub fn parse_delay_ms(n:u64)->u64 { n*1000 }\n'
        (workspace/'src/lib.rs').write_text(before)
        acceptance = root/'acceptance.rs'
        acceptance.write_text('#[path="/workspace/src/lib.rs"] mod parser;\n#[test]fn minute(){assert_eq!(parser::parse_delay_ms(2),120000);}\n#[test]fn zero(){assert_eq!(parser::parse_delay_ms(0),0);}\n#[test]fn one(){assert_eq!(parser::parse_delay_ms(1),60000);}\n')
        runtime_root = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi')
        toolchain_root = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/real-job-20261104/readiness/implementation-evidence/offline-rustc-1.92.0')
        prior = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/real-job-20261104/readiness/implementation-evidence/actual-6')
        # Copy only exact manifests into owned evidence; never copy source/evidence trees.
        for name, expected in [('runtime.json', 'cf9d096e61d8ffd2c35df238ff74e1f8d7dacfb9bc5813333beef6cc73010985'), ('toolchain.json', '90d4c2315715eddc41d2711e3c4d1612e9b7b5fb61541466bcf8066b4c8d8da2')]:
            self.assertEqual(sha(prior/name), expected)
            (root/name).write_bytes((prior/name).read_bytes())
        command = """set -eu
if touch /workspace/forbidden; then exit 31; fi
if touch /workspace/src/forbidden; then exit 32; fi
if printf bad > /acceptance.rs; then exit 33; fi
printf replacement > /tmp/replacement
if mv /tmp/replacement /workspace/src/lib.rs; then exit 34; fi
if ln /workspace/src/lib.rs /workspace/src/alias; then exit 35; fi
rustc --edition=2024 --test /acceptance.rs -o /tmp/before
if /tmp/before; then exit 18; else test "$?" -eq 101; fi
printf 'pub fn parse_delay_ms(n:u64)->u64 { n*60000 }\\n' > /workspace/src/lib.rs
printf 'SIGNED_FAKE_PROVIDER_TOOL_DONE\\n'
"""
        job = dict(schema='work-engine.pi-docker-custody.v1',job_id='signed-pi-fixture',state_dir=str(state),
            authority=write(root/'authority.json',dict(classification='fake signing and identity fixture only')),
            packet=write(root/'packet.json',dict(classification='deterministic fake provider; network none; actual SDK and Docker')),
            docker=artifact('/usr/bin/docker'),image='node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da',
            runtime=dict(root=str(runtime_root),manifest=artifact(root/'runtime.json')),
            workspace=dict(root=str(workspace),manifest=write(root/'workspace.json',{'src/lib.rs':dict(kind='file',sha256=sha(workspace/'src/lib.rs'))})),
            deadline_unix_seconds=int(time.time())+240,command_timeout_seconds=30,idle_seconds=0,
            coding=dict(schema='work-engine.pi-coding-signed-fixture.v1',shim=artifact(REPO/'rust/crates/work-engine-manual-host/receiver/pi-coding.mjs'),
                toolchain=dict(root=str(toolchain_root),manifest=artifact(root/'toolchain.json')),acceptance=artifact(acceptance),fixture_command=command,max_tool_calls=1,workspace_write_boundary='src_lib_rs'))
        job_pin = write(root/'job.json', job)
        bundle = root/'signed-authority'
        env = dict(os.environ, PI_FIXTURE_EXPORT=str(bundle), PI_FIXTURE_JOB_SHA=job_pin['sha256'], PI_FIXTURE_EXECUTOR_SHA=sha(helper))
        generated = subprocess.run([str(generator), '--ignored', '--exact', 'pi_pilot::tests::export_signed_pi_fixture', '--nocapture'],env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=30)
        (root/'generator.log').write_bytes(generated.stdout)
        self.assertEqual(generated.returncode, 0, generated.stdout.decode())
        descriptor = json.loads((bundle/'descriptor-pin.json').read_text())
        config = dict(schema=PROTOCOL,descriptor=descriptor,job=job_pin,principal='fixture-principal',subject='fixture-subject',handle='one-signed-pi-fixture',revision='r1',idempotency_key='one-exact-job')
        config_path = root/'coding-config.json'; write(config_path, config)
        helper_obj = SE.CodingHelper(str(helper),sha(helper),str(config_path),sha(config_path))
        homes = SE.Homes(root/'synthetic-codex',root/'synthetic-claude',root/'events')
        server = SE.Server(homes,'fixture-principal',SE.CallbackPolicy(),coding=helper_obj)
        key = root/'fake-jwt-key.pem'
        subprocess.run(['openssl','genpkey','-quiet','-algorithm','RSA','-pkeyopt','rsa_keygen_bits:2048','-out',str(key)],check=True,stderr=subprocess.DEVNULL)
        modulus = subprocess.check_output(['openssl','rsa','-in',str(key),'-noout','-modulus'],stderr=subprocess.DEVNULL).decode().strip().split('=')[1]
        write(root/'jwks.json',dict(keys=[dict(kty='RSA',kid='fixture',use='sig',alg='RS256',n=b64(bytes.fromhex(modulus)),e='AQAB')]))
        auth_config = root/'http-auth.json'
        auth_value = dict(resource=RESOURCE,issuer=ISSUER,subject='fixture-subject',write_scope='mcp:write',jwks_snapshot=str(root/'jwks.json'),jwks_max_age_seconds=300,verifier=str(verifier))
        write(auth_config,auth_value)
        auth = SE.HttpAuth(str(auth_config))
        handler = SE.make_http_handler(server,'/mcp',auth,coding_only=True)
        with self.assertRaises(ValueError):
            SE.make_http_handler(server,'/mcp',None,coding_only=True)
        wrong = dict(auth_value,subject='wrong-subject');write(root/'wrong-http-auth.json',wrong)
        with self.assertRaises(ValueError):
            SE.make_http_handler(server,'/mcp',SE.HttpAuth(str(root/'wrong-http-auth.json')),coding_only=True)
        original = server.principal;server.principal='foreign'
        with self.assertRaises(ValueError):
            SE.make_http_handler(server,'/mcp',auth,coding_only=True)
        server.principal=original
        httpd = http.server.ThreadingHTTPServer(('127.0.0.1',0),handler)
        thread = threading.Thread(target=httpd.serve_forever);thread.start()
        transcript=[]
        def token(**overrides):
            claims=dict(iss=ISSUER,aud=RESOURCE,sub='fixture-subject',iat=int(time.time()),exp=int(time.time())+240,client_id='synthetic',jti='fixture',scope='openid mcp:write');claims.update(overrides)
            message=(b64(json.dumps(dict(alg='RS256',typ='at+jwt',kid='fixture')).encode())+'.'+b64(json.dumps(claims).encode())).encode()
            sig=subprocess.check_output(['openssl','dgst','-sha256','-sign',str(key)],input=message)
            return message.decode()+'.'+b64(sig)
        def rpc(method,params=None,access=None):
            c=http.client.HTTPConnection('127.0.0.1',httpd.server_address[1],timeout=280)
            msg=dict(jsonrpc='2.0',id=len(transcript)+1,method=method)
            if params is not None:msg['params']=params
            headers={'Content-Type':'application/json'}
            if access is not None:headers['Authorization']='Bearer '+access
            c.request('POST','/mcp',body=json.dumps(msg),headers=headers);r=c.getresponse();raw=r.read();status=r.status;c.close()
            value=json.loads(raw) if raw else None;transcript.append(dict(request=msg,http_status=status,response=value));return status,value
        args={k:config[k] for k in ('handle','revision','idempotency_key')}
        def tool(method,arguments=None):
            status,v=rpc('tools/call',dict(name='pi_coding_'+method,arguments=args if arguments is None else arguments),token())
            self.assertEqual(status,200,v);self.assertNotIn('error',v,v);self.assertNotIn('isError',v['result'],v)
            return v['result']['structuredContent']['pi_coding']
        try:
            self.assertEqual(rpc('initialize',dict(protocolVersion=SE.DEFAULT_PROTOCOL))[0],401)
            self.assertEqual(rpc('initialize',dict(protocolVersion=SE.DEFAULT_PROTOCOL),token(aud='wrong-audience'))[0],401)
            self.assertEqual(rpc('initialize',dict(protocolVersion=SE.DEFAULT_PROTOCOL),token(sub='wrong-subject'))[0],401)
            self.assertEqual(rpc('initialize',dict(protocolVersion=SE.DEFAULT_PROTOCOL),token())[0],200)
            status,v=rpc('tools/list',access=token());self.assertEqual(status,200);self.assertEqual({t['name'] for t in v['result']['tools']},{'pi_coding_submit','pi_coding_status','pi_coding_result'})
            self.assertEqual(rpc('tools/call',dict(name='delivery_ack',arguments={}),token())[0],403)
            self.assertEqual(tool('status')['state'],'not_admitted');self.assertFalse((state/'pi-state.json').exists())
            v=tool('submit');write(root/'domain-result.json',v)
            self.assertEqual(v['state'],'runtime_evidence_recorded',v)
            receipt=v['runtime_result']['receipt'];self.assertEqual(receipt['status'],'component_passed',receipt.get('failures'))
            self.assertEqual(receipt['session_id'],v['native_session_id']);self.assertEqual(receipt['provider'],'fake_fixture');self.assertEqual(receipt['model'],'fake-pi-model')
            self.assertEqual(len(receipt['coding']['provider_requests']),2)
            for request in receipt['coding']['provider_requests'].values():
                self.assertEqual(request['raw_terminal']['classification'],'fake_fixture');self.assertEqual(request['settled_microusd'],0)
            self.assertTrue(receipt['removed']);self.assertTrue(receipt['coding']['tool_removed']);self.assertTrue(receipt['coding']['validation_removed']);self.assertTrue(receipt['coding']['access_removed'])
            self.assertTrue(all(p['status'] is not None and not p['leftovers'] for p in receipt['processes']))
            stable = {p:sha(p) for p in [state/'pi-state.json',bundle/'registry.sqlite']}
            for method in ['status','result','submit','result']:
                self.assertEqual(tool(method),v)
                self.assertEqual({p:sha(p) for p in stable},stable)
            bad=dict(args,idempotency_key='changed-binding')
            status,conflict=rpc('tools/call',dict(name='pi_coding_submit',arguments=bad),token())
            self.assertTrue('error' in conflict or conflict.get('result',{}).get('isError'),conflict)
            self.assertEqual({p:sha(p) for p in stable},stable)
            helper_pin=sha(config_path);config_path.write_text(config_path.read_text()+' ')
            status,drift=rpc('tools/call',dict(name='pi_coding_submit',arguments=args),token())
            self.assertTrue('error' in drift or drift.get('result',{}).get('isError'),drift)
            self.assertNotEqual(sha(config_path),helper_pin)
        finally:
            write(root/'mcp-transcript.json',transcript)
            httpd.shutdown();httpd.server_close();thread.join(timeout=5)
            self.assertFalse(thread.is_alive())
