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

from test_session_events import Fixture, SE, CODEX_ID, URL, whsec

VERIFIER = Path(__file__).resolve().parents[3] / "rust/target/debug/mcp-auth"
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
            self.assertEqual((status, body), (200, {"resource": RESOURCE, "authorization_servers": [ISSUER]}))
        status, headers, _ = self.request({"jsonrpc":"2.0", "id":1, "method":"events/subscribe"})
        self.assertEqual(status, 401); self.assertIn('resource_metadata="https://mcp-test.devwits.ai/.well-known/oauth-protected-resource/mcp"', headers["WWW-Authenticate"])
        self.server.bridge = types.SimpleNamespace(provisioning=True)
        status, _, listing = self.request({"jsonrpc":"2.0", "id":1, "method":"tools/list"})
        self.assertEqual(status, 200)
        self.assertEqual(len(listing["result"]["tools"]), 13)
        self.assertEqual(self.request({"jsonrpc":"2.0", "method":"notifications/initialized"})[0], 202)
        self.assertEqual(self.request({"jsonrpc":"2.0", "id":1, "method":"ping"}, headers={"Origin":"https://evil.example"})[0], 403)

    def test_auth_before_effects_and_bad_requests(self):
        msg = {"jsonrpc":"2.0", "id":2, "method":"events/subscribe", "params": {"name": SE.SIMPLE_EVENT_NAME, "arguments":{"session_id":CODEX_ID,"pattern":"x"}, "delivery":{"mode":"webhook", "url":URL,"secret":whsec()}}}
        self.assertEqual(self.request(msg)[0], 401)
        self.assertEqual(len(self.server.store.all_subscriptions()), 0)
        self.assertEqual(self.request(msg, self.token({"scope":"openid"}))[0], 403)
        self.assertEqual(len(self.server.store.all_subscriptions()), 0)
        bearer = self.token()
        status, _, body = self.request(msg, bearer)
        self.assertEqual(status, 200); self.assertIn("result", body)
        self.assertNotIn(bearer, (self.homes.state / "requests.log").read_text())
        self.assertEqual(self.request(b'{"jsonrpc":"2.0","method":"ping","method":"events/subscribe"}')[0], 401)
        self.assertEqual(self.request(b'bad')[0], 401)
        self.assertEqual(self.request({"jsonrpc":"2.0","id":1,"method":[]})[0], 401)
        self.assertEqual(self.request({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":[]}})[0], 401)
        self.assertEqual(self.request({"jsonrpc":"2.0","id":1,"method":"future/mutate"})[0], 401)

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
