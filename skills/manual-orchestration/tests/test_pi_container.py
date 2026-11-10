"""Container mode for Pi roles: scripts/pi_container.py and role_host.py --container.

Unit tests need neither Docker nor Pi. RealContainerTests run real Pi in real
containers against tests/role_host_fakes/fake_openai_responses.py; the egress
proxy's upstream is that local fake and the key is fake, so no provider is
contacted and nothing is billed.
"""
import http.client
import json
import os
import pathlib
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import pi_container  # noqa: E402
import role_host  # noqa: E402

HOST = ROOT / "scripts" / "role_host.py"
FAKE_SERVER = pathlib.Path(__file__).parent / "role_host_fakes" / "fake_openai_responses.py"
MODEL = "openai/gpt-6.1-sol"
SID = "00000000-0000-4000-8000-000000000002"


class MountPlanTests(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        for d in ("home", "home/ws", "home/ref", "home/out", "state"):
            (self.tmp / d).mkdir()
        self.home = str(self.tmp / "home")

    def plan(self, cwd="home/ws", ro=(), rw=()):
        return pi_container.mount_plan(str(self.tmp / cwd), [str(self.tmp / p) for p in ro], [str(self.tmp / p) for p in rw],
                                       home=self.home, reserved=[str(self.tmp / "state")])

    def refused(self, **kw):
        with self.assertRaises(pi_container.ContainerError) as cm:
            self.plan(**kw)
        return cm.exception.detail

    def test_working_directory_is_read_write_unless_covered(self):
        self.assertEqual(self.plan(ro=["home/ref"]), [[str(self.tmp / "home/ref"), "ro"], [str(self.tmp / "home/ws"), "rw"]])
        self.assertEqual(self.plan(cwd="home/ws", ro=["home/ws"]), [[str(self.tmp / "home/ws"), "ro"]])

    def test_refusals(self):
        self.refused(ro=["home/missing"])
        self.assertIn("everything", self.refused(ro=["home"]))
        self.assertIn("state", self.refused(rw=["state"]))
        self.assertIn("both", self.refused(ro=["home/ref"], rw=["home/ref"]))
        with self.assertRaises(pi_container.ContainerError):
            pi_container.mount_plan(str(self.tmp / "home/ws"), ["/usr/share"], [], home=self.home)
        with self.assertRaises(pi_container.ContainerError):
            pi_container.mount_plan(str(self.tmp / "home/ws"), ["relative/path"], [], home=self.home)
        comma = self.tmp / "home" / "a,b"
        comma.mkdir()
        with self.assertRaises(pi_container.ContainerError):
            pi_container.mount_plan(str(self.tmp / "home/ws"), [str(comma)], [], home=self.home)

    def test_limits(self):
        self.assertEqual(pi_container.validate_limits("4g", 512, "2"), {"memory": "4g", "pids": 512, "cpus": "2"})
        for bad in (("4 GB", 512, "2"), ("4g", 8, "2"), ("4g", 512, "0"), ("4g", 512, "x")):
            with self.assertRaises(pi_container.ContainerError):
                pi_container.validate_limits(*bad)


def sample_env(tmp, codebase_memory=False):
    env = {"runtime": {"root": "/snap/pi-coding-agent/current", "resolved": "/snap/pi-coding-agent/77",
                       "node": "/snap/pi-coding-agent/current/node/bin/node",
                       "cli": "/snap/pi-coding-agent/current/lib/pi/packages/coding-agent/dist/cli.js", "pi_version": "1.4.0"},
           "mounts": [[f"{tmp}/ref", "ro"], [f"{tmp}/ws", "rw"]], "limits": {"memory": "4g", "pids": 512, "cpus": "2"},
           "uid": 1000, "gid": 1000, "home": "/home/u", "agent_dir": "/s/container/agent", "user": "u",
           "lang": "C.UTF-8", "selection": {"model": MODEL, "tier": "flex", "thinking": "high", "pi_argv": []}}
    if codebase_memory:
        env["codebase_memory"] = {"binary": "/home/u/.local/bin/codebase-memory-mcp"}
    return env


class DockerArgvTests(unittest.TestCase):
    def argv(self, codebase_memory=False):
        spec = {"environment": sample_env("/w", codebase_memory), "container_name": "we-pi-x-1", "session_id": SID, "turn": 1,
                "cwd": "/w/ws", "container_dir": "/s/container", "socket_dir": "/run/user/1000/we-pi/x-1",
                "socket_path": "/run/user/1000/we-pi/x-1/egress.sock",
                "container_env": {"OPENAI_API_KEY": pi_container.PLACEHOLDER_KEY, "HOME": "/home/u"},
                "files": {"prompt": "/s/container/turn-1.prompt", "events": "/s/container/turn-1.events.jsonl",
                          "stderr": "/s/container/turn-1.stderr", "exit": "/s/container/turn-1.exit"},
                "pi_argv": ["--mode", "json", "--session-id", SID]}
        return pi_container.docker_argv(spec)

    def test_hardened_and_without_credentials(self):
        argv = self.argv()
        joined = " ".join(argv)
        for flag in ("--network none", "--read-only", "--cap-drop ALL", "--security-opt no-new-privileges",
                     "--user 1000:1000", "--pids-limit 512", "--memory 4g", "--pull never"):
            self.assertIn(flag, joined)
        for path in ("/usr", "/etc"):
            self.assertIn(f"type=bind,src={path},dst={path},readonly", argv)
        self.assertIn("type=bind,src=/w/ref,dst=/w/ref,readonly", argv)
        self.assertIn("type=bind,src=/w/ws,dst=/w/ws", argv)
        self.assertIn("type=bind,src=/snap/pi-coding-agent/77,dst=/snap/pi-coding-agent/current,readonly", argv)
        keys = [a for a in argv if "sk-" in a]
        self.assertEqual(keys, [f"OPENAI_API_KEY={pi_container.PLACEHOLDER_KEY}"])
        self.assertLess(argv.index("--entrypoint"), argv.index(pi_container.IMAGE))
        self.assertNotIn("/run/cbm", joined)

    def test_codebase_memory_adds_binary_and_private_runtime(self):
        argv = self.argv(codebase_memory=True)
        self.assertIn("type=bind,src=/home/u/.local/bin/codebase-memory-mcp,dst=/home/u/.local/bin/codebase-memory-mcp,readonly", argv)
        self.assertIn(f"{pi_container.CBM_RUNTIME}:rw,nosuid,size=16m,uid=1000,gid=1000,mode=0700", argv)


class RequestCheckTests(unittest.TestCase):
    def check(self, body):
        return pi_container.check_request(body, model_id="gpt-6.1-sol")

    def test_pinned_model_and_tools(self):
        ok = {"model": "gpt-6.1-sol", "input": [{"type": "additional_tools", "tools": [
            {"type": "function", "name": "bash"}, {"type": "namespace", "tools": [{"type": "custom", "name": "exec"}]}]}]}
        self.assertIsNone(self.check(ok))
        self.assertEqual(self.check({**ok, "model": "gpt-6-astra"}), "model_not_pinned")
        self.assertEqual(self.check({**ok, "background": True}), "unexpected_field:background")
        self.assertIn("web_search", self.check({**ok, "tools": [{"type": "web_search"}]}))
        nested = {"model": "gpt-6.1-sol", "input": [{"type": "additional_tools", "tools": [
            {"type": "namespace", "tools": [{"type": "file_search"}]}]}]}
        self.assertIn("file_search", self.check(nested))
        self.assertEqual(self.check([1, 2]), "body_not_object")

    def test_every_declaration_location_and_unexamined_field(self):
        base = {"model": "gpt-6.1-sol", "input": []}
        search_output = {**base, "input": [{"type": "tool_search_output", "tools": [{"type": "web_search"}]}]}
        self.assertIn("web_search", self.check(search_output))  # review finding R1
        allowed = {**base, "tool_choice": {"type": "allowed_tools", "mode": "auto", "tools": [{"type": "mcp"}]}}
        self.assertIn("mcp", self.check(allowed))
        self.assertEqual(self.check({**base, "tool_choice": {"type": "web_search_preview"}}), "tool_choice_refused")
        self.assertIsNone(self.check({**base, "tool_choice": "auto"}))
        for field in ("prompt", "previous_response_id", "conversation", "background"):
            self.assertEqual(self.check({**base, field: "x"}), f"unexpected_field:{field}")
        self.assertEqual(self.check({**base, "store": True}), "store_refused")
        pi_request = {"model": "gpt-6.1-sol", "input": [], "include": ["reasoning.encrypted_content"], "store": False,
                      "stream": True, "reasoning": {"effort": "medium"}, "service_tier": "flex",
                      "max_output_tokens": 128000, "prompt_cache_key": "k"}
        self.assertIsNone(self.check(pi_request))


class Upstream(BaseHTTPRequestHandler):
    """Records what the proxy sent and streams a terminal event echoing the tier."""
    seen = []

    def log_message(self, *a):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["content-length"])))
        Upstream.seen.append({"path": self.path, "auth": self.headers.get("authorization"), "body": body})
        event = {"type": "response.completed", "response": {"id": "r1", "model": body["model"],
                 "service_tier": body.get("service_tier", "default"),
                 "usage": {"input_tokens": 7, "output_tokens": 3,
                           "input_tokens_details": {"cached_tokens": 2, "cache_write_tokens": 4}}}}
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.end_headers()
        self.wfile.write(f"event: response.completed\ndata: {json.dumps(event)}\n\n".encode())


