#!/usr/bin/env python3
"""Host side of a containerized Pi turn: mount plan, container command, egress proxy, runner.

role_host.py builds a turn spec and starts `pi_container.py run-turn SPEC` in
its own process group, writing the provider API key to its stdin. The runner:

  1. serves the egress proxy on a Unix socket mounted into the container;
  2. starts the container detached (`docker run -d`), so the Docker client is
     never in the path of Pi's output;
  3. waits for it, records `docker inspect` state, removes it, stops the proxy.

SIGTERM or SIGINT to the runner stops the container (`docker stop -t 10`).

The container runs the host's own /usr and /etc read-only, so tools, versions,
users and paths match the host. It has no network: its only route out is the
proxy, which accepts POST /v1/responses for the pinned model only, refuses
hosted (server-side) tools, sets or removes service_tier, substitutes the real
key for the container's placeholder, and logs the tier and usage the provider
reported. The key never enters the container.

Host facts: metrics/probes/pi-docker-host-20261109.json.
"""
from __future__ import annotations

import hashlib
import http.client
import json
import os
import platform
import re
import shutil
import signal
import socketserver
import subprocess
import sys
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler
from pathlib import Path

HERE = Path(__file__).resolve().parent
# Digest-pinned Node image shared with rust/crates/work-engine-manual-host
# (pi_docker::IMAGE). Only its skeleton is used: /usr and /etc come from the host.
IMAGE = "node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da"
DEFAULT_RUNTIME = "/snap/pi-coding-agent/current"
PLACEHOLDER_KEY = "sk-role-host-container-placeholder"
RELAY_PORT = 8080
# codebase-memory needs an existing private runtime dir whose socket path fits
# in 108 bytes; a container tmpfs gives it one at a short fixed path.
CBM_RUNTIME = "/run/cbm"
USERLAND = ("/usr", "/etc")
# Never offered as user mounts: system trees, and the userland already mounted.
SYSTEM_PREFIXES = ("/usr", "/etc", "/proc", "/sys", "/dev", "/run", "/snap", "/boot", "/var/lib/docker")
ALLOWED_TOOL_TYPES = frozenset({"function", "custom", "namespace"})
# Top-level Responses fields the proxy forwards. Anything else (stored prompts,
# conversations, previous responses, background mode, future fields) is refused
# rather than forwarded unexamined. Pi 1.4.0 sends a subset of these.
ALLOWED_FIELDS = frozenset({
    "model", "input", "instructions", "stream", "stream_options", "store", "include", "reasoning",
    "service_tier", "max_output_tokens", "max_tool_calls", "prompt_cache_key", "prompt_cache_retention",
    "text", "tool_choice", "tools", "parallel_tool_calls", "temperature", "top_p", "top_logprobs",
    "truncation", "metadata", "safety_identifier", "user"})
TOOL_CHOICE_STRINGS = frozenset({"auto", "none", "required"})
TOOL_CHOICE_TYPES = frozenset({"function", "custom", "allowed_tools"})
# Proxy resource bounds: the proxy runs on the host, outside the container's limits.
MAX_CONNECTIONS = 8
MAX_BODY_BYTES = 64 * 1024 * 1024
SOCKET_TIMEOUT = 60
TERMINAL_EVENTS = frozenset({"response.completed", "response.incomplete", "response.failed"})
MEMORY = re.compile(r"^[1-9][0-9]*[kmg]?$")


class ContainerError(Exception):
    def __init__(self, status: str, detail: str):
        super().__init__(detail)
        self.status, self.detail = status, detail


# ------------------------------------------------------------- planning ----

def runtime_paths(root: str = DEFAULT_RUNTIME) -> dict:
    """Pi runtime from a snap-shaped tree: bundled node plus the CLI entry point."""
    resolved = os.path.realpath(root)
    node = os.path.join(root, "node/bin/node")
    cli = os.path.join(root, "lib/pi/packages/coding-agent/dist/cli.js")
    package = os.path.join(resolved, "lib/pi/packages/coding-agent/package.json")
    for path in (os.path.join(resolved, "node/bin/node"), os.path.join(resolved, "lib/pi/packages/coding-agent/dist/cli.js"), package):
        if not os.path.isfile(path):
            raise ContainerError("runtime_not_found", f"Pi runtime file missing: {path}")
    with open(package) as fh:
        version = json.load(fh).get("version")
    return {"root": root, "resolved": resolved, "node": node, "cli": cli, "pi_version": version}


