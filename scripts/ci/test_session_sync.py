import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("session_sync", Path(__file__).with_name("run-session-sync.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class SessionSyncTests(unittest.TestCase):
    def binaries(self):
        return {case[:3]: f"/tmp/{case[0]}-{case[2]}" for case in runner.CASES}

    def test_one_locked_build_covers_all_required_targets(self):
        command = runner.build_command()
        self.assertEqual(command.count("cargo"), 1)
        self.assertIn("--locked", command)
        self.assertIn("--no-run", command)
        self.assertIn("--message-format=json", command)
        for package in runner.PACKAGES:
            self.assertIn(f"zeron-{package}", command)
        self.assertEqual(set(command[i + 1] for i, arg in enumerate(command) if arg == "--test"),
                         {"opencode", "t3_stdio", "e2e", "device_routing", "session_publication",
                          "restart_resume", "codex_subagents", "local_profiles"})

    def test_missing_required_target_refuses_to_run_partial_validation(self):
        binaries = self.binaries()
        del binaries[("engine", "lib", "zeron_engine")]
        with patch.object(runner.subprocess, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "every required"):
                runner.run_cases(binaries)
            run.assert_not_called()

    def test_filters_working_directories_and_failure_propagation(self):
        binaries = self.binaries()
        with patch.object(runner.subprocess, "run") as run:
            runner.run_cases(binaries)
            self.assertEqual(run.call_count, len(runner.CASES))
            for case, call in zip(runner.CASES, run.call_args_list):
                self.assertEqual(call.args[0], runner.test_command(case, binaries[case[:3]]))
                self.assertEqual(call.kwargs["cwd"], runner.ROOT / "crates" / case[0])
                self.assertTrue(call.kwargs["check"])
                self.assertNotIn("--include-ignored", call.args[0])
        with patch.object(runner.subprocess, "run",
                          side_effect=subprocess.CalledProcessError(1, ["failed-test"])) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                runner.run_cases(binaries)
            self.assertEqual(run.call_count, 1)

    def test_only_real_test_executables_from_workspace_packages_are_selected(self):
        binaries = {}
        record = {"reason": "compiler-artifact", "profile": {"test": True},
                  "manifest_path": str(runner.ROOT / "crates/engine/Cargo.toml"),
                  "target": {"name": "zeron_engine", "kind": ["lib"]},
                  "executable": "/tmp/engine-unit-tests"}
        runner.record_executable(record, binaries)
        self.assertEqual(binaries[("engine", "lib", "zeron_engine")], "/tmp/engine-unit-tests")
        runner.record_executable(dict(record, profile={"test": False}), binaries)
        runner.record_executable(dict(record, executable=None), binaries)
        runner.record_executable(dict(record, manifest_path="/other/engine/Cargo.toml"), binaries)
        self.assertEqual(len(binaries), 1)
        with self.assertRaisesRegex(RuntimeError, "Ambiguous"):
            runner.record_executable(dict(record, executable="/tmp/other-engine-tests"), binaries)


if __name__ == "__main__":
    unittest.main()