class UnixHTTP(http.client.HTTPConnection):
    def __init__(self, path):
        super().__init__("localhost")
        self.unix_path = path

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX)
        self.sock.connect(self.unix_path)


class EgressProxyTests(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        Upstream.seen = []
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
        threading.Thread(target=self.upstream.serve_forever, daemon=True).start()
        self.addCleanup(self.upstream.server_close)
        self.addCleanup(self.upstream.shutdown)

    def proxy(self, tier=None, max_requests=None, **bounds):
        p = pi_container.EgressProxy(str(self.tmp / "egress.sock"), upstream=f"http://127.0.0.1:{self.upstream.server_port}/v1",
                                     key="sk-real-key-held-by-host", model_id="gpt-6.1-sol", tier=tier,
                                     log_path=str(self.tmp / "proxy.jsonl"), max_requests=max_requests, **bounds).start()
        self.addCleanup(p.stop)
        return p

    def post(self, body, path="/v1/responses", method="POST"):
        conn = UnixHTTP(str(self.tmp / "egress.sock"))
        conn.request(method, path, json.dumps(body), {"authorization": f"Bearer {pi_container.PLACEHOLDER_KEY}",
                                                      "content-type": "application/json"})
        resp = conn.getresponse()
        data = resp.read()
        conn.close()
        return resp.status, data

    def log(self):
        return [json.loads(line) for line in (self.tmp / "proxy.jsonl").read_text().splitlines()]

    def test_key_substituted_tier_enforced_and_observed(self):
        self.proxy(tier="flex")
        status, data = self.post({"model": "gpt-6.1-sol", "service_tier": "priority", "input": []})
        self.assertEqual(status, 200)
        self.assertIn(b"response.completed", data)
        sent = Upstream.seen[-1]
        self.assertEqual(sent["auth"], "Bearer sk-real-key-held-by-host")
        self.assertEqual((sent["path"], sent["body"]["service_tier"]), ("/v1/responses", "flex"))
        row = self.log()[-1]
        self.assertEqual((row["requested_tier"], row["observed_tier"], row["usage"]["input_tokens"]), ("flex", "flex", 7))
        self.assertEqual((row["usage"]["cached_tokens"], row["usage"]["cache_write_tokens"]), (2, 4))

    def test_untiered_requests_lose_any_tier(self):
        self.proxy(tier=None)
        self.post({"model": "gpt-6.1-sol", "service_tier": "priority", "input": []})
        self.assertNotIn("service_tier", Upstream.seen[-1]["body"])

    def test_refusals_never_reach_upstream(self):
        self.proxy(max_requests=1)
        self.assertEqual(self.post({"model": "gpt-6-astra", "input": []})[0], 403)
        self.assertEqual(self.post({}, path="/v1/files")[0], 403)
        self.assertEqual(self.post({}, path="/v1/models", method="GET")[0], 403)
        self.assertEqual(self.post({"model": "gpt-6.1-sol", "input": [], "tools": [{"type": "web_search"}]})[0], 403)
        self.assertEqual(Upstream.seen, [])
        self.assertEqual(self.post({"model": "gpt-6.1-sol", "input": []})[0], 200)
        self.assertEqual(self.post({"model": "gpt-6.1-sol", "input": []})[0], 429)
        self.assertEqual(len(Upstream.seen), 1)
        reasons = [r.get("reason") for r in self.log() if r["event"] == "refused"]
        self.assertEqual(len(reasons), 5)

    def raw(self, data: bytes, wait=5.0):
        """Send raw bytes and return what comes back before the server closes."""
        conn = socket.socket(socket.AF_UNIX)
        conn.settimeout(wait)
        conn.connect(str(self.tmp / "egress.sock"))
        conn.sendall(data)
        got = b""
        try:
            while chunk := conn.recv(65536):
                got += chunk
        except socket.timeout:
            pass
        conn.close()
        return got

    def test_framing_and_size_refused_before_reading(self):  # review finding R4
        self.proxy(max_body=1000)
        head = b"POST /v1/responses HTTP/1.1\r\nhost: x\r\n"
        self.assertIn(b" 411 ", self.raw(head + b"\r\n"))
        self.assertIn(b" 411 ", self.raw(head + b"transfer-encoding: chunked\r\n\r\n0\r\n\r\n"))
        self.assertIn(b" 413 ", self.raw(head + b"content-length: 999999999\r\n\r\nx"))
        self.assertEqual(Upstream.seen, [])

    def test_cap_checked_before_the_body_is_read(self):
        self.proxy(max_requests=0)
        started = time.time()
        reply = self.raw(b"POST /v1/responses HTTP/1.1\r\ncontent-length: 500\r\n\r\n")
        self.assertIn(b" 429 ", reply)
        self.assertLess(time.time() - started, 2)

    def test_connection_bound_and_slow_body_timeout(self):
        p = self.proxy(max_connections=2, socket_timeout=1)
        held = []
        for _ in range(2):
            c = socket.socket(socket.AF_UNIX)
            c.connect(str(self.tmp / "egress.sock"))
            c.sendall(b"POST /v1/responses HTTP/1.1\r\ncontent-length: 50\r\n\r\nx")
            held.append(c)
        time.sleep(0.3)
        extra = socket.socket(socket.AF_UNIX)
        extra.settimeout(2)
        extra.connect(str(self.tmp / "egress.sock"))
        self.assertEqual(extra.recv(10), b"")  # closed at once, no thread spawned
        extra.close()
        self.assertGreaterEqual(p.dropped_connections, 1)
        time.sleep(1.5)  # the held bodies time out and their slots free
        for c in held:
            c.close()
        self.assertTrue(any(r.get("reason") == "body not received in time" for r in self.log()))
        self.assertEqual(self.post({"model": "gpt-6.1-sol", "input": []})[0], 200)


def runtime_fixture(root: pathlib.Path):
    (root / "node/bin").mkdir(parents=True)
    (root / "node/bin/node").write_text("node")
    (root / "lib/pi/packages/coding-agent/dist").mkdir(parents=True)
    (root / "lib/pi/packages/coding-agent/dist/cli.js").write_text("cli v1")
    (root / "lib/pi/packages/coding-agent/package.json").write_text('{"version": "1.4.0"}')
    return pi_container.runtime_paths(str(root))


class EnvironmentCheckTests(unittest.TestCase):
    """role_host refuses a later turn in a changed environment (review finding R2)."""

    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        for d in ("home/ws", "home/ref", "home/other", "state"):
            (self.tmp / d).mkdir(parents=True)
        self.ledger = role_host.Ledger(self.tmp / "state")
        cwd = str(self.tmp / "home/ws")
        self.env = {"runtime": runtime_fixture(self.tmp / "runtime"),
                    "mounts": pi_container.mount_plan(cwd, [str(self.tmp / "home/ref")], [], home=str(self.tmp / "home")),
                    "limits": {"memory": "4g", "pids": 512, "cpus": "2"}, "uid": 1000, "gid": 1000,
                    "home": str(self.tmp / "home"), "agent_dir": str(self.tmp / "agent"), "user": "u", "lang": "C.UTF-8",
                    "selection": {"model": MODEL, "tier": None, "thinking": None, "pi_argv": []}}
        self.env["fingerprint"] = pi_container.fingerprint(self.env)
        self.cwd = cwd

    def check(self):
        return role_host.Pi().__class__._check_environment(role_host.Pi(), self.env, cwd=self.cwd, ledger=self.ledger)

    def test_unchanged_passes(self):
        self.check()

    def test_changed_runtime_bytes_refused(self):
        (self.tmp / "runtime/lib/pi/packages/coding-agent/dist/cli.js").write_text("cli v2, same version")
        with self.assertRaises(role_host.HostError) as cm:
            self.check()
        self.assertEqual(cm.exception.status, "environment_changed")
        self.assertIn("runtime_identity", cm.exception.detail)

    def test_mount_retargeted_by_symlink_refused(self):
        ref = self.tmp / "home/ref"
        ref.rmdir()
        ref.symlink_to(self.tmp / "home/other")
        with self.assertRaises(role_host.HostError) as cm:
            self.check()
        self.assertEqual(cm.exception.status, "environment_changed")

    def test_replaced_directory_refused(self):
        ref = self.tmp / "home/ref"
        ref.rmdir()
        ref.mkdir()  # same path, new inode
        with self.assertRaises(role_host.HostError) as cm:
            self.check()
        self.assertIn("mount_identities", cm.exception.detail)


class FingerprintTests(unittest.TestCase):
    def test_changes_with_what_the_agent_sees(self):
        env = sample_env("/w")
        a = pi_container.fingerprint(env)
        self.assertEqual(a["sha256"], pi_container.fingerprint(env)["sha256"])
        b = pi_container.fingerprint({**env, "mounts": env["mounts"] + [["/w/extra", "ro"]]})
        c = pi_container.fingerprint({**env, "limits": {**env["limits"], "memory": "8g"}})
        self.assertEqual(len({a["sha256"], b["sha256"], c["sha256"]}), 3)


def container_prerequisites():
    if not shutil.which("docker") or not shutil.which("pi"):
        return "docker or pi missing"
    if subprocess.run(["docker", "image", "inspect", pi_container.IMAGE], capture_output=True).returncode != 0:
        return "pinned image not present"
    try:
        pi_container.runtime_paths()
    except pi_container.ContainerError as e:
        return e.detail
    return None


SKIP = container_prerequisites()


@unittest.skipIf(SKIP, SKIP or "")
class RealContainerTests(unittest.TestCase):
    """Real Pi in real containers; each turn takes a few seconds."""

    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="rhpc-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        for d in ("agent", "home", "ws", "ref", "hidden", "state", "run"):
            (self.tmp / d).mkdir()
        os.chmod(self.tmp / "run", 0o700)
        subprocess.run(["git", "init", "-q", str(self.tmp / "ws")], check=True)
        (self.tmp / "ws" / "AGENTS.md").write_text("CONTAINER-TEST\n")
        (self.tmp / "ws" / "mod.py").write_text("def hello():\n    return 1\n")
        (self.tmp / "hidden" / "secret.txt").write_text("not for the agent\n")
        (self.tmp / "agent" / "settings.json").write_text("{}")
        self.mode = self.tmp / "mode.json"
        self.mode.write_text("{}")
        self.log = self.tmp / "requests.jsonl"
        port_file = self.tmp / "port"
        self.server = subprocess.Popen([sys.executable, str(FAKE_SERVER), "--port-file", str(port_file),
                                        "--log", str(self.log), "--mode-file", str(self.mode)])
        self.addCleanup(self.server.wait)
        self.addCleanup(self.server.terminate)
        for _ in range(100):
            if port_file.exists():
                break
            time.sleep(0.05)
        self.env = {"PATH": os.environ["PATH"], "HOME": str(self.tmp / "home"), "USER": os.environ.get("USER", ""),
                    "XDG_RUNTIME_DIR": str(self.tmp / "run"), "PI_CODING_AGENT_DIR": str(self.tmp / "agent"),
                    "PI_OFFLINE": "1", "PI_TELEMETRY": "0", "OPENAI_API_KEY": "sk-fake-container-test",
                    "ROLE_HOST_PI_UPSTREAM": f"http://127.0.0.1:{port_file.read_text()}/v1",
                    "ROLE_HOST_CBM_EXE": pi_container.default_codebase_memory()}
        self.addCleanup(self.remove_containers)

    def remove_containers(self):
        ledger = self.tmp / "state" / "ledger.jsonl"
        if ledger.exists():
            for line in ledger.read_text().splitlines():
                name = (json.loads(line).get("container") or {}).get("name")
                if name:
                    subprocess.run(["docker", "rm", "-f", name], capture_output=True)

    def host(self, *args):
        r = subprocess.run([sys.executable, str(HOST), "--state", str(self.tmp / "state"), *args],
                           capture_output=True, text=True, env=self.env, timeout=300)
        return r.returncode, (json.loads(r.stdout) if r.returncode == 0 else json.loads(r.stderr or "{}"))

    def launch(self, *extra):
        prompt = self.tmp / f"prompt-{time.monotonic_ns()}"
        prompt.write_text("Run the check.")
        return self.host("launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "container-test",
                         "--prompt-file", str(prompt), "--auth", "api", "--container",
                         "--host-arg=--model", f"--host-arg={MODEL}", *extra)

    def wait_idle(self, sid):
        for _ in range(600):
            code, st = self.host("status", "pi", sid)
            if code != 0 or st["state"] != "running":
                return code, st
            time.sleep(0.1)
        self.fail("turn still running")

    def tool_outputs(self):
        out = []
        for line in self.log.read_text().splitlines():
            for item in json.loads(line)["body"].get("input", []):
                if item.get("type") == "function_call_output":
                    out.append(str(item.get("output")))
        return out

    def test_isolation_from_inside_tier_deliver_and_environment_change(self):
        host_git = subprocess.run(["git", "--version"], capture_output=True, text=True).stdout.strip()
        hidden = self.tmp / "hidden" / "secret.txt"
        self.mode.write_text(json.dumps({"tool_command": (
            "python3 -c 'import socket; socket.create_connection((\"1.1.1.1\", 443), 2)' 2>&1 | tail -1; "
            f"test -e {hidden} && echo HIDDEN-VISIBLE || echo HIDDEN-ABSENT; echo KEY=$OPENAI_API_KEY; git --version; "
            f"cat {self.tmp / 'ref'}/r.txt; touch {self.tmp / 'ref'}/w 2>&1 | tail -1")}))
        (self.tmp / "ref" / "r.txt").write_text("REF-READABLE\n")
        code, row = self.launch("--service-tier", "priority", "--mount-ro", str(self.tmp / "ref"))
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        code, st = self.wait_idle(sid)
        turn = st["last_turn"]
        self.assertEqual((turn["outcome"], turn["service_tier"]["status"], turn["egress"]["requests"]),
                         ("completed", "applied", 2), turn)
        self.assertTrue(turn["container"]["removed"])
        seen = self.tool_outputs()[-1]
        for expected in ("Network is unreachable", "HIDDEN-ABSENT", f"KEY={pi_container.PLACEHOLDER_KEY}",
                         host_git, "REF-READABLE", "Read-only file system"):
            self.assertIn(expected, seen)
        self.assertNotIn("sk-fake-container-test", self.log.read_text())
        self.assertEqual(self.host("identity", "pi", sid)[0], 0)

        message = self.tmp / "msg"
        message.write_text("Again.")
        self.mode.write_text("{}")
        code, got = self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws"), "--message-file", str(message))
        self.assertEqual((code, got.get("turn")), (0, 2), got)
        self.assertEqual(self.wait_idle(sid)[1]["last_turn"]["outcome"], "completed")

        shutil.rmtree(self.tmp / "ref")
        code, err = self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws"), "--message-file", str(message))
        self.assertEqual((code, err.get("status")), (role_host.NOT_ESTABLISHED, "environment_changed"), err)

    def test_force_retire_removes_the_container(self):
        self.mode.write_text(json.dumps({"delay": 120}))
        code, row = self.launch()
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        self.assertEqual(self.host("status", "pi", sid)[1]["state"], "running")
        self.assertEqual(self.host("retire", "pi", sid)[0], role_host.BUSY)
        code, got = self.host("retire", "pi", sid, "--force")
        self.assertEqual((code, got.get("retired")), (0, "turn_terminated"), got)
        code, st = self.host("status", "pi", sid)
        self.assertEqual((st["state"], st["last_turn"]["outcome"]), ("stopped", "interrupted"), st)
        name = st["last_turn"]["container"]["name"]
        self.assertNotEqual(subprocess.run(["docker", "inspect", name], capture_output=True).returncode, 0)

    @unittest.skipUnless(os.access(pi_container.default_codebase_memory(), os.X_OK), "codebase-memory-mcp not installed")
    def test_codebase_memory_uses_the_session_index(self):
        code, row = self.launch("--codebase-memory")
        self.assertEqual(code, 0, row)
        cbm = row["container_environment"]["codebase_memory"]
        self.assertEqual((cbm["check"]["status"], cbm["root_path"]), ("ready", str(self.tmp / "ws")))
        self.assertIn("codebase_memory", row["container_environment"]["fingerprint"])
        sid = row["session_id"]
        self.wait_idle(sid)
        self.mode.write_text(json.dumps({"tool_call": {"name": "mcp__codebase_memory_mcp__search_graph",
                                                       "arguments": {"project": cbm["project"], "name_pattern": "hello"}}}))
        message = self.tmp / "msg"
        message.write_text("Search.")
        self.assertEqual(self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws"), "--message-file", str(message))[0], 0)
        self.wait_idle(sid)
        self.assertIn("mod.hello", self.tool_outputs()[-1])

    def test_leftovers_of_a_dead_runner_are_collected(self):  # review finding R3
        self.mode.write_text(json.dumps({"delay": 120}))
        code, row = self.launch()
        self.assertEqual(code, 0, row)
        sid, name = row["session_id"], row["container"]["name"]
        os.killpg(row["pid"], 9)  # the runner dies before its cleanup
        time.sleep(0.5)
        self.assertEqual(self.host("status", "pi", sid)[1]["kind"], "container_without_runner")
        subprocess.run(["docker", "stop", "-t", "1", name], capture_output=True)  # the container then exits
        code, st = self.host("status", "pi", sid)
        self.assertEqual(st["leftovers"]["container"], name, st)
        self.assertIn("socket_dir", st["leftovers"])
        code, got = self.host("retire", "pi", sid)
        self.assertEqual(code, 0, got)
        self.assertEqual(got["collected"]["container"], name)
        self.assertNotEqual(subprocess.run(["docker", "inspect", name], capture_output=True).returncode, 0)
        self.assertFalse(pathlib.Path(got["collected"]["socket_dir"]).exists())

    def test_refusals_before_any_effect(self):
        self.assertEqual(self.launch("--auth", "subscription")[0], role_host.USAGE)
        code, err = self.host("launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "x", "--prompt-file", str(self.mode),
                              "--auth", "api", "--mount-ro", str(self.tmp / "ref"), "--host-arg=--model", f"--host-arg={MODEL}")
        self.assertEqual(code, role_host.USAGE, err)
        self.assertEqual(self.launch("--mount-ro", str(self.tmp / "home"))[0], role_host.USAGE)
        self.assertFalse((self.tmp / "state" / "ledger.jsonl").exists())


if __name__ == "__main__":
    unittest.main()
