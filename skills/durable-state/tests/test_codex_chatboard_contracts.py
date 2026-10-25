from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from durable_state import DurableStateError, GitRefDurableStateStore  # noqa: E402

SCRIPT = Path(__file__).parents[1] / "scripts/codex_chatboard.py"
REGISTRY = Path(__file__).parents[1] / "contracts/codex-chatboard-kinds.v1.json"
DRAFT = Path(__file__).parents[3] / "docs/codex-chatboard-contracts.md"
OPERATOR_DOC = Path(__file__).parents[3] / "docs/codex-chatboard.md"
SPEC = importlib.util.spec_from_file_location("codex_chatboard_contracts_subject", SCRIPT)
assert SPEC and SPEC.loader
BOARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOARD)

LETTERS = "abcdef01-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
SESSION_1 = "11111111-1111-4111-8111-111111111111"
SESSION_2 = "22222222-2222-4222-8222-222222222222"
HEX = "a" * 64

# Captured by running the unmodified base adapter (9d8b41d8) on exactly the two
# posts made in ByteCompatibilityTest.
BASE_FIXTURE = '''{
  "artifact_type": "work_engine_codex_chatboard_v1",
  "claims": {},
  "messages": [
    {
      "author": "alpha",
      "body": "one",
      "message_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      "observed_at": "2026-08-30T04:00:00Z",
      "references": [
        "a/b"
      ],
      "sequence": 1,
      "session_id": "11111111-1111-4111-8111-111111111111",
      "topic": "campaign"
    },
    {
      "author": "beta",
      "body": "two",
      "message_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
      "observed_at": "2026-08-30T04:00:01Z",
      "references": [],
      "sequence": 2,
      "session_id": "11111111-1111-4111-8111-111111111111",
      "topic": "session:11111111-1111-4111-8111-111111111111",
      "wake": true
    }
  ],
  "next_sequence": 3,
  "schema_version": 1
}
'''


def mid(n: int) -> str:
    return f"{n:08x}-0000-4000-8000-{n:012x}"


class RepoCase(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.repository = Path(self.temporary.name) / "repo"
        self.repository.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repository)], check=True)
        self.counter = 100
        self.registry = BOARD.load_registry()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def post(self, **kw):
        self.counter += 1
        base = dict(author="alpha", session_id=SESSION_1, topic="t", body="b", references=[],
                    message_id=mid(self.counter), observed_at="2026-10-25T00:00:00Z")
        base.update(kw)
        return BOARD.post(self.repository, **base)

    def board(self):
        return BOARD.read_board(self.repository)

    def run_cli(self, *args: str, script: Path = SCRIPT) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, "-B", str(script), "--repository", str(self.repository), *args],
                              capture_output=True, text=True)

    def raw_rewrite(self, mutate_board) -> None:
        store = GitRefDurableStateStore(self.repository, BOARD.NAMESPACE)
        current = store.read(BOARD.BOARD_KEY)
        board = BOARD.decode(current.payload)
        mutate_board(board)
        store.publish(BOARD.BOARD_KEY, BOARD.encode(board), current.revision)

    def full_envelope(self, kind: str) -> dict:
        spec = self.registry["kinds"][kind]
        env: dict = {"kind": kind, "data": {}}
        for path in spec["required"]:
            enum = spec.get("enums", {}).get(path)
            pattern = spec.get("patterns", {}).get(path)
            if path in ("in_reply_to", "supersedes"):
                value = 1
            elif path == "role":
                value = "peer-role"
            elif path == "to":
                value = ["role:peer"]
            elif enum:
                value = enum[0]
            elif pattern:
                value = HEX
            elif path == "data.predecessor_session":
                value = SESSION_2
            else:
                value = "x"
            if path.startswith("data."):
                env["data"][path[5:]] = value
            else:
                env[path] = value
        if not env["data"]:
            del env["data"]
        return env


