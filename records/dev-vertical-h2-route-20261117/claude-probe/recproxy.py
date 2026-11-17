#!/usr/bin/env python3
"""Recording reverse proxy for api.anthropic.com (probe only).

Retains each request body byte-exact, request header names (credential values
replaced by their length), response status/headers and the response body as
streamed. Usage: recproxy.py <port> <outdir>
"""
import http.client, http.server, json, os, ssl, sys, threading, hashlib, time

PORT, OUT = int(sys.argv[1]), sys.argv[2]
os.makedirs(OUT, exist_ok=True)
SECRET = {"authorization", "x-api-key", "cookie", "proxy-authorization"}
HOP = {"host", "connection", "keep-alive", "transfer-encoding", "content-length", "accept-encoding", "proxy-connection", "te", "upgrade"}
lock = threading.Lock()
counter = [0]


class H(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def handle_one(self, method):
        with lock:
            counter[0] += 1
            n = counter[0]
        length = int(self.headers.get("content-length") or 0)
        body = self.rfile.read(length) if length else b""
        stem = os.path.join(OUT, f"{n:04d}")
        with open(stem + ".req.body", "wb") as f:
            f.write(body)
        hdrs = []
        for k, v in self.headers.items():
            hdrs.append([k, f"<redacted len={len(v)}>" if k.lower() in SECRET else v])
        meta = {"n": n, "t": time.time(), "method": method, "path": self.path, "headers": hdrs,
                "body_sha256": hashlib.sha256(body).hexdigest(), "body_len": len(body)}
        fwd = {k: v for k, v in self.headers.items() if k.lower() not in HOP}
        fwd["Host"] = "api.anthropic.com"
        fwd["Accept-Encoding"] = "identity"
        conn = http.client.HTTPSConnection("api.anthropic.com", 443, context=ssl.create_default_context(), timeout=600)
        try:
            conn.request(method, self.path, body=body, headers=fwd)
            resp = conn.getresponse()
            meta["status"] = resp.status
            meta["resp_headers"] = [[k, v] for k, v in resp.getheaders() if k.lower() not in {"set-cookie"}]
            self.send_response(resp.status)
            for k, v in resp.getheaders():
                if k.lower() not in HOP and k.lower() != "set-cookie":
                    self.send_header(k, v)
            self.send_header("Transfer-Encoding", "chunked")
            self.end_headers()
            with open(stem + ".resp.body", "wb") as f:
                while True:
                    chunk = resp.read1(65536)
                    if not chunk:
                        break
                    f.write(chunk)
                    self.wfile.write(b"%x\r\n%s\r\n" % (len(chunk), chunk))
                    self.wfile.flush()
            self.wfile.write(b"0\r\n\r\n")
            self.wfile.flush()
        except Exception as e:
            meta["error"] = repr(e)
            try:
                self.send_error(502)
            except Exception:
                pass
        finally:
            conn.close()
            with open(stem + ".meta.json", "w") as f:
                json.dump(meta, f, indent=1)

    def do_POST(self):
        self.handle_one("POST")

    def do_GET(self):
        self.handle_one("GET")


http.server.ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()
