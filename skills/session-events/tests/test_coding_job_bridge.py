"""Opt-in Events MCP consumer of the existing Rust `work-engine-command` coding-job operations.

Offline only. The backend is a disposable local fake HTTP server speaking the LocalClient wire
format; it is narrower than the native server and proves nothing about native admission or
runtime. The real Rust CLI (set CODING_JOB_COMMAND_BINARY to a binary built from this tree)
performs the strict DTO validation. Without that variable the real-CLI tests are skipped and
only the stub-binary protocol tests run.
"""
import hashlib
import http.client
import http.server
import json
import os
import shutil
import stat
import tempfile
import threading
import time
import unittest
from unittest import mock
from pathlib import Path

from test_session_events import SE

COMMAND = os.environ.get("CODING_JOB_COMMAND_BINARY")
SERVICE_TOKEN = "s" * 40            # service transport credential: never job authority
JOB_CREDENTIAL = "j" * 40           # separate exact job credential
AUTHORIZATION = "fixture-auth"
REQUEST = "11111111-1111-4111-8111-111111111111"
OTHER_REQUEST = "22222222-2222-4222-8222-222222222222"
JOB_SHA, PACKET_SHA = "d" * 64, "e" * 64
DIGEST, RECEIPT, OPERATION = "b" * 64, "c" * 64, "a" * 64


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def private(path, data, mode=0o600):
    path.write_bytes(data if isinstance(data, bytes) else data.encode())
    path.chmod(mode)
    return path


def operation(request_id=REQUEST, state="collecting", submit=False, **over):
    value = {
        "schema": "work-engine.coding-job.v1", "operation_id": OPERATION, "request_id": request_id,
        "authorization_id": AUTHORIZATION, "job_id": "fixture", "state_dir": "/private/state",
        "job_sha256": JOB_SHA, "expected_host_input_digest": DIGEST, "host_input_digest": None,
        "state": state, "entered_possible": True, "collector_live": state == "collecting",
        "host_phase": None, "host_removed": None, "host_receipt_id": None,
        "collector_run": None, "collector_reconcile": None,
        "process_cleanup": {"run": "not_established", "reconcile": None},
        "limitations": ["fake backend; native admission not established"],
        "settlement": "not_established_by_submission"}
    if submit:
        value["entry"] = "collector_started"
    value.update(over)
    return value


def collected_result(request_id=REQUEST):
    op = operation(request_id, "collected", host_input_digest=DIGEST, host_phase="collected",
                   host_removed=True, host_receipt_id=RECEIPT, collector_run={},
                   process_cleanup={"run": "owned_process_group_collected", "reconcile": None})
    return {"schema": "work-engine.coding-job.v1", "result": "available", "operation": op,
            "terminal": {"status": "component_passed", "phase": "collected"},
            "validation": {"rpc_verified": True, "failures": [], "coding_completed": "not_established"},
            "cleanup": {"removed": True, "daemon_exit": None, "tool_removed": "not_established",
                        "collector": {"run": "owned_process_group_collected", "reconcile": None}},
            "acceptance": "owner_pending", "domain_effect_settlement": "not_claimed",
            "receipt_id": RECEIPT, "input_digest": DIGEST,
            "receipt": {"job_id": "fixture", "input_digest": DIGEST, "usage": None},
            "host_output": {"action": "run", "exit_code": None, "note": "verbatim Host stdout"}}


