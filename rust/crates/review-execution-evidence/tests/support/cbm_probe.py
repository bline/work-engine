"""Provider-disabled hostile RP0 peer; run only inside the candidate sandbox."""

import json
import os
import pathlib
import subprocess
import sys
import time


def exchange(child, method, params=None, request_id=1):
    value = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        value["params"] = params
    child.stdin.write(json.dumps(value) + "\n")
    child.stdin.flush()
    line = child.stdout.readline()
    if not line:
        raise RuntimeError("CBM closed before response: " + child.stderr.read()[:1000])
    return json.loads(line)


def unix_and_internet_fds(pid):
    sockets = set()
    for descriptor in pathlib.Path(f"/proc/{pid}/fd").iterdir():
        try:
            target = os.readlink(descriptor)
        except OSError:
            continue
        if target.startswith("socket:["):
            sockets.add(target[8:-1])
    internet = set()
    for name in ("tcp", "tcp6", "udp", "udp6"):
        path = pathlib.Path("/proc/net") / name
        try:
            rows = path.read_text().splitlines()[1:]
        except OSError:
            continue
        for row in rows:
            fields = row.split()
            if len(fields) > 9:
                internet.add(fields[9])
    return len(sockets), len(sockets & internet)


def daemon_processes():
    found = []
    for entry in pathlib.Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            exe = os.readlink(entry / "exe")
            status = (entry / "status").read_text()
        except OSError:
            continue
        if exe == "/cbm/bin":
            attributes = dict(
                line.split(":", 1) for line in status.splitlines() if ":" in line
            )
            count, internet = unix_and_internet_fds(entry.name)
            found.append(
                {
                    "pid": int(entry.name),
                    "no_new_privs": attributes.get("NoNewPrivs", "").strip(),
                    "seccomp": attributes.get("Seccomp", "").strip(),
                    "socket_fd_count": count,
                    "internet_socket_fd_count": internet,
                }
            )
    return found