def mount_plan(cwd: str, ro=(), rw=(), *, home: str, reserved=()) -> list[list[str]]:
    """Same-path bind mounts: [[path, "ro"|"rw"], ...]. The working directory is
    mounted read-write unless a listed mount already covers it."""
    entries: dict[str, str] = {}
    home = os.path.realpath(home)

    def add(raw: str, mode: str):
        if not os.path.isabs(raw):
            raise ContainerError("usage", f"mount {raw!r} must be an absolute path")
        path = os.path.realpath(raw)
        if not os.path.exists(path):
            raise ContainerError("usage", f"mount {raw!r} does not exist")
        if any(c in path for c in ",=\n\r"):
            raise ContainerError("usage", f"mount {path!r} contains a character Docker's --mount syntax cannot carry")
        if path in ("/", home):
            raise ContainerError("usage", f"mounting {path} would expose everything; mount the folders the task needs")
        if any(path == p or path.startswith(p + "/") for p in SYSTEM_PREFIXES):
            raise ContainerError("usage", f"{path} is a system path; /usr and /etc are already mounted read-only")
        for r in reserved:
            r = os.path.realpath(r)
            if path == r or path.startswith(r + "/") or r.startswith(path + "/"):
                raise ContainerError("usage", f"{path} overlaps role_host state {r}")
        if entries.get(path, mode) != mode:
            raise ContainerError("usage", f"{path} is listed both read-only and read-write")
        entries[path] = mode

    for p in ro:
        add(p, "ro")
    for p in rw:
        add(p, "rw")
    work = os.path.realpath(cwd)
    if not any(work == p or work.startswith(p + "/") for p in entries):
        add(work, "rw")
    return [[p, entries[p]] for p in sorted(entries)]


def validate_limits(memory: str, pids: int, cpus: str) -> dict:
    if not MEMORY.match(str(memory)):
        raise ContainerError("usage", f"container memory {memory!r} must look like 4g, 512m or 1073741824")
    if not isinstance(pids, int) or pids < 16:
        raise ContainerError("usage", f"container pids {pids!r} must be an integer of at least 16")
    try:
        if float(cpus) <= 0:
            raise ValueError
    except ValueError:
        raise ContainerError("usage", f"container cpus {cpus!r} must be a positive number")
    return {"memory": str(memory), "pids": pids, "cpus": str(cpus)}