class Fake:
    """Local fake of the server endpoint. Records every request; behaviour is set per test."""

    def __init__(self):
        self.requests, self.mode, self.state, self.lock = [], "normal", "collecting", threading.Lock()
        self.bound = None
        fake = self

        class Handler(http.server.BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                with fake.lock:
                    fake.requests.append({"path": self.path, "authorization": self.headers.get("Authorization"),
                                          "body": body})
                    reply = fake.respond(body)
                if reply == "hang":
                    time.sleep(3)
                    return
                if reply == "drop":
                    self.connection.close()
                    return
                data = json.dumps(reply).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.send_header("Connection", "close")
                self.end_headers()
                self.wfile.write(data)

            def log_message(self, *args):
                pass

        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.port = self.httpd.server_address[1]
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()

    def envelope(self, body, **member):
        return {"schema_version": 1, "request_id": body["request_id"], "server_identity": "sha256:" + "1" * 64,
                "server_build": "sha256:" + "2" * 64, "owner_result": None, "owner_error": None, "error": None} | \
               {k: v for k, v in member.items()}

    def respond(self, body):
        if self.mode in ("hang", "drop"):
            return self.mode
        op, args = body["operation"], body["arguments"]
        request_id = args["request_id"]

        def out(**member):
            env = {"schema_version": 1, "request_id": body["request_id"], "server_identity": "sha256:" + "1" * 64,
                   "server_build": "sha256:" + "2" * 64}
            return env | member
        if self.mode == "null_operation_id":
            return out(owner_result=operation(request_id, submit=True, operation_id=None))
        if self.mode == "extra_field":
            return out(owner_result=operation(request_id, submit=True) | {"unexpected": True})
        if self.mode == "other_identity":
            return out(owner_result=operation(OTHER_REQUEST, submit=True))
        if op == "submit_coding_job":
            if self.bound is None:
                self.bound = request_id
                return out(owner_result=operation(request_id, "collecting", submit=True))
            if self.bound != request_id:
                return out(owner_error={"code": "authorization_consumed"})
            return out(owner_result=operation(request_id, self.state, submit=True) | {"entry": "existing_operation"})
        if self.bound != request_id:
            return out(owner_error={"code": "identity_conflict"})
        if op == "read_coding_job_operation":
            return out(owner_result=operation(request_id, self.state))
        if op == "read_coding_job_result":
            if self.state == "collected":
                return out(owner_result=collected_result(request_id))
            return out(owner_result={"schema": "work-engine.coding-job.v1", "result": "not_available",
                                     "operation": operation(request_id, self.state)})
        return out(owner_result={"schema": "work-engine.coding-job.v1", "operation_id": OPERATION,
                                 "request_id": request_id, "cancellation": "requested",
                                 "cancellation_established": False, "container_closure_established": False,
                                 "settlement": "not_claimed", "next": "read_coding_job_result"})

    def close(self):
        self.httpd.shutdown()
        self.httpd.server_close()


class Base(unittest.TestCase):
    principal, subject = "events-principal", "oauth-subject"
    command_timeout = 1

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="cjb-"))
        self.root.chmod(0o700)
        self.addCleanup(shutil.rmtree, self.root, True)
        self.fake = Fake()
        self.addCleanup(self.fake.close)

    def profile(self, port=None, timeout=None):
        credential = private(self.root / "service.token", SERVICE_TOKEN)
        return private(self.root / "profile.json", json.dumps({
            "version": 1, "kind": "local-service", "endpoint": f"127.0.0.1:{port or self.fake.port}",
            "credential_file": str(credential), "timeout_seconds": timeout or self.command_timeout}))

    def settings(self, profile, **over):
        value = {"schema": "work-engine.mcp-coding-job-bridge.v1", "subject": self.subject,
                 "principal": self.principal, "client": "configured-client",
                 "command_profile": str(profile), "command_profile_sha256": sha(profile),
                 "authorization_ids": [AUTHORIZATION], "timeout_seconds": self.command_timeout}
        return value | over

    def pins(self, binary, settings, name="bridge"):
        staged = self.root / ("bin-" + name)
        if not staged.exists():
            shutil.copyfile(binary, staged)
            staged.chmod(0o700)
        config = private(self.root / (name + ".json"), json.dumps(settings))
        return str(staged), sha(staged), str(config), sha(config)

    def server(self, binary, name="bridge", **over):
        profile = self.profile()
        bridge = SE.CodingJobBridge(*self.pins(binary, self.settings(profile, **over), name=name))
        homes = SE.Homes(self.root / "codex", self.root / "claude", self.root / "state")
        return SE.Server(homes, self.principal, SE.CallbackPolicy(), coding_job=bridge), bridge

    def stub(self, body, name="stub"):
        path = self.root / (name + ".src")
        private(path, "#!/bin/sh\n" + body, 0o700)
        return path

    @staticmethod
    def call(server, name, arguments):
        reply = server.handle({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                               "params": {"name": name, "arguments": arguments}})
        return reply["result"] if "result" in reply else reply

    @staticmethod
    def args(op="submit_coding_job", request_id=REQUEST):
        value = {"request_id": request_id, "authorization_id": AUTHORIZATION, "job_credential": JOB_CREDENTIAL}
        if op == "submit_coding_job":
            value |= {"job_sha256": JOB_SHA, "packet_sha256": PACKET_SHA}
        return value

    @staticmethod
    def payload(result):
        return json.loads(result["content"][0]["text"])


