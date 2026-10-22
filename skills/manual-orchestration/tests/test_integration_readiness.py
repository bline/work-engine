from __future__ import annotations

import ast
import hashlib
import json
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "integration_readiness.py"

BINARY_TEMPLATE = """#!{python}
import json, sys
with open({log!r}, "a") as stream:
    stream.write(json.dumps(sys.argv[1:]) + "\\n")
command, args = sys.argv[1], sys.argv[2:]
out = args[args.index("--out") + 1] if "--out" in args else None
exit_code = {plan_exit} if command == "plan" else {evaluate_exit}
if exit_code == 0 and out and not ({skip_write} and command == "evaluate"):
    with open(out, "x") as stream:
        stream.write("{{}}\\n")
sys.exit(exit_code)
"""

BATTERY_TEMPLATE = """import json, os, sys
args = sys.argv[1:]
save = args[args.index("--save") + 1]
with open({log!r}, "a") as stream:
    stream.write(json.dumps({{"argv": args, "save_existed": os.path.exists(save)}}) + "\\n")
if {write_result}:
    with open(save, "x") as stream:
        json.dump({{"battery_sha256": {digest!r}, "failed": [], "checks": {{}}}}, stream)
if {print_digest}:
    print(json.dumps({{"battery_sha256": {digest!r}, "check_count": 0, "failed": []}}))
sys.exit({exit_code})
"""

DIGEST = "ab" * 32


class Fixture:
    """One stub binary, one stub battery and a profile/subject pair in a scratch directory."""

    def __init__(self, *, plan_exit=0, evaluate_exit=0, skip_write=False, battery_exit=0,
                 write_result=True, print_digest=True) -> None:
        self.directory = Path(tempfile.mkdtemp())
        self.binary_log = self.directory / "binary.log"
        self.battery_log = self.directory / "battery.log"
        self.binary = self.directory / "work-engine-readiness"
        self.binary.write_text(BINARY_TEMPLATE.format(
            python=sys.executable, log=str(self.binary_log), plan_exit=plan_exit,
            evaluate_exit=evaluate_exit, skip_write=skip_write))
        self.binary.chmod(self.binary.stat().st_mode | stat.S_IXUSR)
        self.digest = hashlib.sha256(self.binary.read_bytes()).hexdigest()
        self.battery = self.directory / "evidence_battery.py"
        self.battery.write_text(BATTERY_TEMPLATE.format(
            log=str(self.battery_log), write_result=write_result, print_digest=print_digest,
            digest=DIGEST, exit_code=battery_exit))
        self.profiles = []
        self.subjects = []
        for name in ("a", "b"):
            profile, subject = self.directory / f"profile-{name}.json", self.directory / f"subject-{name}.json"
            profile.write_text("{}\n")
            subject.write_text("{}\n")
            self.profiles.append(profile)
            self.subjects.append(subject)
        self.out_dir = self.directory / "out"

    def run(self, pairs=1, binary_sha256=None, extra=()):
        command = [sys.executable, "-I", str(SCRIPT), "--binary", str(self.binary),
                   "--binary-sha256", binary_sha256 or self.digest, "--battery", str(self.battery),
                   "--out-dir", str(self.out_dir), *extra]
        for index in range(pairs):
            command += ["--profile", str(self.profiles[index]), "--subject", str(self.subjects[index])]
        return subprocess.run(command, capture_output=True, text=True, check=False)

    def binary_calls(self):
        if not self.binary_log.exists():
            return []
        return [json.loads(line) for line in self.binary_log.read_text().splitlines()]

    def battery_calls(self):
        if not self.battery_log.exists():
            return []
        return [json.loads(line) for line in self.battery_log.read_text().splitlines()]


