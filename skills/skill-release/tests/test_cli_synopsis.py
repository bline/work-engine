from __future__ import annotations

import contextlib
import importlib.util
import io
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "scripts" / "cli_synopsis.py"
spec = importlib.util.spec_from_file_location("cli_synopsis", SCRIPT)
synopsis = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = synopsis
spec.loader.exec_module(synopsis)

TOOL = '''\
import argparse

parser = argparse.ArgumentParser(description="fixture")
parser.add_argument("--repository", required=True)
sub = parser.add_subparsers(dest="command", required=True)
for name, flags in (("alpha", ["--one"]), ("beta", ["--two", "--three"]), ("gamma", [])):
    p = sub.add_parser(name, help=f"{name} help")
    for flag in flags:
        p.add_argument(flag, required=flag != "--three")
parser.parse_args()
'''

FLAT = '''\
import argparse

parser = argparse.ArgumentParser(description="flat fixture")
parser.add_argument("--request", required=True)
parser.add_argument("--limit", type=int)
parser.parse_args()
'''

BEGIN = '<!-- cli-synopsis script=tools/tool.py{extra} -->'
END = "<!-- /cli-synopsis -->"


def block(extra: str = "", body: str = "") -> str:
    return f"{BEGIN.format(extra=extra)}\n{body}{END}\n"


class ParseTest(unittest.TestCase):
    def parse(self, text: str):
        return synopsis.parse_blocks("doc.md", text.split("\n"))

    def test_marker_defaults_and_options(self):
        blocks, problems = self.parse("intro\n\n" + block())
        self.assertEqual(problems, [])
        (b,) = blocks
        self.assertEqual((b.script, b.subs, b.display), ("tools/tool.py", None, "python3 tools/tool.py"))
        self.assertEqual((b.begin, b.end), (2, 3))
        blocks, problems = self.parse(block(' subs="beta, alpha" display="python3 <skill-dir>/x.py --repository ."'))
        self.assertEqual(problems, [])
        self.assertEqual(blocks[0].subs, ("beta", "alpha"))
        self.assertEqual(blocks[0].display, "python3 <skill-dir>/x.py --repository .")

    def test_markers_inside_fenced_code_are_ignored(self):
        text = "```markdown\n" + block() + "```\n\n~~~\n" + END + "\n~~~\n"
        self.assertEqual(self.parse(text), ([], []))

    def test_malformed_markers_are_reported_with_line_numbers(self):
        cases = {
            "unknown key": ('<!-- cli-synopsis script=a.py color=red -->\n' + END, "unknown marker key 'color'"),
            "missing script": ('<!-- cli-synopsis subs=a -->\n' + END, "no script="),
            "duplicate key": ('<!-- cli-synopsis script=a.py script=b.py -->\n' + END, "given twice"),
            "bare word": ('<!-- cli-synopsis script=a.py subs -->\n' + END, "not key=value"),
            "bad quoting": ('<!-- cli-synopsis script=a.py display="x -->\n' + END, "unparseable"),
            "absolute script": ('<!-- cli-synopsis script=/bin/ls -->\n' + END, "repository-relative"),
            "parent script": ('<!-- cli-synopsis script=../a.py -->\n' + END, "repository-relative"),
            "empty subs": ('<!-- cli-synopsis script=a.py subs=, -->\n' + END, "subs= is empty"),
            "repeated subs": ('<!-- cli-synopsis script=a.py subs="a a" -->\n' + END, "repeats"),
            "no closing arrow": ('<!-- cli-synopsis script=a.py\n' + END, "malformed begin marker"),
            "no end": ('<!-- cli-synopsis script=a.py -->\ntext\n', "without an end marker"),
            "stray end": ('text\n' + END + '\n', "without a begin marker"),
            "nested": ('<!-- cli-synopsis script=a.py -->\n<!-- cli-synopsis script=b.py -->\n' + END,
                       "begin marker inside the block begun at line 1"),
            "bad end": ('<!-- cli-synopsis script=a.py -->\n<!-- /cli-synopsis  x -->\n',
                        "malformed end marker inside"),
        }
        for label, (text, expected) in cases.items():
            with self.subTest(label):
                blocks, problems = self.parse(text)
                self.assertEqual(blocks, [], problems)
                self.assertTrue(any(expected in p and p.startswith("doc.md:") for p in problems), problems)


class GeneratorTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)
        self.put("tools/tool.py", TOOL)
        self.put("tools/flat.py", FLAT)

    def put(self, rel: str, text: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def doc(self, text: str) -> None:
        self.put("doc.md", text)

    def read(self) -> str:
        return (self.root / "doc.md").read_text()

    def run_cli(self, *args: str):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = synopsis.main(["--repository", str(self.root), *args])
        return code, out.getvalue(), err.getvalue()

    def test_all_subcommands_show_global_options_once_and_drop_help_flag(self):
        self.doc("Intro.\n\n" + block())
        self.assertEqual(self.run_cli("write")[0], 0)
        self.assertEqual(self.read(), "Intro.\n\n" + block(body="\n".join([
            "```text",
            "# global options, given before the subcommand: --repository REPOSITORY",
            "python3 tools/tool.py alpha --one ONE",
            "python3 tools/tool.py beta --two TWO [--three THREE]",
            "python3 tools/tool.py gamma",
            "```"]) + "\n"))

    def test_subset_follows_marker_order_and_names_the_others(self):
        self.doc(block(' subs="beta alpha" display="run <dir>/tool.py"'))
        self.run_cli("write")
        lines = self.read().split("\n")
        self.assertEqual(lines[2:6], [
            "# global options, given before the subcommand: --repository REPOSITORY",
            "run <dir>/tool.py beta --two TWO [--three THREE]",
            "run <dir>/tool.py alpha --one ONE",
            "# other subcommands (put --help after one for its usage): gamma"])

    def test_globals_already_in_display_are_not_repeated(self):
        self.doc(block(' subs=gamma display="python3 tools/tool.py --repository ."'))
        self.run_cli("write")
        body = self.read()
        self.assertIn("python3 tools/tool.py --repository . gamma\n", body)
        self.assertNotIn("global options", body)

    def test_script_without_subcommands_shows_one_line(self):
        self.doc(block().replace("tools/tool.py", "tools/flat.py"))
        self.run_cli("write")
        self.assertIn("```text\npython3 tools/flat.py --request REQUEST [--limit LIMIT]\n```", self.read())

    def test_unknown_subcommand_missing_script_and_subs_on_flat_script_are_problems(self):
        for extra, script, expected in ((' subs=nope', "tools/tool.py", "has no subcommand nope"),
                                        ("", "tools/missing.py", "does not exist"),
                                        (' subs=alpha', "tools/flat.py", "has no subcommands")):
            with self.subTest(expected):
                self.doc(block(extra).replace("tools/tool.py", script))
                before = self.read()
                code, _, err = self.run_cli("check")
                self.assertEqual(code, 1)
                self.assertIn(expected, err)
                self.assertEqual(self.run_cli("write")[0], 1)
                self.assertEqual(self.read(), before)

    def test_check_names_every_stale_block_and_write_repairs_them(self):
        self.doc("# A\n" + block(' subs=alpha') + "\n# B\n" + block(' subs=gamma') + "\n# C\n" + block(' subs=beta'))
        self.run_cli("write")
        code, out, err = self.run_cli("check")
        self.assertEqual((code, err), (0, ""))
        self.assertIn("3 block(s), 0 stale", out)
        edited = self.read().replace("--one ONE", "--one WRONG").replace("tool.py gamma", "tool.py gamma --x")
        self.doc(edited)
        code, _, err = self.run_cli("check")
        self.assertEqual(code, 1)
        stale = [line for line in err.splitlines() if "stale synopsis" in line]
        self.assertEqual(len(stale), 2, err)
        self.assertIn("doc.md:2: stale synopsis for tools/tool.py subs=alpha", stale[0])
        self.assertIn("subs=gamma", stale[1])
        self.assertEqual(self.read(), edited, "check must not change files")
        self.assertEqual(self.run_cli("write")[0], 0)
        self.assertEqual(self.run_cli("check")[0], 0)

    def test_writing_twice_changes_nothing(self):
        self.doc("Text.\n\n" + block() + "\nTail without newline")
        self.run_cli("write")
        once = self.read()
        mtime = (self.root / "doc.md").stat().st_mtime_ns
        code, out, _ = self.run_cli("write")
        self.assertEqual(code, 0)
        self.assertIn("0 file(s) rewritten", out)
        self.assertEqual(self.read(), once)
        self.assertEqual((self.root / "doc.md").stat().st_mtime_ns, mtime)
        self.assertTrue(once.endswith("Tail without newline"))

    def test_output_is_independent_of_terminal_width(self):
        self.doc(block(' subs=beta'))
        self.run_cli("write")
        wide = self.read()
        import os
        previous = os.environ.get("COLUMNS")
        os.environ["COLUMNS"] = "20"
        self.addCleanup(lambda: os.environ.__setitem__("COLUMNS", previous) if previous is not None
                        else os.environ.pop("COLUMNS", None))
        self.assertEqual(self.run_cli("check")[0], 0)
        self.assertEqual(self.read(), wide)

    def test_files_without_markers_are_untouched_and_unread_by_the_generator(self):
        self.put("plain.md", "no markers here\n")
        self.doc(block(' subs=gamma'))
        self.assertEqual(self.run_cli("write")[0], 0)
        self.assertEqual((self.root / "plain.md").read_text(), "no markers here\n")


class RepositoryTest(unittest.TestCase):
    def test_repository_synopses_are_current(self):
        repository = Path(__file__).resolve().parents[3]
        self.assertEqual(synopsis.synopsis_problems(repository), [])


if __name__ == "__main__":
    unittest.main()