class DisabledAndPins(Base):
    def test_default_off_discovery_is_unchanged(self):
        homes = SE.Homes(self.root / "codex", self.root / "claude", self.root / "state")
        server = SE.Server(homes, self.principal, SE.CallbackPolicy())
        names = {t["name"] for t in server.handle({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})["result"]["tools"]}
        self.assertFalse(names & set(SE.CODING_JOB_OPERATIONS))
        self.assertEqual(names, {t["name"] for t in SE.TOOLS})
        reply = server.handle({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                               "params": {"name": "submit_coding_job", "arguments": self.args()}})
        self.assertEqual(reply["error"]["data"]["reason"], "unknown_tool")
        keys = ("CODING_JOB_BRIDGE_ENABLED", "CODING_JOB_BINARY", "CODING_JOB_BINARY_SHA256",
                "CODING_JOB_CONFIG", "CODING_JOB_CONFIG_SHA256")
        saved = {k: os.environ.pop(k, None) for k in keys}
        try:
            self.assertIsNone(SE.CodingJobBridge.from_environment())
            os.environ["CODING_JOB_BRIDGE_ENABLED"] = "1"
            with self.assertRaises(ValueError):
                SE.CodingJobBridge.from_environment()  # switch without pins
            del os.environ["CODING_JOB_BRIDGE_ENABLED"]
            os.environ["CODING_JOB_BINARY"] = "/x"
            with self.assertRaises(ValueError):
                SE.CodingJobBridge.from_environment()  # pins without switch
        finally:
            for k in keys:
                os.environ.pop(k, None)
                if saved[k] is not None:
                    os.environ[k] = saved[k]

    def test_enabled_tools_annotations_and_no_issuance_surface(self):
        marker = self.root / "ran"
        server, _ = self.server(self.stub(f"touch {marker}\n"))
        tools = server.handle({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})["result"]["tools"]
        job = {t["name"]: t for t in tools if t["name"] in SE.CODING_JOB_OPERATIONS}
        self.assertEqual(set(job), set(SE.CODING_JOB_OPERATIONS))
        self.assertFalse(any(w in t["name"] for t in tools for w in ("create", "issue", "approve", "grant")
                             if t["name"] in job))
        for tool in job.values():
            self.assertIs(tool["annotations"]["readOnlyHint"], False)  # reads may reconcile custody
            self.assertIs(tool["inputSchema"]["additionalProperties"], False)
            self.assertNotIn("qualified", json.dumps(tool).lower().replace("not ", ""))
        self.assertIs(job["cancel_coding_job"]["annotations"]["destructiveHint"], True)
        self.assertFalse(marker.exists())  # listing never runs the binary

    def test_pin_and_binding_failures_fail_closed(self):
        profile = self.profile()
        good = self.settings(profile)
        stub = self.stub("exit 0\n")
        pins = self.pins(stub, good)
        SE.CodingJobBridge(*pins)
        bad_cases = {
            "wrong binary sha": (pins[0], "0" * 64, pins[2], pins[3]),
            "wrong config sha": (pins[0], pins[1], pins[2], "0" * 64),
            "relative binary": ("bin", pins[1], pins[2], pins[3]),
        }
        for label, args in bad_cases.items():
            with self.assertRaises(ValueError, msg=label):
                SE.CodingJobBridge(*args)
        for label, settings in {
            "unknown member": good | {"binary": "/bin/sh"},
            "missing member": {k: v for k, v in good.items() if k != "client"},
            "empty acl": good | {"authorization_ids": []},
            "wrong profile sha": good | {"command_profile_sha256": "0" * 64},
            "relative profile": good | {"command_profile": "profile.json"},
            "bad schema": good | {"schema": "work-engine.mcp-pi-coding-fixture.v1"},
            "timeout": good | {"timeout_seconds": 61},
        }.items():
            with self.subTest(label), self.assertRaises(ValueError):
                SE.CodingJobBridge(*self.pins(stub, settings, name=label.replace(" ", "-")))
        group_writable = self.root / "bin-gw"
        shutil.copyfile(stub, group_writable)
        group_writable.chmod(0o770)
        with self.assertRaises(ValueError):
            SE.CodingJobBridge(str(group_writable), sha(group_writable), pins[2], pins[3])

    def test_principal_and_helper_binding(self):
        profile = self.profile()
        bridge = SE.CodingJobBridge(*self.pins(self.stub("exit 0\n"), self.settings(profile)))
        homes = SE.Homes(self.root / "codex", self.root / "claude", self.root / "state")
        with self.assertRaises(ValueError):  # configured principal differs from the Events principal
            SE.Server(homes, "someone-else", SE.CallbackPolicy(), coding_job=bridge)
        with self.assertRaises(ValueError):  # no second owner/helper alongside it
            SE.Server(homes, self.principal, SE.CallbackPolicy(), coding_job=bridge, coding=object())
        with self.assertRaises(SE.RpcError):  # direct cross-principal call
            bridge.call("submit_coding_job", "someone-else", self.args())
        auth_config = private(self.root / "auth.json", json.dumps({
            "resource": "https://mcp.example/mcp", "issuer": "https://issuer.example", "write_scope": "events.write",
            "verifier": str(self.stub("echo allow\n", "verifier")), "subject": "another-subject"}))
        server = SE.Server(homes, self.principal, SE.CallbackPolicy(), coding_job=bridge)
        auth = SE.HttpAuth(str(auth_config))
        with self.assertRaises(ValueError):  # authenticated subject differs from the pinned subject
            SE.make_http_handler(server, "/mcp", auth, coding_job_only=True)
        with self.assertRaises(ValueError):  # no auth
            SE.make_http_handler(server, "/mcp", None, coding_job_only=True)
        with self.assertRaises(ValueError):  # not job-only
            SE.make_http_handler(server, "/mcp", auth)