class ByteCompatibilityTest(RepoCase):
    def run_fixture(self, repository: Path, script: Path) -> str:
        sys.path.insert(0, str(script.parent))
        spec = importlib.util.spec_from_file_location("fixture_adapter", script)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.post(repository, author="alpha", session_id=SESSION_1, topic="campaign", body="one",
                    references=["a/b"], message_id="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                    observed_at="2026-08-30T04:00:00Z")
        module.post(repository, author="beta", session_id=SESSION_1, topic=f"session:{SESSION_1}", body="two",
                    references=[], message_id="bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                    observed_at="2026-08-30T04:00:01Z", wake=True)
        board, _ = module.read_board(repository)
        return module.encode(board).decode()

    def test_registry_present_matches_base_fixture(self) -> None:
        self.assertEqual(self.run_fixture(self.repository, SCRIPT), BASE_FIXTURE)

    def test_script_only_copy_matches_base_fixture(self) -> None:
        copy = Path(self.temporary.name) / "copy"
        copy.mkdir()
        for name in ("codex_chatboard.py", "durable_state.py"):
            shutil.copy(SCRIPT.parent / name, copy / name)
        self.assertFalse((copy.parent / "contracts").exists())
        self.assertEqual(self.run_fixture(self.repository, copy / "codex_chatboard.py"), BASE_FIXTURE)


class ScriptOnlyCopyTest(RepoCase):
    def test_subcommands_without_registry_need_none(self) -> None:
        copy = Path(self.temporary.name) / "copy"
        copy.mkdir()
        for name in ("codex_chatboard.py", "durable_state.py"):
            shutil.copy(SCRIPT.parent / name, copy / name)
        script = copy / "codex_chatboard.py"
        posted = self.run_cli("post", "--author", "a", "--session-id", SESSION_1, "--topic", "t",
                              "--body", "x", script=script)
        self.assertEqual(posted.returncode, 0, posted.stderr)
        for args in (("read",), ("topics",), ("wait", "--since", "0", "--timeout-seconds", "0"),
                     ("claim", "--resource", "r", "--author", "a", "--session-id", SESSION_1, "--note", "n",
                      "--claim-id", mid(7)),
                     ("release", "--resource", "r", "--session-id", SESSION_1, "--claim-id", mid(7))):
            result = self.run_cli(*args, script=script)
            self.assertEqual(result.returncode, 0, (args, result.stderr))
        expected = str(copy.parent / "contracts" / "codex-chatboard-kinds.v1.json")
        # The expected path is relative to the script: <copy>/../contracts
        for args in (("post", "--author", "a", "--session-id", SESSION_1, "--topic", "t", "--body", "x",
                      "--kind", "note"),
                     ("post", "--author", "a", "--session-id", SESSION_1, "--topic", "t", "--body", "x",
                      "--role", "r"),
                     ("kinds",)):
            result = self.run_cli(*args, script=script)
            self.assertEqual(result.returncode, 1, args)
            self.assertIn(expected, result.stderr)


class EnvelopeTest(RepoCase):
    def test_each_kind_accepted_and_rejected_without_each_required_field(self) -> None:
        self.post(kind="request", to=["role:peer"], data={"ask": "q"})  # sequence 1
        for kind, spec in self.registry["kinds"].items():
            env = self.full_envelope(kind)
            if spec.get("writer", "post") == "post":
                result = self.post(**{k: v for k, v in env.items() if k != "kind"}, kind=kind)
                self.assertEqual(result["message"]["kind"], kind)
            else:
                BOARD.validate_envelope(self.registry, **env)
            for path in spec["required"]:
                partial = json.loads(json.dumps(env))
                if path.startswith("data."):
                    partial["data"].pop(path[5:])
                else:
                    partial.pop(path)
                with self.assertRaises(BOARD.ChatboardError) as caught:
                    BOARD.validate_envelope(self.registry, **partial)
                self.assertIn(path, str(caught.exception))
        # a real post is rejected the same way and changes nothing
        _, before = self.board()
        with self.assertRaises(BOARD.ChatboardError):
            self.post(kind="request", to=["role:peer"])
        self.assertEqual(self.board()[1], before)

    def test_enum_violations_name_path_and_every_allowed_value(self) -> None:
        for kind, spec in self.registry["kinds"].items():
            for path, allowed in spec.get("enums", {}).items():
                env = self.full_envelope(kind)
                env["data"][path[5:]] = "bogus"
                with self.assertRaises(BOARD.ChatboardError) as caught:
                    BOARD.validate_envelope(self.registry, **env)
                for text in (path, *allowed):
                    self.assertIn(text, str(caught.exception))

    def test_unregistered_kind_rejected_naming_note_and_kinds(self) -> None:
        with self.assertRaises(BOARD.ChatboardError) as caught:
            self.post(kind="statuss")
        message = str(caught.exception)
        self.assertIn("note", message)
        for name in self.registry["kinds"]:
            self.assertIn(name, message)

    def test_grammar_violations_name_field_and_pattern(self) -> None:
        grammar = self.registry["envelope"]
        with self.assertRaises(BOARD.ChatboardError) as caught:
            BOARD.validate_envelope(self.registry, role="Bad Role")
        self.assertIn("role", str(caught.exception))
        self.assertIn(grammar["role_pattern"], str(caught.exception))
        for bad in ("peer", "role:Bad", "session:not-a-uuid"):
            with self.assertRaises(BOARD.ChatboardError) as caught:
                BOARD.validate_envelope(self.registry, to=[bad])
            self.assertIn("to", str(caught.exception))
            for pattern in grammar["to_patterns"]:
                self.assertIn(pattern, str(caught.exception))
        upper = f"session:{LETTERS.upper()}"
        with self.assertRaises(BOARD.ChatboardError) as caught:
            BOARD.validate_envelope(self.registry, to=[upper])
        self.assertIn("to", str(caught.exception))
        self.assertIn(grammar["to_patterns"][1], str(caught.exception))
        BOARD.validate_envelope(self.registry, role="s10-root")
        BOARD.validate_envelope(self.registry, role="claude-root", to=["role:claude-metrics", f"session:{SESSION_1}"])

    def test_effects_shape(self) -> None:
        BOARD.validate_envelope(self.registry, effects={"shared_writes": [], "external_actions": ["x"]})
        for bad, needle in (({"shared_writes": []}, "effects.external_actions"),
                            ({"shared_writes": [], "external_actions": [], "extra": []}, "effects.extra"),
                            ({"shared_writes": [""], "external_actions": []}, "effects.shared_writes"),
                            ({"shared_writes": "x", "external_actions": []}, "effects.shared_writes")):
            with self.assertRaises(BOARD.ChatboardError) as caught:
                BOARD.validate_envelope(self.registry, effects=bad)
            self.assertIn(needle, str(caught.exception))
            self.assertIn("shared_writes", str(caught.exception))
            self.assertIn("external_actions", str(caught.exception))

    def test_adoption_digest_pattern(self) -> None:
        env = self.full_envelope("adoption")
        env["data"]["handoff_sha256"] = "A" * 64
        with self.assertRaises(BOARD.ChatboardError) as caught:
            BOARD.validate_envelope(self.registry, **env)
        self.assertIn("data.handoff_sha256", str(caught.exception))

    def test_envelope_without_kind_reads_as_note_and_has_no_contract_version(self) -> None:
        result = self.post(role="s10-root", to=["role:peer"], data={"free": 1})
        self.assertNotIn("kind", result["message"])
        self.assertNotIn("contract_version", result["message"])
        self.assertEqual(result["message"]["data"], {"free": 1})

    def test_row_contract_version(self) -> None:
        typed = self.post(kind="note")
        self.assertEqual(typed["message"]["contract_version"], self.registry["version"])
        self.assertIs(type(typed["message"]["contract_version"]), int)
        self.assertNotIn("contract_version", self.post()["message"])
        self.raw_rewrite(lambda b: b["messages"][0].__setitem__("contract_version", 99))
        row = self.board()[0]["messages"][0]
        self.assertEqual(row["contract_version"], 99)
        view = BOARD.read_view(self.board()[0], since=None, limit=10, active_claims=False,
                               observed_at="2026-10-25T00:00:00Z")
        self.assertEqual(view["messages"][0]["contract_version"], 99)

    def test_replay_requires_identical_envelope(self) -> None:
        args = dict(message_id=mid(5), kind="status", data={"subject": "s"}, role="r1")
        first = self.post(**args)
        self.assertEqual(self.post(**args)["message"], first["message"])
        self.assertEqual(len(self.board()[0]["messages"]), 1)
        for change in ({"data": {"subject": "other"}}, {"role": "r2"}, {"kind": "note"}):
            with self.assertRaises(BOARD.ChatboardError) as caught:
                self.post(**{**args, **change})
            self.assertIn("different content", str(caught.exception))


class CrossMessageTest(RepoCase):
    def test_dangling_and_forward_references_rejected(self) -> None:
        self.post()  # sequence 1
        for field in ("in_reply_to", "supersedes"):
            for value in (self.board()[0]["next_sequence"], self.board()[0]["next_sequence"] + 1, 99):
                _, before = self.board()
                with self.assertRaises(BOARD.ChatboardError) as caught:
                    self.post(**{field: value})
                self.assertIn(field, str(caught.exception))
                self.assertEqual(self.board()[1], before)
            self.assertEqual(self.post(**{field: 1})["message"][field], 1)

    def test_forward_reference_rule_on_a_board_whose_next_sequence_is_stale(self) -> None:
        # The existence check would accept a row at sequence >= next_sequence; only the
        # forward-reference rule rejects it.
        self.post()  # sequence 1, next_sequence 2

        def stale(board) -> None:
            board["messages"].append({**board["messages"][0], "message_id": mid(900), "sequence": board["next_sequence"]})

        self.raw_rewrite(stale)
        stale_sequence = self.board()[0]["next_sequence"]
        for field in ("in_reply_to", "supersedes"):
            _, before = self.board()
            with self.assertRaises(BOARD.ChatboardError) as caught:
                self.post(**{field: stale_sequence})
            self.assertIn(field, str(caught.exception))
            self.assertIn("must name an existing earlier message", str(caught.exception))
            self.assertEqual(self.board()[1], before)

    def test_closing_reply_without_in_reply_to_is_a_chatboard_error_even_if_registry_omits_it(self) -> None:
        registry = copy.deepcopy(self.registry)
        registry["kinds"]["reply"]["required"] = ["data.disposition"]
        for disposition in ("answered", "declined"):
            with self.assertRaises(BOARD.ChatboardError) as caught:
                self.post(kind="reply", data={"disposition": disposition}, registry=registry)
            self.assertIn("in_reply_to", str(caught.exception))

    def test_closing_reply_must_name_a_request(self) -> None:
        self.post()  # 1: legacy row without kind
        self.post(kind="note")  # 2
        self.post(kind="status", data={"subject": "s"})  # 3
        self.post(kind="request", to=["role:p"], data={"ask": "q"})  # 4
        for target in (1, 2, 3):
            for disposition in ("answered", "declined"):
                with self.assertRaises(BOARD.ChatboardError) as caught:
                    self.post(kind="reply", in_reply_to=target, data={"disposition": disposition})
                self.assertIn("request", str(caught.exception))
            for disposition in ("deferred", "partial"):
                self.post(kind="reply", in_reply_to=target, data={"disposition": disposition})
        for disposition in ("answered", "declined"):
            self.post(kind="reply", in_reply_to=4, data={"disposition": disposition})
        self.post(kind="receipt", in_reply_to=1)

    def test_closing_check_runs_inside_change_on_retry(self) -> None:
        self.post(kind="request", to=["role:p"], data={"ask": "q"})  # 1
        injected = self.conflict_injector(
            lambda board: board["messages"][0].pop("kind"))
        with injected:
            with self.assertRaises(BOARD.ChatboardError) as caught:
                self.post(kind="reply", in_reply_to=1, data={"disposition": "answered"})
        self.assertIn("request", str(caught.exception))
        self.assertEqual(injected.counts["change"], 2)
        self.assertEqual(injected.counts["publish"], 1)
        self.assertEqual(len(self.board()[0]["messages"]), 1)

    def conflict_injector(self, injection):
        """Context manager: the first publish applies `injection` to the stored board, then conflicts."""
        case = self
        counts = {"change": 0, "publish": 0}
        real_store = GitRefDurableStateStore
        real_mutate = BOARD.mutate

        class Store(real_store):
            injected = False

            def publish(self, key, payload, expected):
                counts["publish"] += 1
                if not Store.injected:
                    Store.injected = True
                    case.raw_rewrite(injection)
                    raise DurableStateError("revision conflict (injected)")
                return super().publish(key, payload, expected)

        def counting_mutate(repository, change):
            def counted(board):
                counts["change"] += 1
                return change(board)
            return real_mutate(repository, counted)

        class Manager:
            def __init__(self) -> None:
                self.counts = counts
                self.patches = [patch.object(BOARD, "GitRefDurableStateStore", Store),
                                patch.object(BOARD, "mutate", counting_mutate)]

            def __enter__(self):
                for item in self.patches:
                    item.start()
                return self

            def __exit__(self, *exc):
                for item in self.patches:
                    item.stop()

        return Manager()

    def test_cas_forward_reference_to_stale_next_sequence_rejected_at_first_change(self) -> None:
        self.post()  # next_sequence is now 2
        injected = self.conflict_injector(lambda board: None)
        with injected:
            with self.assertRaises(BOARD.ChatboardError):
                self.post(kind="receipt", in_reply_to=2)
        self.assertEqual(injected.counts, {"change": 1, "publish": 0})
        self.assertEqual(len(self.board()[0]["messages"]), 1)

    def test_cas_retry_after_append_publishes_once_with_fresh_sequence(self) -> None:
        self.post()  # 1

        def append(board):
            board["messages"].append({**board["messages"][0], "message_id": mid(900), "sequence": 2})
            board["next_sequence"] = 3

        injected = self.conflict_injector(append)
        with injected:
            result = self.post(kind="receipt", in_reply_to=1)
        self.assertEqual(result["message"]["sequence"], 3)
        self.assertEqual(injected.counts["change"], 2)
        self.assertEqual(injected.counts["publish"], 2)  # one conflict, one success
        self.assertEqual(len(self.board()[0]["messages"]), 3)

    def test_cas_retry_after_rewrite_without_referenced_row_rejects(self) -> None:
        self.post()  # 1

        def drop(board):
            board["messages"] = []

        injected = self.conflict_injector(drop)
        with injected:
            with self.assertRaises(BOARD.ChatboardError):
                self.post(kind="receipt", in_reply_to=1)
        self.assertEqual(injected.counts["change"], 2)
        self.assertEqual(injected.counts["publish"], 1)
        self.assertEqual(self.board()[0]["messages"], [])


class AdoptTest(RepoCase):
    def adopt(self, **kw):
        self.counter += 1
        base = dict(author="succ", session_id=SESSION_1, body="adopted", role="claude-root",
                    predecessor_session=SESSION_2, message_id=mid(self.counter),
                    observed_at="2026-10-25T00:00:00Z", handoff_sha256=HEX)
        base.update(kw)
        return BOARD.adopt(self.repository, **base)

    def cli_adopt(self, *extra: str):
        return self.run_cli("adopt", "--author", "succ", "--session-id", SESSION_1, "--body", "b",
                            "--role", "claude-root", "--predecessor-session", SESSION_2,
                            "--handoff-sha256", HEX, *extra)

    def test_post_refuses_adoption_by_registry_writer(self) -> None:
        env = self.full_envelope("adoption")
        _, before = self.board()
        with self.assertRaises(BOARD.ChatboardError) as caught:
            self.post(**env)
        self.assertIn("adopt", str(caught.exception))
        self.assertEqual(self.board()[1], before)
        result = self.run_cli("post", "--author", "a", "--session-id", SESSION_1, "--topic", "t", "--body", "b",
                              "--kind", "adoption", "--role", "r",
                              "--data-file", self.write_json({"predecessor_session": SESSION_2,
                                                              "handoff_sha256": HEX}))
        self.assertEqual(result.returncode, 1)
        self.assertIn("adopt", result.stderr)
        self.assertEqual(self.board()[1], before)

    def write_json(self, value) -> str:
        path = Path(self.temporary.name) / f"f{self.counter}.json"
        self.counter += 1
        path.write_text(json.dumps(value))
        return str(path)

    def test_adopt_row_equals_fixture(self) -> None:
        result = self.adopt(message_id=mid(1), claims_reclaimed=["r1"], channels=["c1"])
        self.assertEqual(result["message"], {
            "message_id": mid(1), "sequence": 1, "observed_at": "2026-10-25T00:00:00Z", "author": "succ",
            "session_id": SESSION_1, "topic": f"session:{SESSION_1}", "body": "adopted", "references": [],
            "kind": "adoption", "role": "claude-root", "to": ["role:claude-root"],
            "data": {"predecessor_session": SESSION_2, "handoff_sha256": HEX,
                     "claims_reclaimed": ["r1"], "channels": ["c1"]},
            "contract_version": self.registry["version"]})
        minimal = self.adopt(message_id=mid(2))["message"]
        self.assertEqual(minimal["data"], {"predecessor_session": SESSION_2, "handoff_sha256": HEX})
        self.assertEqual(len(self.board()[0]["messages"]), 2)

    def test_adopt_cli_refuses_other_envelope_flags_and_topic(self) -> None:
        self.assertEqual(self.cli_adopt().returncode, 0)
        _, before = self.board()
        for flag, value in (("--kind", "note"), ("--to", "role:x"), ("--reply-to", "1"), ("--supersedes", "1"),
                            ("--effects-file", "x.json"), ("--data-file", "x.json"), ("--topic", "t"),
                            ("--ref", "r"), ("--wake", None)):
            result = self.cli_adopt(*([flag] if value is None else [flag, value]))
            self.assertEqual(result.returncode, 2, flag)
            self.assertEqual(self.board()[1], before)

    def test_handoff_path_digest_rules(self) -> None:
        path = Path(self.temporary.name) / "handoff.md"
        path.write_bytes(b"handoff bytes\n")
        digest = hashlib.sha256(b"handoff bytes\n").hexdigest()
        _, before = self.board()
        with self.assertRaises(BOARD.ChatboardError):
            self.adopt(handoff_path=path, handoff_sha256=HEX)
        self.assertEqual(self.board()[1], before)
        self.assertEqual(self.adopt(handoff_path=path, handoff_sha256=digest)["message"]["data"]["handoff_sha256"],
                         digest)
        self.assertEqual(self.adopt(handoff_path=path, handoff_sha256=None)["message"]["data"]["handoff_sha256"],
                         digest)
        with self.assertRaises(BOARD.ChatboardError):
            self.adopt(handoff_path=Path(self.temporary.name) / "missing", handoff_sha256=None)
        with self.assertRaises(BOARD.ChatboardError):
            self.adopt(handoff_sha256=None)

    def test_digest_and_predecessor_rejections(self) -> None:
        _, before = self.board()
        for bad in ("abc", "A" * 64, "g" * 64, "a" * 63):
            with self.assertRaises(BOARD.ChatboardError):
                self.adopt(handoff_sha256=bad)
        letters = "abcdef01-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
        for bad in (letters.upper(), "not-a-uuid", letters.replace("-", ""), SESSION_1):
            with self.assertRaises(BOARD.ChatboardError):
                self.adopt(predecessor_session=bad)
        with self.assertRaises(BOARD.ChatboardError) as caught:
            self.adopt(session_id=letters.upper(), predecessor_session=letters)
        self.assertIn("session_id", str(caught.exception))
        self.assertIn("lowercase", str(caught.exception))
        self.assertEqual(self.board()[1], before)
        self.adopt(predecessor_session=letters)

    def test_adopt_topic_is_lowercase_session_address(self) -> None:
        row = self.adopt()["message"]
        self.assertEqual(row["topic"], f"session:{SESSION_1}")
        result = self.run_cli("adopt", "--author", "succ", "--session-id", LETTERS.upper(), "--body", "b",
                              "--role", "claude-root", "--predecessor-session", SESSION_2,
                              "--handoff-sha256", HEX)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("session_id", result.stderr)

    def test_json_null_data_and_effects_files_are_rejected(self) -> None:
        null_file = Path(self.temporary.name) / "null.json"
        null_file.write_text("null")
        _, before = self.board()
        for flag in ("--data-file", "--effects-file"):
            result = self.run_cli("post", "--author", "a", "--session-id", SESSION_1, "--topic", "t",
                                  "--body", "b", "--kind", "note", flag, str(null_file))
            self.assertNotEqual(result.returncode, 0, flag)
            self.assertIn(f"{flag} must contain a JSON object", result.stderr)
        self.assertEqual(self.board()[1], before)

    def test_replay_of_identical_adopt_succeeds(self) -> None:
        first = self.adopt(message_id=mid(3))
        again = self.adopt(message_id=mid(3))
        self.assertEqual(first["message"], again["message"])
        self.assertEqual(len(self.board()[0]["messages"]), 1)
        with self.assertRaises(BOARD.ChatboardError):
            self.adopt(message_id=mid(3), body="different")


class KindsViewTest(RepoCase):
    def test_view_equals_rendering_of_every_registry_kind(self) -> None:
        lines = BOARD.kinds_view(self.registry)
        self.assertEqual(len(lines), len(self.registry["kinds"]))
        for name, line in zip(self.registry["kinds"], lines):
            self.assertTrue(line.startswith(f"{name} |"))
            spec = self.registry["kinds"][name]
            self.assertIn(f"writer={spec.get('writer', 'post')}", line)
            for path in spec["required"]:
                self.assertIn(path, line)
            for path, values in spec.get("enums", {}).items():
                self.assertIn(f"{path}={'|'.join(values)}", line)
            for optional in spec["optional_data"]:
                self.assertIn(optional, line)
            self.assertEqual(BOARD.kinds_view(self.registry, name), [line])

    def test_cli_output_and_unknown_kind(self) -> None:
        full = self.run_cli("kinds")
        self.assertEqual(full.stdout.splitlines(), BOARD.kinds_view(self.registry))
        one = self.run_cli("kinds", "reply")
        self.assertEqual(one.stdout.splitlines(), BOARD.kinds_view(self.registry, "reply"))
        bad = self.run_cli("kinds", "nope")
        self.assertEqual(bad.returncode, 1)
        for name in self.registry["kinds"]:
            self.assertIn(name, bad.stderr)
        self.assertFalse(self.board()[0]["messages"])  # reads no board

    def test_view_follows_a_modified_registry(self) -> None:
        modified = json.loads(REGISTRY.read_text())
        modified["kinds"]["zeta"] = {"consumer": "x", "required": ["data.q"], "enums": {}, "optional_data": ["opt"]}
        modified["kinds"]["reply"]["enums"]["data.disposition"] = ["answered", "novel"]
        temporary = Path(self.temporary.name) / "registry.json"
        temporary.write_text(json.dumps(modified))
        loaded = BOARD.load_registry(temporary)
        lines = BOARD.kinds_view(loaded)
        self.assertEqual(len(lines), len(self.registry["kinds"]) + 1)
        self.assertTrue(any(line.startswith("zeta |") and "data.q" in line for line in lines))
        reply = BOARD.kinds_view(loaded, "reply")[0]
        self.assertIn("answered|novel", reply)
        self.assertNotIn("declined", reply)
        with patch.object(BOARD, "REGISTRY_PATH", temporary):
            self.assertEqual(self.post_kind("zeta")["message"]["kind"], "zeta")

    def post_kind(self, kind):
        return BOARD.post(self.repository, author="a", session_id=SESSION_1, topic="t", body="b",
                          references=[], message_id=mid(1), observed_at="2026-10-25T00:00:00Z",
                          kind=kind, data={"q": "v"})

    def test_post_help_prints_kind_line_above_help(self) -> None:
        result = self.run_cli("post", "--kind", "reply", "--help")
        self.assertEqual(result.returncode, 0)
        lines = result.stdout.splitlines()
        self.assertEqual(lines[0], BOARD.kinds_view(self.registry, "reply")[0])
        self.assertTrue(lines[1].startswith("usage:"))


BASE_COMMIT = "7a7796002e95ce3874a3b27b3f951b6f5a6f9c28"
BASE_ADAPTER_PATH = "skills/durable-state/scripts/codex_chatboard.py"
# The S2<->S5 adoption fixture (plan S2 Checks, D10): an adoption row addressed to
# the successor by `to` whose topic is not the successor's session topic.
ADOPTION_SUCCESSOR = "33333333-3333-4333-8333-333333333333"


class ReadFilterTest(RepoCase):
    def seed(self) -> None:
        self.post(topic="t", body="legacy")                                   # 1: no kind
        self.post(topic="t", body="s", kind="status", role="alpha-role", data={"subject": "s"})      # 2
        self.post(topic="t", body="n", kind="note", role="beta-role")         # 3
        self.post(topic="t", body="s2", kind="status", role="beta-role", data={"subject": "s"})      # 4
        for i in range(5):
            self.post(topic="t", body=f"filler {i}")                          # 5..9

    def view(self, **kw):
        board, _ = self.board()
        args = dict(since=None, limit=100, active_claims=False, observed_at="2026-10-25T00:00:00Z")
        args.update(kw)
        return [row["sequence"] for row in BOARD.read_view(board, **args)["messages"]]

    def test_kind_and_role_filters_apply_before_limit_and_tail_slice(self) -> None:
        self.seed()
        # The two status rows are older than the newest 3 rows of any kind.
        self.assertEqual(self.view(limit=3, kinds=["status"]), [2, 4])
        self.assertEqual(self.view(limit=1, kinds=["status"]), [4])
        self.assertEqual(self.view(limit=1, since=0, kinds=["status"]), [2])
        self.assertEqual(self.view(limit=100, roles=["beta-role"]), [3, 4])
        self.assertEqual(self.view(limit=1, roles=["beta-role"]), [4])
        self.assertEqual(self.view(kinds=["status"], roles=["beta-role"]), [4])
        self.assertEqual(self.view(kinds=["status", "request"]), [2, 4])

    def test_note_matches_rows_without_kind(self) -> None:
        self.seed()
        self.assertEqual(self.view(kinds=["note"]), [1, 3, 5, 6, 7, 8, 9])

    def test_stored_unknown_kind_is_filterable_as_recorded(self) -> None:
        self.seed()
        self.raw_rewrite(lambda board: board["messages"][1].update(kind="future-kind"))
        self.assertEqual(self.view(kinds=["future-kind"]), [2])

    def test_cli_flags_and_default_output(self) -> None:
        self.seed()
        out = self.run_cli("read", "--kind", "status", "--role", "beta-role", "--limit", "5")
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertEqual([m["sequence"] for m in json.loads(out.stdout)["messages"]], [4])
        out = self.run_cli("read", "--format", "compact", "--active-claims")
        self.assertEqual(out.returncode, 1)
        self.assertIn("--active-claims", out.stderr)


class CompactReadTest(RepoCase):
    def test_golden(self) -> None:
        self.post(topic="legacy-topic", author="old", body="a legacy\nmultiline   body")        # 1
        self.post(topic="aspect:x", author="alpha", kind="request", role="s10-root",
                  to=["role:a", "session:" + SESSION_2], body="x" * 100,
                  data={"ask": "a"}, effects={"shared_writes": [], "external_actions": []})      # 2
        self.post(topic="aspect:x", author="beta", kind="reply", role="peer", in_reply_to=2,
                  supersedes=1, body="y" * 80, data={"disposition": "deferred"})             # 3
        self.raw_rewrite(lambda board: board["messages"][0].update(kind="future-kind"))
        out = self.run_cli("read", "--format", "compact")
        self.assertEqual(out.returncode, 0, out.stderr)
        expected = "\n".join([
            "1\tfuture-kind\t-\t-\t-\told\tlegacy-topic\ta legacy multiline body",
            f"2\trequest\ts10-root\trole:a,session:{SESSION_2}\t-\talpha\taspect:x\t" + "x" * 77 + "...",
            "3\treply\tpeer\t-\treply:2,supersedes:1\tbeta\taspect:x\t" + "y" * 80,
        ]) + "\n"
        self.assertEqual(out.stdout, expected)

    def test_untyped_row_reads_as_note_and_malformed_to_is_dash(self) -> None:
        self.post(body="plain")
        self.raw_rewrite(lambda board: board["messages"][0].update(to="role:a"))
        line = self.run_cli("read", "--format", "compact").stdout
        self.assertEqual(line.split("\t")[:4], ["1", "note", "-", "-"])


class WaitDeliveryByToTest(RepoCase):
    def wait(self, *topics: str, since: int = 0, limit: int = 100, **kw):
        return BOARD.wait_board(self.repository, since=since, limit=limit, topics=list(topics),
                                timeout_seconds=0, **kw)

    def test_role_and_session_addresses_wake_each_addressee_once(self) -> None:
        self.post(topic="aspect:x", to=["role:a", "role:b"], kind="note", role="p")
        self.post(topic="aspect:y", to=[f"session:{SESSION_2}"])
        a = self.wait("role:a")
        self.assertEqual([m["sequence"] for m in a["messages"]], [1])
        self.assertEqual(a["matches"], [{"sequence": 1, "matched_by": ["to"]}])
        self.assertEqual([m["sequence"] for m in self.wait("role:b")["messages"]], [1])
        self.assertEqual(self.wait("role:c")["messages"], [])
        self.assertNotIn("matches", self.wait("role:c"))
        s = self.wait(f"session:{SESSION_2}")
        self.assertEqual([m["sequence"] for m in s["messages"]], [2])
        self.assertEqual(s["matches"], [{"sequence": 2, "matched_by": ["to"]}])

    def test_returned_row_is_byte_unchanged(self) -> None:
        self.post(topic="aspect:x", to=["role:a"])
        board, _ = self.board()
        self.assertEqual(self.wait("role:a")["messages"], board["messages"])

    def test_topic_only_match_has_no_matches_key(self) -> None:
        self.post(topic="aspect:x")
        self.post(topic="aspect:x", to=["role:z"])
        result = self.wait("aspect:x")
        self.assertEqual(len(result["messages"]), 2)
        self.assertNotIn("matches", result)

    def test_mixed_result_reports_each_rows_path(self) -> None:
        self.post(topic="role:a")                          # 1 topic only
        self.post(topic="aspect:x", to=["role:a"])         # 2 to only
        self.post(topic="role:a", to=["role:a"])           # 3 both
        self.post(topic="aspect:x")                        # 4 neither
        result = self.wait("role:a")
        self.assertEqual([m["sequence"] for m in result["messages"]], [1, 2, 3])
        self.assertEqual(result["matches"], [{"sequence": 1, "matched_by": ["topic"]},
                                             {"sequence": 2, "matched_by": ["to"]},
                                             {"sequence": 3, "matched_by": ["topic", "to"]}])

    def test_row_matching_by_topic_and_to_is_delivered_once(self) -> None:
        self.post(topic="role:a", to=["role:a"])
        result = self.wait("role:a")
        self.assertEqual(len(result["messages"]), 1)
        self.assertEqual(result["matches"], [{"sequence": 1, "matched_by": ["topic", "to"]}])

    def test_duplicate_to_address_and_duplicate_topic_filter_deliver_once(self) -> None:
        self.post(topic="aspect:x", to=["role:a", "role:a"])
        result = self.wait("role:a", "role:a")
        self.assertEqual(len(result["messages"]), 1)
        self.assertEqual(result["matches"], [{"sequence": 1, "matched_by": ["to"]}])
        result = self.wait("role:a", "aspect:x")
        self.assertEqual(result["matches"], [{"sequence": 1, "matched_by": ["topic", "to"]}])

    def test_malformed_stored_to_does_not_match(self) -> None:
        self.post(topic="aspect:x")
        for bad in ("role:a", 5, [1, "role:a"], {"role:a": 1}, None):
            with self.subTest(bad=bad):
                self.raw_rewrite(lambda board: board["messages"][0].update(to=bad))
                self.assertEqual(self.wait("role:a")["messages"], [])

    def test_no_topic_selected_matches_all_without_matches_key(self) -> None:
        self.post(topic="aspect:x", to=["role:a"])
        self.post(topic="aspect:y")
        result = self.wait()
        self.assertEqual(len(result["messages"]), 2)
        self.assertNotIn("matches", result)

    def test_filter_before_limit_and_cursor_hold_for_to_matches(self) -> None:
        self.post(topic="aspect:x", to=["role:a"])   # 1
        self.post(topic="aspect:x", to=["role:a"])   # 2
        for _ in range(4):
            self.post(topic="aspect:other")          # 3..6
        self.post(topic="aspect:x", to=["role:a"])   # 7
        first = self.wait("role:a", limit=2)
        self.assertEqual([m["sequence"] for m in first["messages"]], [1, 2])
        self.assertTrue(first["has_more"])
        self.assertEqual(first["next_since"], 2)
        self.assertEqual(len(first["matches"]), 2)
        second = self.wait("role:a", since=first["next_since"], limit=2)
        self.assertEqual([m["sequence"] for m in second["messages"]], [7])
        self.assertFalse(second["has_more"])
        self.assertEqual(second["next_since"], 7)
        self.assertEqual(second["matches"], [{"sequence": 7, "matched_by": ["to"]}])

    def test_control_topic_only_wait_gets_no_to_matches(self) -> None:
        # to-patterns allow only role:/session: addresses, so a to address can
        # never equal a control: topic; a waiter selecting only its control
        # topic gets no `to` matches and control rows stay excluded as at HEAD.
        topic = f"control:session:{SESSION_2}"
        for pattern in self.registry["envelope"]["to_patterns"]:
            self.assertIsNone(re.match(pattern, topic))
        self.post(topic="aspect:x", to=["role:a"])
        self.post(topic="aspect:x", to=["role:a", "role:b"])
        with self.assertRaisesRegex(BOARD.ChatboardError, "session binding"):
            self.wait(topic)  # unbound control topic is still refused

        class StubControl:  # no telemetry: only the board side of the wait is under test
            def __init__(self, path: Path) -> None:
                self.topic, self.rollout, self.binary, self.cursor = topic, path, path, None

            def read(self) -> dict:
                return {"event": None}

        stub_file = Path(self.temporary.name) / "stub"
        stub_file.write_text("x")
        result = self.wait(topic, control=StubControl(stub_file))
        self.assertEqual(result["messages"], [])
        self.assertNotIn("matches", result)
        self.assertEqual(result["next_since"], 2)
        # tests/test_context_control.py l.114-115 pins the same result with the real reader.

    def test_adoption_fixture_s2_s5(self) -> None:
        """S2<->S5 adoption fixture: an adoption row addressed by `to` to the successor
        on a topic that is not the successor's session topic reaches a waiter on
        session:<successor> with matched_by ["to"]."""
        handoff = "b" * 64
        # `post` refuses kind adoption (its writer is `adopt`), so store the row shape directly.
        self.post(topic="aspect:handoff")
        self.raw_rewrite(lambda board: board["messages"][0].update(
            kind="adoption", role="s10-root", to=[f"session:{ADOPTION_SUCCESSOR}"],
            data={"handoff_sha256": handoff, "predecessor_session": SESSION_2}))
        result = self.wait(f"session:{ADOPTION_SUCCESSOR}")
        self.assertEqual(len(result["messages"]), 1)
        row = result["messages"][0]
        self.assertNotEqual(row["topic"], f"session:{ADOPTION_SUCCESSOR}")
        self.assertEqual(row["kind"], "adoption")
        self.assertEqual(row["data"], {"handoff_sha256": handoff, "predecessor_session": SESSION_2})
        self.assertEqual(result["matches"], [{"sequence": 1, "matched_by": ["to"]}])


class BaseByteIdentityTest(RepoCase):
    """Default `read` and a topic-only `wait` equal the base adapter's output on one board."""

    def base_script(self) -> Path:
        directory = Path(self.temporary.name) / "base"
        directory.mkdir()
        source = subprocess.run(["git", "show", f"{BASE_COMMIT}:{BASE_ADAPTER_PATH}"],
                                capture_output=True, check=True, cwd=Path(__file__).parent).stdout
        (directory / "codex_chatboard.py").write_bytes(source)
        shutil.copy(SCRIPT.parent / "durable_state.py", directory / "durable_state.py")
        return directory / "codex_chatboard.py"

    def test_read_and_topic_only_wait_match_base(self) -> None:
        base = self.base_script()
        self.post(topic="aspect:x", body="one")
        self.post(topic="role:a", body="two", wake=True)
        self.post(topic="aspect:x", kind="request", role="r", to=["role:q"], body="three",
                  data={"ask": "a"}, effects={"shared_writes": [], "external_actions": []})
        for args in (("read",), ("read", "--since", "1", "--limit", "1")):
            now, then = self.run_cli(*args), self.run_cli(*args, script=base)
            self.assertEqual(now.stdout, then.stdout, args)
            self.assertEqual(now.returncode, then.returncode)
        for args in (("wait", "--since", "0", "--timeout-seconds", "0", "--topic", "aspect:x"),
                     ("wait", "--since", "0", "--timeout-seconds", "0"),
                     ("wait", "--since", "0", "--timeout-seconds", "0", "--topic", "role:a")):
            now, then = self.run_cli(*args), self.run_cli(*args, script=base)
            self.assertEqual(now.stdout, then.stdout, args)
            self.assertEqual(now.returncode, then.returncode)
            self.assertNotIn('"matches"', now.stdout)


def backticked(text: str) -> list[str]:
    return re.findall(r"`([^`]+)`", text)


def table_rows(section: str) -> list[list[str]]:
    rows = []
    for line in section.splitlines():
        if line.startswith("|") and not re.match(r"^\|[-| ]+\|$", line):
            rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return rows[1:]  # drop the header


def section(text: str, start: str, end: str) -> str:
    begin = text.index(start)
    return text[begin:text.index(end, begin + len(start))]


class ParityTest(RepoCase):
    """docs -> registry. Registry is the expected value; each document is checked against it.

    Included (closed list): the draft's Kinds table (kind, consumer, required,
    optional data), the enumerations in its field notes (reply.disposition,
    availability.state), availability.ending -> handoff (path, sha256), the
    draft's Envelope table (field names; `to` forms; contract_version is a
    pending projection), and the operator reference's kind -> writer table.
    Excluded, with reasons: notice.action "or none" (a sentinel inside free
    text that no check reads); live_children empty-versus-absent (a reading
    rule, not a validation attribute); the request-open rule (S3's `requests`
    view owns and tests it); the registry `writer` attribute against the draft
    (the draft does not project it); the adoption digest pattern against the
    draft's Deterministic checks prose (prose, same value stated in two forms).
    """

    def test_draft_kinds_table(self) -> None:
        text = DRAFT.read_text()
        rows = table_rows(section(text, "## Kinds", "Field notes:"))
        self.assertTrue(rows, "Kinds table not extracted")
        documented = {}
        for cells in rows:
            self.assertEqual(len(cells), 4, cells)
            name = backticked(cells[0])
            self.assertEqual(len(name), 1, cells)
            documented[name[0]] = cells
        registry_kinds = self.registry["kinds"]
        for name in registry_kinds:
            self.assertIn(name, documented, f"registry kind {name} missing from draft Kinds table")
        for name in documented:
            self.assertIn(name, registry_kinds, f"draft kind {name} missing from registry (source of truth)")
        for name, cells in documented.items():
            spec = registry_kinds[name]
            self.assertEqual(cells[1], spec["consumer"], f"{name}: consumer; registry value is the source")
            required = [] if cells[2] == "—" else backticked(cells[2])
            self.assertEqual(required, spec["required"], f"{name}: required; registry value is the source")
            optional = [] if cells[3] == "—" else backticked(cells[3])
            top = [item for item in spec["optional_data"] if "." not in item]
            self.assertEqual(optional, top, f"{name}: optional data; registry value is the source")

    def test_draft_field_note_enumerations_and_nested_optionals(self) -> None:
        text = re.sub(r"\s+", " ", section(DRAFT.read_text(), "Field notes:", "### Not defined"))
        found = {}
        match = re.search(r"`reply\.disposition` is one of ((?:`\w+`(?:, | or )?)+)\.", text)
        self.assertTrue(match, "reply.disposition enumeration not extracted")
        found["reply"] = backticked(match.group(1))
        match = re.search(r"`availability\.state` is ((?:`\w+`(?:, | or )?)+)\.", text)
        self.assertTrue(match, "availability.state enumeration not extracted")
        found["availability"] = backticked(match.group(1))
        self.assertEqual(found["reply"], self.registry["kinds"]["reply"]["enums"]["data.disposition"])
        self.assertEqual(found["availability"], self.registry["kinds"]["availability"]["enums"]["data.state"])
        match = re.search(r"carries `handoff` \(`path`, `sha256`\)", text)
        self.assertTrue(match, "availability handoff (path, sha256) not extracted")
        drafted = {f"handoff.{leaf}" for leaf in re.findall(r"`(\w+)`", match.group(0))[1:]}
        self.assertEqual(drafted, {"handoff.path", "handoff.sha256"})
        nested = {item for item in self.registry["kinds"]["availability"]["optional_data"] if "." in item}
        self.assertEqual(nested, drafted, "nested optional data; registry value is the source")

    def test_draft_envelope_table(self) -> None:
        rows = table_rows(section(DRAFT.read_text(), "## Envelope", "### Effects"))
        self.assertTrue(rows, "Envelope table not extracted")
        documented = {backticked(cells[0])[0]: cells for cells in rows}
        registry_fields = self.registry["envelope"]["fields"]
        for name in registry_fields:
            if name == "contract_version" and name not in documented:
                print("pending R2 item: draft Envelope table has no contract_version row")
                continue
            self.assertIn(name, documented, f"registry envelope field {name} missing from draft")
        for name in documented:
            self.assertIn(name, registry_fields, f"draft envelope field {name} missing from registry")
        forms = backticked(documented["to"][2])
        self.assertEqual([f for f in forms if ":" in f], self.registry["envelope"]["to_forms"])
        patterns = self.registry["envelope"]["to_patterns"]
        self.assertEqual(len(self.registry["envelope"]["to_forms"]), len(patterns))
        for form, pattern in zip(self.registry["envelope"]["to_forms"], patterns):
            self.assertTrue(pattern.startswith("^" + form.split(":")[0] + ":"), (form, pattern))
        session_pattern = next(p for p in patterns if p.startswith("^session:"))
        self.assertIsNone(re.match(session_pattern, f"session:{LETTERS.upper()}"))
        self.assertIsNotNone(re.match(session_pattern, f"session:{LETTERS}"))
        if "contract_version" in documented:
            self.assertIn("integer", documented["contract_version"][1])

    def test_operator_reference_states_role_pattern_and_effects_keys(self) -> None:
        ticks = backticked(re.sub(r"```.*?```", "", OPERATOR_DOC.read_text(), flags=re.S))
        envelope = self.registry["envelope"]
        self.assertTrue(envelope["role_pattern"] in ticks, "role_pattern not stated in backticks")
        for key in envelope["effects_keys"]:
            self.assertTrue(key in ticks, f"{key} not stated in backticks")

    def test_operator_reference_documents_read_filters_and_delivery_by_to(self) -> None:
        text = OPERATOR_DOC.read_text()
        for needle in ("--kind", "--role", "--format compact", "matched_by", "delivery is not authorization"):
            self.assertIn(needle, text)
        self.assertNotIn("`wait` does not read `to`", text)

    def test_operator_reference_writers(self) -> None:
        text = OPERATOR_DOC.read_text()
        rows = table_rows(section(text, "Registered kinds and writers:", "Checks before the board"))
        self.assertTrue(rows, "kind/writer table not extracted")
        documented = {backticked(cells[0])[0]: backticked(cells[1])[0] for cells in rows}
        expected = {name: spec.get("writer", "post") for name, spec in self.registry["kinds"].items()}
        self.assertEqual(documented, expected, "registry values are the source")


if __name__ == "__main__":
    unittest.main()