def file_sha256(path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def tree_digest(root: str) -> str:
    """Digest of every regular file and symlink under root, by relative path."""
    h = hashlib.sha256()
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        for name in sorted(filenames + [d for d in dirnames if os.path.islink(os.path.join(dirpath, d))]):
            path = os.path.join(dirpath, name)
            rel = os.path.relpath(path, root)
            try:
                if os.path.islink(path):
                    h.update(f"L {rel} {os.readlink(path)}\n".encode())
                elif os.path.isfile(path):
                    h.update(f"F {rel} {file_sha256(path)}\n".encode())
            except OSError as e:
                h.update(f"E {rel} {e.errno}\n".encode())
    return h.hexdigest()


def metadata_digest(root: str) -> str:
    """Digest of path, size, mode and mtime of everything under root: catches
    upgrades and replacements without reading large binaries."""
    h = hashlib.sha256()
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        for name in sorted(filenames + dirnames):
            path = os.path.join(dirpath, name)
            try:
                st = os.lstat(path)
            except OSError as e:
                h.update(f"E {path} {e.errno}\n".encode())
                continue
            h.update(f"{os.path.relpath(path, root)} {st.st_mode} {st.st_size} {st.st_mtime_ns}\n".encode())
    return h.hexdigest()


def is_squashfs_mount(path: str) -> bool:
    """A snap revision is a read-only squashfs mount: its path identifies its bytes."""
    try:
        for line in Path("/proc/self/mountinfo").read_text().splitlines():
            left, _, right = line.partition(" - ")
            fields = left.split()
            if len(fields) > 4 and fields[4] == path and right.split()[:1] == ["squashfs"]:
                return True
    except OSError:
        pass
    return False


def runtime_identity(resolved: str) -> dict:
    if is_squashfs_mount(resolved):
        return {"kind": "squashfs mount (immutable)", "path": resolved}
    return {"kind": "content digest", "sha256": tree_digest(resolved)}


def userland_identity() -> dict:
    """What the read-only host /usr and /etc give the agent, as far as it can be
    read: the package database (versions of everything apt installed) and the
    metadata of /usr/local. Hand edits under /etc that change no package, and an
    edit under /usr/local that keeps size and mtime, are not covered."""
    out = {}
    try:
        out["dpkg_status_sha256"] = file_sha256("/var/lib/dpkg/status")
    except OSError:
        out["dpkg_status_sha256"] = None
    out["usr_local_metadata_sha256"] = metadata_digest("/usr/local")
    return out


def mount_identities(mounts) -> dict:
    """Device and inode of each mount root: a replaced directory changes them."""
    out = {}
    for path, _mode in mounts:
        try:
            st = os.stat(path)
            out[path] = [st.st_dev, st.st_ino]
        except OSError:
            out[path] = None
    return out


def container_env(env: dict, record_path: str) -> dict:
    """The container's entire environment. Nothing else from the host reaches it."""
    sel = env["selection"]
    return {"HOME": env["home"], "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            "LANG": env["lang"], "USER": env["user"], "LOGNAME": env["user"],
            "PI_CODING_AGENT_DIR": env["agent_dir"], "PI_OFFLINE": "1", "PI_TELEMETRY": "0",
            "OPENAI_API_KEY": PLACEHOLDER_KEY,
            "WORK_ENGINE_PI_SERVICE_TIER": sel["tier"] or "", "WORK_ENGINE_PI_SERVICE_TIER_MODEL": sel["model"],
            "WORK_ENGINE_PI_SERVICE_TIER_RECORD": record_path}


def fingerprint(env: dict) -> dict:
    """The environment's identity: everything that decides what the agent sees or can do."""
    os_release = {}
    try:
        for line in Path("/etc/os-release").read_text().splitlines():
            k, _, v = line.partition("=")
            if k in ("ID", "VERSION_ID"):
                os_release[k] = v.strip('"')
    except OSError:
        pass
    body = {
        "mode": "container", "image": IMAGE, "network": "none (egress proxy only)",
        "userland": list(USERLAND), "userland_identity": userland_identity(),
        "host_os": os_release, "kernel": platform.release(),
        "runtime": env["runtime"]["resolved"], "pi_version": env["runtime"]["pi_version"],
        "runtime_identity": runtime_identity(env["runtime"]["resolved"]),
        "mounts": env["mounts"], "mount_identities": mount_identities(env["mounts"]),
        "limits": env["limits"], "uid_gid": [env["uid"], env["gid"]],
        "selection": env["selection"], "max_requests": env.get("max_requests"),
        "container_env": container_env(env, "<per turn>"),
        "entry_sha256": file_sha256(HERE / "pi_container_entry.py"),
        "extension_sha256": file_sha256(HERE / "pi_service_tier.ts"),
        "pi_container_sha256": file_sha256(HERE / "pi_container.py"),
    }
    if env.get("codebase_memory"):
        cbm = env["codebase_memory"]
        body["codebase_memory"] = {"binary_sha256": file_sha256(cbm["binary"]), "version": cbm["version"],
                                   "project": cbm["project"], "root_path": cbm["root_path"]}
    canonical = json.dumps(body, sort_keys=True, separators=(",", ":")).encode()
    return {**body, "sha256": hashlib.sha256(canonical).hexdigest()}


def run_profile(env: dict, *, cwd: str, container_dir: str, socket_dir: str | None = None) -> list[str]:
    """Hardened flags and same-path mounts shared by every container this module starts."""
    uid, gid, home = env["uid"], env["gid"], env["home"]
    runtime = env["runtime"]
    argv = ["--network", "none", "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
            "--user", f"{uid}:{gid}", "--pids-limit", str(env["limits"]["pids"]),
            "--memory", env["limits"]["memory"], "--cpus", env["limits"]["cpus"],
            "--tmpfs", "/tmp:rw,nosuid,size=1g",
            "--tmpfs", f"{home}:rw,nosuid,size=1g,uid={uid},gid={gid},mode=0700",
            "--workdir", cwd]
    mounts = [(p, p, True) for p in USERLAND]
    mounts += [(runtime["resolved"], runtime["root"], True),
               (str(HERE / "pi_container_entry.py"), str(HERE / "pi_container_entry.py"), True),
               (str(HERE / "pi_service_tier.ts"), str(HERE / "pi_service_tier.ts"), True),
               (container_dir, container_dir, False)]
    if socket_dir:
        mounts.append((socket_dir, socket_dir, False))
    if env.get("codebase_memory"):
        binary = env["codebase_memory"]["binary"]
        mounts.append((binary, binary, True))
        argv += ["--tmpfs", f"{CBM_RUNTIME}:rw,nosuid,size=16m,uid={uid},gid={gid},mode=0700"]
    mounts += [(p, p, mode == "ro") for p, mode in env["mounts"]]
    for src, dst, readonly in mounts:
        argv += ["--mount", f"type=bind,src={src},dst={dst}" + (",readonly" if readonly else "")]
    return argv


def docker_argv(spec: dict) -> list[str]:
    """`docker run` arguments for one turn. Contains no credential."""
    env, files = spec["environment"], spec["files"]
    argv = ["run", "-d", "--pull", "never", "--name", spec["container_name"],
            "--label", "work-engine.role-host=pi", "--label", f"work-engine.session={spec['session_id']}",
            "--label", f"work-engine.turn={spec['turn']}"]
    argv += run_profile(env, cwd=spec["cwd"], container_dir=spec["container_dir"], socket_dir=spec["socket_dir"])
    argv += ["--entrypoint", "/usr/bin/python3"]
    for key, value in spec["container_env"].items():
        argv += ["--env", f"{key}={value}"]
    runtime = env["runtime"]
    argv += [IMAGE, str(HERE / "pi_container_entry.py"), spec["socket_path"],
             files["prompt"], files["events"], files["stderr"], files["exit"], "--",
             runtime["node"], runtime["cli"], *spec["pi_argv"]]
    return argv


# ------------------------------------------------------ codebase memory ----

def default_codebase_memory() -> str:
    return os.environ.get("ROLE_HOST_CBM_EXE") or os.path.expanduser("~/.local/bin/codebase-memory-mcp")


def index_codebase_memory(binary: str, repo: str, *, cache: str, runtime: str, env: dict) -> dict:
    """Index exactly `repo` into a cache of its own, on the host, before launch."""
    r = subprocess.run([binary, "cli", "--quiet", "index_repository", "--repo-path", repo, "--index-worktree", "true"],
                       capture_output=True, text=True, timeout=3600, stdin=subprocess.DEVNULL,
                       env={**env, "CBM_CACHE_DIR": cache, "CBM_RUNTIME_DIR": runtime})
    result = None
    for line in reversed(r.stdout.strip().splitlines()):
        try:
            result = json.loads(line)
            break
        except ValueError:
            continue
    if r.returncode != 0 or not isinstance(result, dict) or result.get("status") != "indexed":
        raise ContainerError("codebase_memory_not_established",
                             f"index_repository exit {r.returncode}: {(r.stderr or r.stdout).strip()[-300:]}")
    return result


def check_codebase_memory(docker: str, env: dict, *, cwd: str, container_dir: str) -> dict:
    """Run index_status inside the turn's own container profile; no model, no network."""
    cbm = env["codebase_memory"]
    argv = [docker, "run", "--rm", "--pull", "never", "--label", "work-engine.role-host=pi-check",
            *run_profile(env, cwd=cwd, container_dir=container_dir),
            "--entrypoint", cbm["binary"], "--env", f"HOME={env['home']}",
            "--env", f"CBM_CACHE_DIR={cbm['cache_dir']}", "--env", f"CBM_RUNTIME_DIR={cbm['runtime_dir']}",
            IMAGE, "cli", "index_status", "--project", cbm["project"]]
    r = subprocess.run(argv, capture_output=True, text=True, timeout=300, stdin=subprocess.DEVNULL)
    text = r.stdout
    fields = dict(line.split(": ", 1) for line in text.splitlines() if ": " in line and not line.startswith(" "))
    if r.returncode != 0 or fields.get("status") != "ready" or fields.get("root_path") != cwd:
        raise ContainerError("codebase_memory_not_established",
                             f"in-container index_status exit {r.returncode}: status={fields.get('status')!r} "
                             f"root_path={fields.get('root_path')!r} (expected ready, {cwd}) {r.stderr.strip()[-200:]}")
    return {"status": fields.get("status"), "root_path": fields.get("root_path"), "nodes": fields.get("nodes"),
            "indexed_at": fields.get("indexed_at")}


# ---------------------------------------------------------------- proxy ----

def declared_tool_types(value) -> set:
    """Types of every tool declared in any `tools` list anywhere in the request,
    through namespaces, tool_search_output items and allowed_tools choices."""
    found = set()
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "tools" and isinstance(item, list):
                for tool in item:
                    found.add(tool.get("type") if isinstance(tool, dict) else type(tool).__name__)
            found |= declared_tool_types(item)
    elif isinstance(value, list):
        for item in value:
            found |= declared_tool_types(item)
    return found


def check_request(body, *, model_id: str) -> str | None:
    """A refusal reason, or None when the request may go upstream."""
    if not isinstance(body, dict):
        return "body_not_object"
    unexpected = sorted(set(body) - ALLOWED_FIELDS)
    if unexpected:
        return "unexpected_field:" + ",".join(unexpected)
    if body.get("model") != model_id:
        return "model_not_pinned"
    if body.get("store") is True:
        return "store_refused"
    choice = body.get("tool_choice")
    if choice is not None and not (
            (isinstance(choice, str) and choice in TOOL_CHOICE_STRINGS)
            or (isinstance(choice, dict) and choice.get("type") in TOOL_CHOICE_TYPES)):
        return "tool_choice_refused"
    hosted = declared_tool_types(body) - ALLOWED_TOOL_TYPES
    if hosted:
        return "hosted_tool_refused:" + ",".join(sorted(str(t) for t in hosted))
    return None


class EgressProxy:
    """HTTP/1.1 on a Unix socket; one upstream request per accepted request."""

    def __init__(self, socket_path: str, *, upstream: str, key: str, model_id: str,
                 tier: str | None, log_path: str, max_requests: int | None = None,
                 max_connections: int = MAX_CONNECTIONS, max_body: int = MAX_BODY_BYTES,
                 socket_timeout: float = SOCKET_TIMEOUT):
        self.socket_path, self.key, self.model_id, self.tier = socket_path, key, model_id, tier
        self.upstream = urllib.parse.urlsplit(upstream)
        self.log_path, self.max_requests = log_path, max_requests
        self.max_connections, self.max_body, self.socket_timeout = max_connections, max_body, socket_timeout
        self.accepted = 0
        self.dropped_connections = 0
        self.lock = threading.Lock()
        self.server = None

    def log(self, row: dict):
        with self.lock, open(self.log_path, "a") as fh:
            fh.write(json.dumps({"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), **row}) + "\n")

    def start(self):
        proxy = self

        class Handler(BaseHTTPRequestHandler):
            # HTTP/1.0 framing: the streamed body ends when the connection closes.
            timeout = proxy.socket_timeout  # bounds slow headers and bodies

            def log_message(self, *args):
                pass

            def refuse(self, code: int, reason: str):
                proxy.log({"event": "refused", "method": self.command, "path": self.path, "http_status": code, "reason": reason})
                payload = json.dumps({"error": {"message": f"role_host egress proxy: {reason}", "type": "egress_refused"}}).encode()
                self.send_response(code)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(payload)))
                self.send_header("connection", "close")
                self.end_headers()
                self.wfile.write(payload)
                self.close_connection = True

            def do_GET(self):
                self.refuse(403, "only POST /v1/responses is allowed")

            do_PUT = do_DELETE = do_PATCH = do_GET

            def do_POST(self):
                if self.path.split("?", 1)[0] != "/v1/responses":
                    return self.refuse(403, "only POST /v1/responses is allowed")
                if self.headers.get("transfer-encoding"):
                    return self.refuse(411, "chunked or encoded request bodies are not accepted")
                length = self.headers.get("content-length", "")
                if not length.isdigit() or not 0 < int(length) <= proxy.max_body:
                    return self.refuse(413 if length.isdigit() else 411,
                                       f"content-length must be between 1 and {proxy.max_body}")
                # Reserve a slot under the cap before any body is buffered; a
                # request refused below gives its slot back.
                with proxy.lock:
                    if proxy.max_requests is not None and proxy.accepted >= proxy.max_requests:
                        n = None
                    else:
                        proxy.accepted += 1
                        n = proxy.accepted
                if n is None:
                    return self.refuse(429, f"request cap {proxy.max_requests} reached")

                def release():
                    with proxy.lock:
                        proxy.accepted -= 1

                try:
                    raw = self.rfile.read(int(length))
                except OSError:
                    release()
                    return proxy.log({"event": "refused", "reason": "body not received in time"})
                if len(raw) != int(length):
                    release()
                    return self.refuse(400, "body shorter than content-length")
                try:
                    body = json.loads(raw)
                except ValueError:
                    release()
                    return self.refuse(400, "body is not JSON")
                reason = check_request(body, model_id=proxy.model_id)
                if reason:
                    release()
                    return self.refuse(403, reason)
                if proxy.tier:
                    body["service_tier"] = proxy.tier
                else:
                    body.pop("service_tier", None)
                data = json.dumps(body).encode()
                headers = {k: v for k, v in self.headers.items()
                           if k.lower() not in ("host", "authorization", "content-length", "connection",
                                                "transfer-encoding", "keep-alive")}
                headers.update({"authorization": f"Bearer {proxy.key}", "content-length": str(len(data))})
                up = proxy.upstream
                conn_cls = http.client.HTTPSConnection if up.scheme == "https" else http.client.HTTPConnection
                started = time.time()
                try:
                    conn = conn_cls(up.hostname, up.port, timeout=900)
                    conn.request("POST", up.path.rstrip("/") + "/responses", data, headers)
                    resp = conn.getresponse()
                except OSError as e:
                    proxy.log({"event": "upstream_error", "n": n, "detail": str(e)[:300]})
                    return self.refuse(502, "upstream unreachable")
                self.send_response(resp.status)
                for k, v in resp.getheaders():
                    if k.lower() not in ("transfer-encoding", "connection", "content-length", "keep-alive"):
                        self.send_header(k, v)
                self.send_header("connection", "close")
                self.end_headers()
                terminal, buf, head = None, b"", b""
                try:
                    while True:
                        chunk = resp.read1(65536)
                        if not chunk:
                            break
                        self.wfile.write(chunk)
                        self.wfile.flush()
                        if resp.status != 200 and len(head) < 2000:
                            head += chunk[: 2000 - len(head)]
                        buf += chunk
                        *lines, buf = buf.split(b"\n")
                        for line in lines:
                            if line.startswith(b"data: "):
                                try:
                                    event = json.loads(line[6:])
                                except ValueError:
                                    continue
                                if isinstance(event, dict) and event.get("type") in TERMINAL_EVENTS:
                                    terminal = event
                except OSError as e:
                    proxy.log({"event": "stream_error", "n": n, "detail": str(e)[:300]})
                finally:
                    conn.close()
                response = (terminal or {}).get("response") or {}
                usage = response.get("usage") or {}
                proxy.log({
                    "event": "request", "n": n, "http_status": resp.status, "model": body.get("model"),
                    "requested_tier": body.get("service_tier"), "observed_tier": response.get("service_tier"),
                    "terminal": (terminal or {}).get("type"), "response_id": response.get("id"),
                    "response_model": response.get("model"),
                    "usage": {"input_tokens": usage.get("input_tokens"), "output_tokens": usage.get("output_tokens"),
                              "cached_tokens": (usage.get("input_tokens_details") or {}).get("cached_tokens"),
                              "reasoning_tokens": (usage.get("output_tokens_details") or {}).get("reasoning_tokens")},
                    "seconds": round(time.time() - started, 3),
                    **({"error_head": head.decode("utf-8", "replace")} if resp.status != 200 else {}),
                })
                self.close_connection = True

        slots = threading.BoundedSemaphore(self.max_connections)

        class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
            daemon_threads = True

            def process_request(self, request, client_address):
                # Refuse beyond the connection bound instead of spawning a thread.
                if not slots.acquire(blocking=False):
                    with proxy.lock:
                        proxy.dropped_connections += 1
                    self.shutdown_request(request)
                    return
                super().process_request(request, client_address)

            def process_request_thread(self, request, client_address):
                try:
                    super().process_request_thread(request, client_address)
                finally:
                    slots.release()

        if os.path.exists(self.socket_path):
            os.unlink(self.socket_path)
        self.server = Server(self.socket_path, Handler)
        os.chmod(self.socket_path, 0o600)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        return self

    def stop(self):
        if self.server:
            self.server.shutdown()
            self.server.server_close()


# --------------------------------------------------------------- runner ----

def run_turn(spec_path: str) -> int:
    spec = json.loads(Path(spec_path).read_text())
    key = sys.stdin.readline().strip()
    sys.stdin.close()
    docker = spec["docker"]
    record = spec["files"]["runner_record"]

    def note(event: str, **fields):
        with open(record, "a") as fh:
            fh.write(json.dumps({"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "event": event, **fields}) + "\n")

    if not key:
        note("refused", detail="no provider key on stdin")
        return 64
    os.makedirs(spec["socket_dir"], mode=0o700, exist_ok=True)
    proxy = EgressProxy(spec["socket_path"], upstream=spec["upstream"], key=key, model_id=spec["model_id"],
                        tier=spec.get("service_tier"), log_path=spec["files"]["proxy_log"],
                        max_requests=spec.get("max_requests")).start()
    del key
    note("proxy_listening", socket=spec["socket_path"], upstream=spec["upstream"])
    name = spec["container_name"]
    stopping = threading.Event()

    def stop_container(signum, _frame):
        if not stopping.is_set():
            stopping.set()
            note("stop_requested", signal=signum)
            threading.Thread(target=subprocess.run, args=([docker, "stop", "-t", "10", name],),
                             kwargs={"capture_output": True}, daemon=True).start()

    signal.signal(signal.SIGTERM, stop_container)
    signal.signal(signal.SIGINT, stop_container)
    rc = 0
    try:
        if stopping.is_set():
            note("container_not_started", detail="stop requested before start")
            return 0
        started = subprocess.run([docker, *docker_argv(spec)], capture_output=True, text=True)
        if started.returncode != 0:
            note("container_not_started", exit=started.returncode, stderr=started.stderr.strip()[-800:])
            return 23
        note("container_started", id=started.stdout.strip())
        waited = subprocess.run([docker, "wait", name], capture_output=True, text=True)
        note("container_exited", wait_exit=waited.returncode, status=waited.stdout.strip())
        inspected = subprocess.run([docker, "inspect", "--format", "{{json .State}}", name], capture_output=True, text=True)
        if inspected.returncode == 0:
            try:
                state = json.loads(inspected.stdout)
                note("container_state", **{k: state.get(k) for k in ("ExitCode", "OOMKilled", "Error", "StartedAt", "FinishedAt")})
            except ValueError:
                note("container_state_unreadable")
    finally:
        removed = subprocess.run([docker, "rm", "-f", name], capture_output=True, text=True)
        note("container_removed", exit=removed.returncode)
        proxy.stop()
        shutil.rmtree(spec["socket_dir"], ignore_errors=True)
    return rc


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) == 2 and argv[0] == "run-turn":
        return run_turn(argv[1])
    print("usage: pi_container.py run-turn SPEC_JSON  (the provider key is read from stdin)", file=sys.stderr)
    return 64


if __name__ == "__main__":
    sys.exit(main())