class Arguments(Base):
    def test_extra_missing_and_unbound_arguments_never_start_the_command(self):
        marker = self.root / "ran"
        server, _ = self.server(self.stub(f"touch {marker}\nexit 1\n"))
        good = self.args()
        cases = [good | {"principal": "x"}, good | {"binary": "/bin/sh"}, good | {"profile": "/p"},
                 good | {"job": {"schema": "x"}}, good | {"credentials_file": "/f"}, good | {"endpoint": "127.0.0.1:1"},
                 {k: v for k, v in good.items() if k != "job_credential"},
                 good | {"job_sha256": 5}, good | {"job_credential": "x" * 257},
                 good | {"job_credential": "with space " * 4}, good | {"authorization_id": "other-auth"}]
        for arguments in cases:
            result = self.call(server, "submit_coding_job", arguments)
            self.assertTrue(result["isError"], arguments)
        self.assertTrue(self.call(server, "read_coding_job_operation", good)["isError"])  # submit-only fields
        self.assertFalse(marker.exists())


@unittest.skipUnless(COMMAND, "set CODING_JOB_COMMAND_BINARY to a work-engine-command built from this source")
class RealCommand(Base):
    def setUp(self):
        super().setUp()
        self.server_, self.bridge = self.server(COMMAND)

    def run_tool(self, name, **over):
        return self.call(self.server_, name, self.args(name) | over)

    def test_exact_native_input_and_result_projection(self):
        result = self.run_tool("submit_coding_job")
        self.assertNotIn("isError", result)
        body = result["structuredContent"]
        self.assertEqual(body["disposition"], "native_result")
        self.assertEqual(body["native_envelope"]["owner_result"], operation(submit=True))
        self.assertEqual(body["client_process"]["exit_code"], 0)
        self.assertIn("Process exit is client process closure only", body["limitation"])
        self.assertEqual(len(self.fake.requests), 1)
        sent = self.fake.requests[0]
        self.assertEqual(sent["path"], "/v1/session-queries")
        self.assertEqual(sent["authorization"], "Bearer " + SERVICE_TOKEN)  # transport only
        self.assertEqual(sent["body"]["operation"], "submit_coding_job")
        self.assertEqual(sent["body"]["arguments"], self.args())  # exact job credential and hashes, nothing added
        self.assertNotIn(JOB_CREDENTIAL, json.dumps(body))

    def test_repeat_read_result_and_cancel_projections_keep_nulls_and_unknowns(self):
        first = self.run_tool("submit_coding_job")["structuredContent"]["native_envelope"]["owner_result"]
        again = self.run_tool("submit_coding_job")["structuredContent"]["native_envelope"]["owner_result"]
        self.assertEqual((first["entry"], again["entry"]), ("collector_started", "existing_operation"))
        self.assertEqual(first["operation_id"], again["operation_id"])
        read = self.run_tool("read_coding_job_operation")["structuredContent"]["native_envelope"]["owner_result"]
        self.assertIsNone(read["host_input_digest"])
        self.assertEqual(read["process_cleanup"]["run"], "not_established")
        pending = self.run_tool("read_coding_job_result")["structuredContent"]["native_envelope"]["owner_result"]
        self.assertEqual(pending["result"], "not_available")
        self.fake.state = "collected"
        done = self.run_tool("read_coding_job_result")["structuredContent"]["native_envelope"]["owner_result"]
        self.assertEqual(done, collected_result())
        self.assertEqual(done["validation"]["coding_completed"], "not_established")
        self.assertIsNone(done["receipt"]["usage"])
        self.assertEqual(done["acceptance"], "owner_pending")
        self.assertEqual(done["domain_effect_settlement"], "not_claimed")
        cancel = self.run_tool("cancel_coding_job")["structuredContent"]["native_envelope"]["owner_result"]
        self.assertIs(cancel["cancellation_established"], False)
        self.assertIs(cancel["container_closure_established"], False)
        self.assertEqual([r["body"]["operation"] for r in self.fake.requests],
                         ["submit_coding_job"] * 2 + ["read_coding_job_operation", "read_coding_job_result",
                                                      "read_coding_job_result", "cancel_coding_job"])

    def test_native_owner_error_is_passed_through_not_reinterpreted(self):
        self.run_tool("submit_coding_job")
        result = self.run_tool("submit_coding_job", request_id=OTHER_REQUEST)
        self.assertTrue(result["isError"])
        error = self.payload(result)["error"]
        self.assertEqual(error["disposition"], "native_owner_error")
        self.assertEqual(error["native_envelope"]["owner_error"], {"code": "authorization_consumed"})
        self.assertEqual(error["entry_uncertain"], "not_established")  # neither entered nor not entered
        cross = self.run_tool("read_coding_job_operation", request_id=OTHER_REQUEST)
        self.assertEqual(self.payload(cross)["error"]["native_envelope"]["owner_error"], {"code": "identity_conflict"})

    def test_invalid_backend_shapes_are_unknown_and_never_resubmitted(self):
        for mode in ("null_operation_id", "extra_field", "other_identity", "drop"):
            with self.subTest(mode):
                self.fake.mode, self.fake.requests[:] = mode, []
                result = self.run_tool("submit_coding_job")
                self.assertTrue(result["isError"])
                error = self.payload(result)["error"]
                self.assertEqual(error["reason"], "coding_job_outcome_unknown")
                self.assertEqual(error["disposition"], "unknown_no_retry")
                self.assertEqual(error["request_id"], REQUEST)
                self.assertEqual(len(self.fake.requests), 1, "exactly one native invocation, no retry")

    def test_timeout_surfaces_uncertainty_then_original_operation_is_readable(self):
        self.fake.mode = "hang"
        result = self.run_tool("submit_coding_job")
        error = self.payload(result)["error"]
        self.assertEqual(error["disposition"], "unknown_no_retry")
        self.assertIn("transport_timeout", error["cause"])
        self.assertEqual(error["client_process"]["status"], "collected")
        self.assertEqual(len([r for r in self.fake.requests if r["body"]["operation"] == "submit_coding_job"]), 1)
        self.fake.mode = "normal"
        self.fake.bound = REQUEST  # the backend did receive the original submit
        read = self.run_tool("read_coding_job_operation")["structuredContent"]["native_envelope"]["owner_result"]
        self.assertEqual(read["request_id"], REQUEST)
        self.assertEqual(len([r for r in self.fake.requests if r["body"]["operation"] == "submit_coding_job"]), 1)

    def test_profile_swap_after_pinning_is_refused_before_execution(self):
        before = len(self.fake.requests)
        profile = Path(self.bridge.settings["command_profile"])
        profile.write_text(profile.read_text() + " ")
        result = self.run_tool("submit_coding_job")
        self.assertTrue(result["isError"])
        self.assertEqual(self.payload(result)["error"]["reason"], "coding_job_unavailable")
        self.assertEqual(len(self.fake.requests), before)

    def test_profile_replacement_after_verification_cannot_redirect_the_cli(self):
        profile = Path(self.bridge.settings["command_profile"])
        verified_read = self.bridge.read_profile

        def replace_after_verification():
            data = verified_read()
            replacement = json.loads(data)
            replacement["endpoint"] = "127.0.0.1:1"
            private(profile, json.dumps(replacement))
            return data

        with mock.patch.object(self.bridge, "read_profile", side_effect=replace_after_verification):
            result = self.run_tool("submit_coding_job")
        self.assertNotIn("isError", result)
        self.assertEqual(len(self.fake.requests), 1)
        self.assertEqual(self.fake.requests[0]["body"]["arguments"], self.args())