class ExitZeroTests(unittest.TestCase):
    def test_evaluation_written_and_observation_shaped(self) -> None:
        fixture = Fixture()
        done = fixture.run()
        self.assertEqual(done.returncode, 0, done.stderr)
        summary = json.loads(done.stdout)
        evaluation = fixture.out_dir / "evaluation-1.json"
        self.assertTrue(evaluation.is_file())
        self.assertEqual(summary["evaluations"][0]["sha256"], hashlib.sha256(evaluation.read_bytes()).hexdigest())
        observation = json.loads((fixture.out_dir / "observation.json").read_text())
        self.assertEqual(observation["schema"], "decision-readiness-observation-v1")
        self.assertEqual(observation["battery_exit"], 0)
        self.assertEqual(observation["stdout_battery_sha256"], DIGEST)
        for key, name in (("plan", "run-plan.json"), ("result", "battery-result.json")):
            path = fixture.out_dir / name
            self.assertEqual(observation[key]["path"], str(path.resolve()))
            self.assertEqual(observation[key]["sha256"], hashlib.sha256(path.read_bytes()).hexdigest())
        self.assertEqual(observation["battery_script"]["sha256"], hashlib.sha256(fixture.battery.read_bytes()).hexdigest())
        self.assertEqual(observation["invoker"]["sha256"], hashlib.sha256(SCRIPT.read_bytes()).hexdigest())
        self.assertRegex(observation["started_at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")
        self.assertLessEqual(observation["started_at"], observation["finished_at"])

    def test_battery_exit_one_with_saved_result_is_a_valid_observation(self) -> None:
        fixture = Fixture(battery_exit=1)
        done = fixture.run()
        self.assertEqual(done.returncode, 0, done.stderr)
        observation = json.loads((fixture.out_dir / "observation.json").read_text())
        self.assertEqual(observation["battery_exit"], 1)
        self.assertTrue((fixture.out_dir / "evaluation-1.json").is_file())

    def test_commands_use_the_given_binary_and_a_fresh_save_path(self) -> None:
        fixture = Fixture()
        self.assertEqual(fixture.run().returncode, 0)
        plan, evaluate = fixture.binary_calls()
        self.assertEqual(plan[0], "plan")
        self.assertIn("--out", plan)
        self.assertEqual(evaluate[0], "evaluate")
        for flag in ("--profile", "--subject", "--run-plan", "--battery", "--observation", "--evaluated-at", "--out"):
            self.assertIn(flag, evaluate)
        (battery,) = fixture.battery_calls()
        self.assertFalse(battery["save_existed"])
        self.assertIn("--quiet", battery["argv"])
        self.assertIn("--plan", battery["argv"])

    def test_two_pairs_make_one_plan_one_battery_and_two_evaluations(self) -> None:
        fixture = Fixture()
        done = fixture.run(pairs=2)
        self.assertEqual(done.returncode, 0, done.stderr)
        plan, first, second = fixture.binary_calls()
        self.assertEqual(plan.count("--profile"), 2)
        self.assertEqual(len(fixture.battery_calls()), 1)
        self.assertNotEqual(first[first.index("--out") + 1], second[second.index("--out") + 1])
        self.assertTrue((fixture.out_dir / "evaluation-2.json").is_file())

    def test_evaluated_at_can_be_supplied(self) -> None:
        fixture = Fixture()
        self.assertEqual(fixture.run(extra=("--evaluated-at", "2026-10-22T03:00:00Z")).returncode, 0)
        evaluate = fixture.binary_calls()[1]
        self.assertEqual(evaluate[evaluate.index("--evaluated-at") + 1], "2026-10-22T03:00:00Z")


class ExitTwoTests(unittest.TestCase):
    def test_unequal_pair_counts(self) -> None:
        fixture = Fixture()
        command = [sys.executable, "-I", str(SCRIPT), "--binary", str(fixture.binary), "--binary-sha256", fixture.digest,
                   "--battery", str(fixture.battery), "--out-dir", str(fixture.out_dir),
                   "--profile", str(fixture.profiles[0])]
        done = subprocess.run(command, capture_output=True, text=True, check=False)
        self.assertEqual(done.returncode, 2)
        self.assertEqual(fixture.binary_calls(), [])

    def test_missing_required_argument(self) -> None:
        done = subprocess.run([sys.executable, "-I", str(SCRIPT)], capture_output=True, text=True, check=False)
        self.assertEqual(done.returncode, 2)

    def test_plan_refusal_runs_no_battery(self) -> None:
        fixture = Fixture(plan_exit=2)
        done = fixture.run()
        self.assertEqual(done.returncode, 2)
        self.assertEqual(fixture.battery_calls(), [])
        self.assertFalse((fixture.out_dir / "observation.json").exists())

    def test_existing_result_path_is_refused_before_anything_runs(self) -> None:
        fixture = Fixture()
        fixture.out_dir.mkdir()
        (fixture.out_dir / "battery-result.json").write_text("{}\n")
        done = fixture.run()
        self.assertEqual(done.returncode, 2)
        self.assertIn("refusing to overwrite", done.stderr)
        self.assertEqual(fixture.binary_calls(), [])
        self.assertEqual(fixture.battery_calls(), [])

    def test_second_run_into_the_same_directory_is_refused(self) -> None:
        fixture = Fixture()
        self.assertEqual(fixture.run().returncode, 0)
        self.assertEqual(fixture.run().returncode, 2)


class ExitThreeTests(unittest.TestCase):
    def test_evaluate_refusal_exits_three_and_keeps_the_observation(self) -> None:
        fixture = Fixture(evaluate_exit=3)
        done = fixture.run()
        self.assertEqual(done.returncode, 3)
        self.assertTrue((fixture.out_dir / "observation.json").is_file())
        self.assertFalse((fixture.out_dir / "evaluation-1.json").exists())
        self.assertEqual(json.loads(done.stdout)["evaluate_exit_codes"], [3])

    def test_evaluate_usage_error_exits_two_not_three(self) -> None:
        self.assertEqual(Fixture(evaluate_exit=2).run().returncode, 2)

    def test_evaluate_exit_zero_without_output_is_not_success(self) -> None:
        self.assertEqual(Fixture(skip_write=True).run().returncode, 2)


class ExitFourTests(unittest.TestCase):
    def assert_no_observation(self, fixture: Fixture) -> None:
        done = fixture.run()
        self.assertEqual(done.returncode, 4, done.stderr)
        self.assertFalse((fixture.out_dir / "observation.json").exists())
        self.assertEqual([call[0] for call in fixture.binary_calls()], ["plan"])

    def test_battery_exit_two(self) -> None:
        self.assert_no_observation(Fixture(battery_exit=2, write_result=False))

    def test_other_battery_exit_even_with_a_result(self) -> None:
        self.assert_no_observation(Fixture(battery_exit=3))

    def test_battery_exit_two_even_with_a_result(self) -> None:
        self.assert_no_observation(Fixture(battery_exit=2))

    def test_exit_zero_without_a_result_file(self) -> None:
        self.assert_no_observation(Fixture(write_result=False))

    def test_exit_zero_without_a_digest_on_stdout(self) -> None:
        self.assert_no_observation(Fixture(print_digest=False))


class ExitFiveTests(unittest.TestCase):
    def test_digest_mismatch_runs_nothing(self) -> None:
        fixture = Fixture()
        done = fixture.run(binary_sha256="0" * 64)
        self.assertEqual(done.returncode, 5)
        self.assertIn("binary sha256 mismatch", done.stderr)
        self.assertEqual(fixture.binary_calls(), [])
        self.assertEqual(fixture.battery_calls(), [])
        self.assertFalse(fixture.out_dir.exists())

    def test_binary_changed_after_pinning(self) -> None:
        fixture = Fixture()
        fixture.binary.write_text(fixture.binary.read_text() + "# changed\n")
        self.assertEqual(fixture.run().returncode, 5)
        self.assertEqual(fixture.binary_calls(), [])


class StaticTests(unittest.TestCase):
    def test_standard_library_only_and_no_build(self) -> None:
        tree = ast.parse(SCRIPT.read_text())
        modules = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                modules.update(alias.name.split(".")[0] for alias in node.names)
            elif isinstance(node, ast.ImportFrom) and node.module:
                modules.add(node.module.split(".")[0])
        self.assertLessEqual(modules, set(sys.stdlib_module_names))
        self.assertNotIn("cargo", SCRIPT.read_text().lower())


if __name__ == "__main__":
    unittest.main()
