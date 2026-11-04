"""Synthetic local HTTP -> Rust verifier tests; no provider credentials or network."""
import base64
import http.client
import http.server
import json
import os
from pathlib import Path
import subprocess
import threading
import time
import types
import unittest
from unittest import mock

from test_session_events import Fixture, SE, CODEX_ID, URL, whsec

VERIFIER = Path(os.environ.get("MCP_AUTH_TEST_BINARY", Path(__file__).resolve().parents[3] / "rust/target/debug/mcp-auth"))
RESOURCE = "https://mcp-test.devwits.ai/mcp"
ISSUER = "https://checkered-replace-0099.customers.stytch.dev"
SUBJECT = "user-test-8b1b7b52-0fa4-4b70-8594-b2b043592a1c"

def b64(data):
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

class AuthHttp(Fixture):
    def setUp(self):
        super().setUp()
        self.key = self.directory / "key.pem"
        subprocess.run(["openssl", "genpkey", "-quiet", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out", str(self.key)], check=True, stderr=subprocess.DEVNULL)
        modulus = subprocess.check_output(["openssl", "rsa", "-in", str(self.key), "-noout", "-modulus"], stderr=subprocess.DEVNULL).decode().strip().split("=")[1]
        self.jwk = {"kty": "RSA", "kid": "one", "use": "sig", "alg": "RS256", "n": b64(bytes.fromhex(modulus)), "e": "AQAB"}
        self.snapshot = self.directory / "jwks.json"
        self.snapshot.write_text(json.dumps({"keys": [self.jwk]}))
        self.config = self.directory / "config.json"
        self.config.write_text(json.dumps({"resource": RESOURCE, "issuer": ISSUER, "subject": SUBJECT, "write_scope": "mcp:write", "jwks_snapshot": str(self.snapshot), "jwks_max_age_seconds": 300, "verifier": str(VERIFIER)}))
        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), SE.make_http_handler(self.server, "/mcp", SE.HttpAuth(str(self.config))))
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()
        self.addCleanup(self.httpd.server_close)
        self.addCleanup(self.httpd.shutdown)
        self.port = self.httpd.server_address[1]

    def token(self, overrides=None, header=None):
        now = int(time.time())
        claims = {"iss": ISSUER, "aud": RESOURCE, "sub": SUBJECT, "iat": now, "client_id": "synthetic-client", "jti": "synthetic-token", "exp": now + 120, "scope": "openid mcp:write"}
        claims.update(overrides or {})
        h = {"alg": "RS256", "typ": "at+jwt", "kid": "one"}
        h.update(header or {})
        message = (b64(json.dumps(h).encode()) + "." + b64(json.dumps(claims).encode())).encode()
        sig = subprocess.check_output(["openssl", "dgst", "-sha256", "-sign", str(self.key)], input=message)
        return message.decode() + "." + b64(sig)

    def request(self, body=None, token=None, path="/mcp", method="POST", headers=None):
        c = http.client.HTTPConnection("127.0.0.1", self.port, timeout=5)
        hs = {"Content-Type": "application/json"}
        if token is not None: hs["Authorization"] = "Bearer " + token
        hs.update(headers or {})
        data = body if isinstance(body, bytes) else json.dumps(body).encode() if body is not None else None
        c.request(method, path, body=data, headers=hs)
        r = c.getresponse(); raw = r.read(); result = (r.status, dict(r.getheaders()), json.loads(raw) if raw else None); c.close()
        return result

    def test_discovery_challenge_and_public_reads(self):
        for path in ("/.well-known/oauth-protected-resource", "/.well-known/oauth-protected-resource/mcp"):
            status, _, body = self.request(path=path, method="GET")
            self.assertEqual((status, body), (200, {"resource": RESOURCE, "authorization_servers": [ISSUER], "scopes_supported": ["mcp:write"]}))
        status, headers, _ = self.request({"jsonrpc":"2.0", "id":1, "method":"events/subscribe"})
        self.assertEqual(status, 401); self.assertIn('resource_metadata="https://mcp-test.devwits.ai/.well-known/oauth-protected-resource/mcp"', headers["WWW-Authenticate"])
        self.server.bridge = types.SimpleNamespace(provisioning=True)
        status, _, listing = self.request({"jsonrpc":"2.0", "id":1, "method":"tools/list"})
        self.assertEqual(status, 200)
        self.assertEqual(len(listing["result"]["tools"]), 13)
        self.assertEqual(self.request({"jsonrpc":"2.0", "method":"notifications/initialized"})[0], 202)
        # These apparent reads reconcile or update bridge custody state.
        for name in ("read_session_operation", "read_session_result", "read_session_target", "list_sessions", "read_session", "test_pattern", "budget_status"):
            self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":name}})[0], 401)
        self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":"ping"}, headers={"Origin":"https://evil.example"})[0], 403)

    def test_auth_before_effects_and_bad_requests(self):
        msg = {"jsonrpc":"2.0", "id":2, "method":"events/subscribe", "params": {"name": SE.SIMPLE_EVENT_NAME, "arguments":{"session_id":CODEX_ID,"pattern":"x"}, "delivery":{"mode":"webhook", "url":URL,"secret":whsec()}}}
        self.assertEqual(self.request(msg)[0], 401)
        self.assertEqual(len(self.server.store.all_subscriptions()), 0)
        status, headers, _ = self.request(msg, self.token({"scope":"openid"}))
        self.assertEqual(status, 403)
        self.assertEqual(headers["WWW-Authenticate"], 'Bearer resource_metadata="https://mcp-test.devwits.ai/.well-known/oauth-protected-resource/mcp", error="insufficient_scope", scope="mcp:write"')
        self.assertEqual(len(self.server.store.all_subscriptions()), 0)
        bearer = self.token()
        status, _, body = self.request(msg, bearer)
        self.assertEqual(status, 200); self.assertIn("result", body)
        self.assertNotIn(bearer, (self.homes.state / "requests.log").read_text())
        self.assertEqual(self.request(b'{"jsonrpc":"2.0","method":"ping","method":"events/subscribe"}')[0], 401)
        self.assertEqual(self.request(b'bad')[0], 401)
        malformed = {"jsonrpc":"2.0", "id":1, "method":[]}
        self.assertEqual(self.request(malformed)[0], 401)
        before = len(self.server.store.all_subscriptions())
        status, _, body = self.request(malformed, self.token())
        self.assertEqual(status, 200)
        self.assertEqual(body["error"]["code"], -32600)
        self.assertEqual(len(self.server.store.all_subscriptions()), before)
        self.assertEqual(self.request({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":[]}})[0], 401)
        self.assertEqual(self.request({"jsonrpc":"2.0","id":1,"method":"future/mutate"})[0], 401)

    def test_resource_metadata_routes_and_scope_configuration(self):
        original = json.loads(self.config.read_text())
        for path in ("/mcp", "/tenant/mcp", "/one/two/mcp"):
            with self.subTest(path=path):
                config = {**original, "resource": "https://mcp-test.devwits.ai" + path}
                self.config.write_text(json.dumps(config))
                auth = SE.HttpAuth(str(self.config))
                self.httpd.RequestHandlerClass = SE.make_http_handler(self.server, path, auth)
                status, headers, _ = self.request({"jsonrpc":"2.0", "id":1, "method":"events/unsubscribe"}, path=path)
                self.assertEqual(status, 401)
                metadata_path = "/.well-known/oauth-protected-resource" + path
                self.assertIn('resource_metadata="https://mcp-test.devwits.ai' + metadata_path + '"', headers["WWW-Authenticate"])
                for discovery in (metadata_path, "/.well-known/oauth-protected-resource"):
                    status, _, body = self.request(path=discovery, method="GET")
                    self.assertEqual((status, body["resource"], body["scopes_supported"]), (200, config["resource"], ["mcp:write"]))
                self.assertEqual(self.request(path=metadata_path, method="GET", headers={"Host":"evil.example"})[0], 404)
                self.assertEqual(self.request(path=metadata_path, method="GET", headers={"Origin":"https://evil.example"})[0], 404)
        for path in ("", "/", "/mcp/", "/tenant/mcp/", "//mcp"):
            with self.subTest(rejected_path=path):
                self.config.write_text(json.dumps({**original, "resource":"https://mcp-test.devwits.ai" + path}))
                with self.assertRaises(ValueError): SE.HttpAuth(str(self.config))
        for scope in ("", "two scopes", 'mcp:write"', "mcp:write\\evil", "mcp:write\r\nInjected: yes", None, 1):
            with self.subTest(rejected_scope=scope):
                self.config.write_text(json.dumps({**original, "write_scope":scope}))
                with self.assertRaises(ValueError): SE.HttpAuth(str(self.config))

    def test_stytch_profile_requires_bound_client_project_and_resource(self):
        config = json.loads(self.config.read_text())
        client = "connected-app-test-7607c19e-b18a-4ef5-b675-480c2b7457c5"
        project = "project-test-5f16c2cc-3020-4a9a-93af-b3a0f22aa9bd"
        config.update(profile="stytch-connected-apps-v1", expected_client_id=client, expected_project_id=project)
        self.config.write_text(json.dumps(config))
        auth = SE.HttpAuth(str(self.config))
        self.httpd.RequestHandlerClass = SE.make_http_handler(self.server, "/mcp", auth)
        msg = {"jsonrpc":"2.0", "id":1, "method":"events/unsubscribe", "params":{}}
        claims = {"client_id":client, "aud":[project, RESOURCE]}
        token = lambda extra=None, header=None: self.token(claims | (extra or {}), {"typ":"JWT"} | (header or {}))
        self.assertEqual(self.request(msg, token())[0], 200)
        self.assertEqual(self.request(msg, token({"scope":"openid"}))[0], 403)
        now = int(time.time())
        for value in ({"aud":[project]}, {"aud":[RESOURCE]}, {"aud":[project,"https://wrong/mcp"]}, {"client_id":"other"}, {"client_id":None}, {"jti":None}, {"sub":"other"}, {"iss":"https://wrong"}, {"exp":now-1}, {"iat":now+60}, {"nbf":now+60}):
            with self.subTest(value=value): self.assertEqual(self.request(msg, token(value))[0], 401)
        self.assertEqual(self.request(msg, token(header={"typ":"at+jwt"}))[0], 401)
        valid = token(); self.assertEqual(self.request(msg, valid.rsplit(".",1)[0]+"."+"A"*342)[0], 401)
        for change in ({"expected_client_id":""}, {"expected_project_id":None}, {"profile":"unknown"}, {"profile":None}):
            self.config.write_text(json.dumps(config | change))
            with self.assertRaises(ValueError): SE.HttpAuth(str(self.config))
        # Omission keeps the original strict at+jwt profile for direct callers.
        strict = {k:v for k,v in config.items() if k not in {"profile", "expected_client_id", "expected_project_id"}}
        strict_token = self.token()
        strict_result = subprocess.run([str(VERIFIER)], input=json.dumps({"config":strict, "token":strict_token}).encode(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3, check=True)
        self.assertEqual(strict_result.stdout, b"allow\n")
        self.assertEqual(strict_result.stderr, b"")
        malformed_strict = strict | {"profile":{"rfc9068-v1":None}}
        strict_bad = subprocess.run([str(VERIFIER)], input=json.dumps({"config":malformed_strict, "token":strict_token}).encode(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3, check=True)
        self.assertEqual((strict_bad.stdout, strict_bad.stderr), (b"invalid\n", b""))
        # Direct verifier callers cannot bypass profile selection or client/project binding.
        for change in ({"profile":"unknown"}, {"profile":None}, {"profile":[]}, {"profile":1}, {"profile":True}, {"profile":{"stytch-connected-apps-v1":None}}, {"profile":{"rfc9068-v1":None}}, {"expected_client_id":None}, {"expected_project_id":""}):
            result = subprocess.run([str(VERIFIER)], input=json.dumps({"config":config | change, "token":token()}).encode(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3, check=True)
            self.assertEqual(result.stdout, b"invalid\n")
            self.assertEqual(result.stderr, b"")
        # JWT-shaped session/ID fixtures lacking Connected Apps client or jti are denied.
        self.assertEqual(self.request(msg, token({"client_id":None}))[0], 401)
        self.assertEqual(self.request(msg, token({"jti":None}))[0], 401)

    def test_authenticated_read_only_staging(self):
        client = "connected-app-test-7607c19e-b18a-4ef5-b675-480c2b7457c5"
        project = "project-test-5f16c2cc-3020-4a9a-93af-b3a0f22aa9bd"
        config = json.loads(self.config.read_text())
        config.update(profile="stytch-connected-apps-v1", expected_client_id=client,
                      expected_project_id=project, write_scope="openid")
        self.config.write_text(json.dumps(config))
        auth = SE.HttpAuth(str(self.config))
        self.httpd.RequestHandlerClass = SE.make_http_handler(self.server, "/mcp", auth, authenticated_read_only=True)
        signed = lambda extra=None: self.token({"client_id":client, "aud":[project, RESOURCE], "scope":"openid"} | (extra or {}), {"typ":"JWT"})
        good, insufficient = signed(), signed({"scope":"profile"})
        for method in ("initialize", "ping", "tools/list", "events/list", "notifications/initialized", "server/discover"):
            msg = {"jsonrpc":"2.0", "id":1, "method":method}
            with self.subTest(method=method):
                self.assertEqual(self.request(msg)[0], 401)
                self.assertEqual(self.request(msg, "bad")[0], 401)
                status, headers, _ = self.request(msg, insufficient)
                self.assertEqual(status, 403); self.assertIn('error="insufficient_scope"', headers["WWW-Authenticate"])
        for name in ("delivery_status", "replay_events"):
            msg = {"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":name}}
            self.assertEqual(self.request(msg)[0], 401)
        status, _, discovery = self.request({"jsonrpc":"2.0", "id":1, "method":"server/discover"}, good)
        self.assertEqual(status, 200)
        self.assertEqual(discovery["error"]["code"], -32601)
        self.server.bridge = types.SimpleNamespace(provisioning=True)
        status, _, listing = self.request({"jsonrpc":"2.0", "id":1, "method":"tools/list"}, good)
        self.assertEqual(status, 200); self.assertEqual(len(listing["result"]["tools"]), 13)
        self.assertEqual(self.request({"jsonrpc":"2.0", "method":"notifications/initialized"}, good)[0], 202)
        for method in ("initialize", "ping", "events/list"):
            self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":method}, good)[0], 200)
        self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":"delivery_status"}}, good)[0], 200)
        self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":"replay_events", "arguments":{}}}, good)[0], 200)
        mutations = [{"jsonrpc":"2.0", "id":1, "method":m} for m in ("events/subscribe", "events/unsubscribe", "future/mutate", "server/discover/extra")]
        mutations += [{"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":n}} for n in ("submit_session_request", "read_session_operation", "read_session_target", "read_session", "list_sessions", "budget_status", "future_tool")]
        with mock.patch.object(self.server, "handle", wraps=self.server.handle) as dispatch:
            for message in mutations:
                status, _, body = self.request(message, good)
                self.assertEqual((status, body), (403, {"error":"read_only"}))
            dispatch.assert_not_called()
        self.assertEqual(len(self.server.store.all_subscriptions()), 0)
        self.assertEqual(self.request(path="/.well-known/oauth-protected-resource/mcp", method="GET")[0], 200)
        self.assertEqual(self.request(path="/mcp", method="GET")[0], 401)
        self.assertEqual(self.request(token="bad", path="/mcp", method="GET")[0], 401)
        self.assertEqual(self.request(token=insufficient, path="/mcp", method="GET")[0], 403)
        self.assertEqual(self.request(token=good, path="/mcp", method="GET")[0], 405)
        self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":"ping"}, good, headers={"Origin":"https://evil.example"})[0], 403)
        self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":"ping"}, good, headers={"Host":"evil.example"})[0], 403)
        log = (self.homes.state / "requests.log").read_text()
        self.assertNotIn(good, log); self.assertNotIn(insufficient, log)

    def test_observation_only_exact_subject_and_write_boundary(self):
        seen = []
        auth = SE.HttpAuth(str(self.config))
        self.server.observation = types.SimpleNamespace(
            settings={"subject": SUBJECT, "principal": self.server.principal},
            call=lambda method, principal, arguments: seen.append((method, principal)) or {"schema_version": 1})
        self.httpd.RequestHandlerClass = SE.make_http_handler(self.server, "/mcp", auth, observation_only=True)
        msg = {"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":"pi_observation_observe", "arguments":{}}}
        for token, status in ((None,401), (self.token({"sub":"other"}),401),
                              (self.token({"exp":int(time.time())-1}),401),
                              (self.token({"scope":"openid"}),403)):
            self.assertEqual(self.request(msg,token)[0],status)
        self.assertEqual(seen,[])
        good = self.token()
        self.assertEqual(self.request(msg,good)[0],200)
        self.assertEqual(seen,[("observe",self.server.principal)])
        for name in ("submit_session_request", "create_session_target", "issue_session_request_grant", "delivery_ack"):
            msg["params"]["name"] = name
            self.assertEqual(self.request(msg,good)[0],403)
        for method in ("events/subscribe", "events/unsubscribe"):
            self.assertEqual(self.request({"jsonrpc":"2.0","id":1,"method":method},good)[0],403)
        listing = self.request({"jsonrpc":"2.0","id":1,"method":"tools/list"},good)[2]["result"]["tools"]
        self.assertEqual({t["name"] for t in listing},
                         {"pi_observation_observe", "pi_observation_status", "pi_observation_result", "delivery_status", "replay_events"})
        self.assertFalse(next(t for t in listing if t["name"]=="pi_observation_observe")["annotations"]["readOnlyHint"])

    def test_read_only_requires_auth_before_bind(self):
        with self.assertRaises(ValueError):
            SE.make_http_handler(self.server, "/mcp", authenticated_read_only=True)
        with mock.patch.object(SE.http.server, "ThreadingHTTPServer") as bind:
            with self.assertRaises(SystemExit):
                SE.serve_http(self.server, "127.0.0.1:0", authenticated_read_only=True)
            bind.assert_not_called()
        with mock.patch.object(SE.http.server, "ThreadingHTTPServer") as bind:
            with self.assertRaises(SystemExit):
                SE.main(["serve-http", "--authenticated-read-only"])
            bind.assert_not_called()

    def test_rejections_and_rotation(self):
        msg = {"jsonrpc":"2.0", "id":1, "method":"events/unsubscribe", "params":{}}
        now = int(time.time())
        for token in ["bad", "x"*16385, self.token({"exp": now-1}), self.token({"nbf":now+60}), self.token({"nbf":"invalid"}), self.token({"nbf":None}), self.token({"iat":now+60}), self.token({"client_id":""}), self.token({"iss":"https://wrong"}), self.token({"aud":"project-only"}), self.token({"sub":"other"}), self.token(header={"alg":"HS256"}), self.token(header={"typ":"JWT"}), self.token(header={"kid":"unknown"})]:
            with self.subTest(token=token[:12]): self.assertEqual(self.request(msg, token)[0], 401)
        self.snapshot.write_text(json.dumps({"keys": [{**self.jwk, "kid":"two"}]}))
        self.assertEqual(self.request(msg, self.token())[0], 401)
        self.assertEqual(self.request(msg, self.token(header={"kid":"two"}))[0], 200)
        os.utime(self.snapshot, (now-301, now-301))
        self.assertEqual(self.request(msg, self.token(header={"kid":"two"}))[0], 401)
        os.utime(self.snapshot, None); self.snapshot.write_text("bad")
        self.assertEqual(self.request(msg, self.token())[0], 401)
        logs = (self.homes.state / "requests.log").read_text() if (self.homes.state / "requests.log").exists() else ""
        self.assertNotIn(self.token(), logs)

if __name__ == "__main__": unittest.main()