class StubProtocol(Base):
    """Bridge-level failure handling with a stub binary (no Rust, no network)."""

    def test_stdout_eof_allows_a_natural_exit_before_the_deadline(self):
        response = json.dumps({"schema": SE.CodingJobBridge.native_schema, "ok": True, "envelope": {}})
        server, _ = self.server(self.stub(
            f"cat >/dev/null\nprintf '%s\\n' '{response}'\nexec 1>&-\nsleep 0.2\nexit 0\n"))
        result = self.call(server, "submit_coding_job", self.args())
        self.assertNotIn("isError", result)
        self.assertEqual(result["structuredContent"]["client_process"]["exit_code"], 0)
        self.assertIs(result["structuredContent"]["client_process"]["killed_by_bridge"], False)

    def test_hanging_client_is_killed_and_reported_unknown(self):
        server, bridge = self.server(self.stub("cat >/dev/null\nsleep 30\n"), timeout_seconds=1)
        bridge.grace_seconds = 0
        started = time.monotonic()
        error = self.payload(self.call(server, "submit_coding_job", self.args()))["error"]
        self.assertLess(time.monotonic() - started, 10)
        self.assertEqual(error["cause"], "client_timeout")
        self.assertIs(error["client_process"]["killed_by_bridge"], True)
        self.assertEqual(error["client_process"]["status"], "collected")
        self.assertIn("client process closure only", error["limitation"])

    def test_oversize_and_malformed_output_are_unknown(self):
        server, bridge = self.server(self.stub("cat >/dev/null\nyes x | head -c 5000\n"))
        bridge.stdout_limit = 1000
        self.assertEqual(self.payload(self.call(server, "read_coding_job_result", self.args("read_coding_job_result")))
                         ["error"]["cause"], "output_too_large")
        server, _ = self.server(self.stub("cat >/dev/null\necho '{\"schema\":\"x\"}'\n", "other"), name="other")
        self.assertEqual(self.payload(self.call(server, "read_coding_job_result", self.args("read_coding_job_result")))
                         ["error"]["cause"], "invalid_client_output")

    def test_stdin_is_exactly_the_fixed_operation_arguments(self):
        record = self.root / "stdin.json"
        argv = self.root / "argv.txt"
        profile_copy = self.root / "consumed-profile.json"
        server, bridge = self.server(self.stub(f'printf "%s\\n" "$@" > {argv}\ncat "$1" > {profile_copy}\ncat > {record}\nexit 1\n'))
        self.call(server, "cancel_coding_job", self.args("cancel_coding_job"))
        self.assertEqual(json.loads(record.read_text()), self.args("cancel_coding_job"))
        self.assertEqual(argv.read_text().split("\n")[1:5], ["cancel_coding_job", "--json", "-", ""])
        consumed = Path(argv.read_text().split("\n")[0])
        self.assertNotEqual(str(consumed), bridge.settings["command_profile"])
        self.assertEqual(profile_copy.read_bytes(), Path(bridge.settings["command_profile"]).read_bytes())
        self.assertFalse(consumed.exists(), "temporary profile is removed after client collection")