def main():
    candidate = pathlib.Path("/repo/candidate/lib.rs")
    original = candidate.read_bytes()
    try:
        candidate.write_bytes(b"mutated\n")
        write_refused = False
    except OSError:
        write_refused = True
    assert candidate.read_bytes() == original
    assert write_refused
    assert not pathlib.Path("/owner-canary").exists()
    assert not pathlib.Path("/home/bline/code/work-engine").exists()
    assert not pathlib.Path("/host-credential").exists()
    assert pathlib.Path("/proc/1/root/owner-canary").exists() is False
    assert not pathlib.Path("/cache").exists()
    assert not pathlib.Path("/runtime").exists()
    assert not pathlib.Path("/cbm/bin").exists()
    assert not pathlib.Path("/host-credential").exists()
    assert "RP0_SECRET_CANARY" not in os.environ
    mount_readonly = {
        name: bool(os.statvfs(name).f_flag & os.ST_RDONLY)
        for name in ("/repo/candidate", "/cbm/socket", "/cbm/client",
                     "/session/mcp.json", "/scratch")
    }
    assert all(mount_readonly[name] for name in
               ("/repo/candidate", "/cbm/socket", "/cbm/client", "/session/mcp.json"))
    assert mount_readonly["/scratch"] is False
    inherited = []
    for descriptor in pathlib.Path("/proc/self/fd").iterdir():
        if int(descriptor.name) <= 2:
            continue
        try:
            inherited.append(os.readlink(descriptor))
        except OSError:
            continue
    assert not any("/home/bline" in target or "owner-canary" in target
                   for target in inherited), inherited
    mcp_config = pathlib.Path("/session/mcp.json")
    original_config = mcp_config.read_bytes()
    try:
        mcp_config.write_text('{"mcpServers":{}}')
        mcp_config_write_refused = False
    except OSError:
        mcp_config_write_refused = True
    assert mcp_config_write_refused and mcp_config.read_bytes() == original_config
    try:
        subprocess.run(["/cbm/bin", "cli", "index_repository"], check=False)
        direct_index_refused = False
    except FileNotFoundError:
        direct_index_refused = True
    assert direct_index_refused

    child = subprocess.Popen(
        ["/cbm/client"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True, bufsize=1, close_fds=True,
        env={"HOME": "/", "PATH": "/usr/bin:/bin"},
    )
    try:
        hello = exchange(child, "initialize", {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "rp0-hostile-peer", "version": "1"},
        })
        assert "result" in hello, hello
        child.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        child.stdin.flush()
        listed = exchange(child, "tools/list", request_id=2)
        names = {entry["name"] for entry in listed["result"]["tools"]}
        assert names == {"search_graph", "trace_path", "get_code_snippet",
                         "check_index_coverage", "index_status", "query_graph",
                         "get_architecture"}, names
        status = exchange(child, "tools/call", {"name": "index_status", "arguments": {
            "project": "rp0_fixture", "diagnostics": "none", "format": "json"}}, 3)
        assert "result" in status, status
        assert "/repo/candidate" in json.dumps(status), status
        for request_id in range(100, 180):
            repeated = exchange(child, "tools/list", request_id=request_id)
            assert len(repeated["result"]["tools"]) == 7
        projects = exchange(child, "tools/call", {"name": "list_projects", "arguments": {}}, 6)
        assert "unavailable" in json.dumps(projects).lower(), projects
        other = exchange(child, "tools/call", {"name": "index_status", "arguments": {
            "project": "home-bline-code-work-engine", "diagnostics": "none"}}, 4)
        assert "unavailable" in json.dumps(other).lower(), other
        hostile_query = exchange(child, "tools/call", {"name": "query_graph", "arguments": {
            "project": "home-bline-code-work-engine", "query": "MATCH (n) RETURN n LIMIT 1"}}, 7)
        assert "unavailable" in json.dumps(hostile_query).lower(), hostile_query
        admitted_hostile_query = exchange(child, "tools/call", {"name": "query_graph", "arguments": {
            "project": "rp0_fixture", "query": "CREATE (n:File {path:'/owner-canary'}) RETURN n"}}, 8)
        assert admitted_hostile_query["result"]["isError"] is True, admitted_hostile_query
        assert "write operations not supported" in json.dumps(admitted_hostile_query), admitted_hostile_query
        hostile_paths = exchange(child, "tools/call", {"name": "check_index_coverage", "arguments": {
            "project": "rp0_fixture", "paths": ["../owner-canary", "/home/bline/code/work-engine/AGENTS.md"],
            "path_limit": 2, "diagnostics": "none", "format": "json"}}, 9)
        records = hostile_paths["result"]["structuredContent"]["paths"]
        assert len(records) == 2 and all(row["status"] == "outside_project" for row in records), hostile_paths
        assert "Repository Instructions" not in json.dumps(hostile_paths), hostile_paths
        hostile_source = exchange(child, "tools/call", {"name": "get_code_snippet", "arguments": {
            "project": "rp0_fixture", "qualified_name": "/home/bline/code/work-engine/AGENTS.md"}}, 10)
        assert hostile_source["result"]["isError"] is True, hostile_source
        assert "Repository Instructions" not in json.dumps(hostile_source), hostile_source
        mutation = exchange(child, "tools/call", {"name": "index_repository", "arguments": {
            "repo_path": "/repo/candidate", "name": "rp0_fixture"}}, 5)
        assert "unavailable" in json.dumps(mutation).lower() or "not allowed" in json.dumps(mutation).lower(), mutation
        first = daemon_processes()
        time.sleep(3)
        later = daemon_processes()
        assert not first and not later, (first, later)
        print(json.dumps({"candidate_write_refused": write_refused,
                          "cache_unmounted": True,
                          "direct_index_refused": direct_index_refused,
                          "mcp_config_write_refused": mcp_config_write_refused,
                          "hostile_query_refused": True,
                          "admitted_hostile_query_response": admitted_hostile_query,
                          "hostile_paths_response": hostile_paths,
                          "hostile_source_response": hostile_source,
                          "mount_readonly": mount_readonly,
                          "inherited_fd_targets": inherited,
                          "tools": sorted(names), "daemon_initial": first,
                          "daemon_after_reload_interval": later,
                          "other_project_refused": True, "mutation_refused": True}))
    finally:
        child.terminate()
        try:
            child.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.communicate(timeout=5)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"RP0 hostile peer failed: {error!r}", file=sys.stderr)
        raise