@unittest.skipUnless(COMMAND, "set CODING_JOB_COMMAND_BINARY to a work-engine-command built from this source")
class HttpSurface(Base):
    def test_job_only_http_requires_auth_and_lists_only_job_tools(self):
        server, _ = self.server(COMMAND)
        verifier = self.stub('cat >/dev/null\nif [ -f "$0.deny" ]; then echo deny; exit 1; fi\necho allow\n', "verifier")
        auth_config = private(self.root / "auth.json", json.dumps({
            "resource": "https://mcp.example/mcp", "issuer": "https://issuer.example", "write_scope": "events.write",
            "verifier": str(verifier), "subject": self.subject}))
        handler = SE.make_http_handler(server, "/mcp", SE.HttpAuth(str(auth_config)), coding_job_only=True)
        httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        self.addCleanup(httpd.server_close)
        self.addCleanup(httpd.shutdown)

        def post(message, token="t"):
            connection = http.client.HTTPConnection("127.0.0.1", httpd.server_address[1])
            headers = {"Content-Type": "application/json"} | ({"Authorization": "Bearer " + token} if token else {})
            connection.request("POST", "/mcp", json.dumps(message), headers)
            response = connection.getresponse()
            try:
                return response.status, response.read()
            finally:
                connection.close()
        listing = {"jsonrpc": "2.0", "id": 1, "method": "tools/list"}
        self.assertEqual(post(listing, None)[0], 401)
        status, body = post(listing)
        self.assertEqual(status, 200)
        self.assertEqual({t["name"] for t in json.loads(body)["result"]["tools"]}, set(SE.CODING_JOB_OPERATIONS))
        other = {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "list_sessions", "arguments": {}}}
        self.assertEqual(post(other)[0], 403)
        call = {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": {"name": "submit_coding_job", "arguments": self.args()}}
        self.assertEqual(post(call, None)[0], 401)
        self.assertEqual(len(self.fake.requests), 0)
        status, body = post(call)
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(body)["result"]["structuredContent"]["native_envelope"]["owner_result"]["state"],
                         "collecting")


if __name__ == "__main__":
    unittest.main()
