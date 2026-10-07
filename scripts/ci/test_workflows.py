import json
from pathlib import Path
import subprocess
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]


def workflow(name):
    # BaseLoader preserves `on` as text (YAML 1.1 SafeLoader treats it as true)
    # and does not construct executable Python objects.
    return yaml.load((ROOT / ".github/workflows" / name).read_text(), Loader=yaml.BaseLoader)


class WorkflowTests(unittest.TestCase):
    def test_single_owner_for_desktop_validation_per_event(self):
        release = workflow("release.yml")
        ui = workflow("ui-tests.yml")
        cursor = workflow("cursor-compatibility.yml")
        self.assertEqual(release["on"]["pull_request"]["branches"], ["main"])
        self.assertIn("push", release["on"])
        self.assertNotIn("push", ui["on"])
        self.assertIn("pull_request", ui["on"])
        self.assertEqual(ui["on"]["pull_request"]["branches-ignore"], ["main"])
        self.assertIn("workflow_call", ui["on"])
        self.assertNotIn("pull_request", cursor["on"])
        self.assertEqual(release["jobs"]["validation"]["uses"], "./.github/workflows/ui-tests.yml")
        self.assertEqual(ui["jobs"]["compatibility"]["uses"], "./.github/workflows/cursor-compatibility.yml")
        self.assertEqual(release["jobs"]["compatibility"]["uses"], "./.github/workflows/cursor-compatibility.yml")
        self.assertEqual(release["jobs"]["validation"]["with"]["skip-compatibility"], "true")
        self.assertIn("!inputs.skip-compatibility", ui["jobs"]["compatibility"]["if"])
        self.assertIn("vendor/**", ui["on"]["pull_request"]["paths"])
        self.assertEqual(set(ui["jobs"]), {
            "changes", "compatibility", "session-sync-regressions", "engine-integration",
            "macos-engine-integration", "ui-tests",
            "app-tests", "macos-ui", "macos-browser", "macos-frame-recovery",
            "linux-browser", "chromium-browser", "ios-tests",
        })

    def test_native_routing_pushes_renames_and_incomplete_pr_diffs(self):
        script = workflow("ui-tests.yml")["jobs"]["changes"]["steps"][0]["with"]["script"]
        program = """
          const input = JSON.parse(require('fs').readFileSync(0, 'utf8'));
          const outputs = {};
          const core = {setOutput: (k, v) => outputs[k] = v};
          const context = {eventName: input.event, repo: {owner:'owner', repo:'repo'},
            payload: {pull_request: {number:1, changed_files:input.total, base:{ref:input.base}}}};
          const github = {rest:{pulls:{listFiles:()=>{}}}, paginate:async()=>input.files};
          async function route() {
        """ + script + """
          }
          route().then(()=>console.log(JSON.stringify(outputs))).catch(e=>{console.error(e);process.exit(1)});
        """
        for event, base, files, total, native, ios in (
            ("push", "main", [], 0, "true", "false"),
            ("workflow_dispatch", "main", [], 0, "true", "false"),
            ("pull_request", "dev", [{"filename": "apps/ios/Zeron/Test.swift"}], 1, "false", "false"),
            ("pull_request", "main", [{"filename": "apps/ios/Zeron/Test.swift"}], 1, "false", "true"),
            ("pull_request", "main", [{"filename": "crates/engine/src/lib.rs"}], 1, "true", "false"),
            ("pull_request", "dev", [{"filename": "crates/engine/src/lib.rs"}], 1, "true", "false"),
            ("pull_request", "dev", [{"filename": "apps/ios/moved.rs",
                                     "previous_filename": "crates/engine/src/old.rs"}], 1, "true", "false"),
            ("pull_request", "dev", [{"filename": "apps/ios/Test.swift"}], 2, "true", "false"),
            ("pull_request", "main", [{"filename": "apps/ios/Test.swift"}], 2, "true", "true"),
            ("pull_request", "main", [{"filename": ".github/actions/ios-tests/action.yml"}], 1, "true", "true"),
        ):
            with self.subTest(event=event, base=base, files=files, total=total):
                result = subprocess.run(["node", "-e", program], check=True, text=True, capture_output=True,
                                        input=json.dumps({"event": event, "base": base, "files": files, "total": total}))
                self.assertEqual(json.loads(result.stdout), {"native": native, "ios": ios})

    def test_mobile_validation_is_independent_but_mandatory_for_testflight(self):
        ui = workflow("ui-tests.yml")["jobs"]
        self.assertIn("github.event_name == 'pull_request'", ui["ios-tests"]["if"])
        self.assertEqual(ui["ios-tests"]["steps"][1]["uses"], "./.github/actions/ios-tests")
        ios = workflow("ios-tests.yml")
        for event in ("pull_request", "push"):
            self.assertIn("apps/ios/**", ios["on"][event]["paths"])
            self.assertNotIn(".github/workflows/ui-tests.yml", ios["on"][event]["paths"])
        self.assertEqual(ios["on"]["pull_request"]["branches-ignore"], ["main"])
        self.assertIn("workflow_call", ios["on"])
        self.assertEqual(ios["jobs"]["ios-tests"]["steps"][1]["uses"], "./.github/actions/ios-tests")
        action = yaml.load((ROOT / ".github/actions/ios-tests/action.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertIn("-only-testing:ZeronTests", str(action["runs"]["steps"]))
        testflight = workflow("testflight.yml")["jobs"]
        self.assertEqual(testflight["upload"]["needs"], "validation")
        self.assertEqual(testflight["validation"]["uses"], "./.github/workflows/ios-tests.yml")

    def test_main_protected_check_names_are_preserved(self):
        release = workflow("release.yml")["jobs"]
        ui = workflow("ui-tests.yml")["jobs"]
        cursor = workflow("cursor-compatibility.yml")["jobs"]
        checks = {job.get("name", name) for name, job in release.items()}
        for caller, callee in (("validation", ui), ("compatibility", cursor)):
            prefix = release[caller].get("name", caller)
            checks.update(f"{prefix} / {job.get('name', name)}" for name, job in callee.items())
        self.assertTrue({
            "validation / ui-tests", "validation / session-sync-regressions",
            "validation / ios-tests", "compatibility / cursor-compatibility",
            "edge-validation", "prepare",
        }.issubset(checks))

    def test_publish_still_waits_for_complete_desktop_validation(self):
        release = workflow("release.yml")
        self.assertEqual(set(release["jobs"]["publish"]["needs"]),
                         {"prepare", "linux", "macos", "validation", "compatibility", "edge-validation"})
        self.assertEqual(release["jobs"]["publish"]["concurrency"]["cancel-in-progress"], "false")
        self.assertNotIn("concurrency", release, "Never cancel an in-flight publisher")
        for job in release["jobs"].values():
            self.assertNotIn("continue-on-error", job)
        for job in workflow("ui-tests.yml")["jobs"].values():
            self.assertNotIn("continue-on-error", job)
        publisher = (ROOT / "scripts/noches-release.py").read_text()
        self.assertIn("if not branch_is_current(repo, branch, commit):", publisher)

    def test_only_obsolete_dev_builds_are_cancelled(self):
        release = workflow("release.yml")
        for name in ("linux", "macos"):
            concurrency = release["jobs"][name]["concurrency"]
            self.assertEqual(concurrency["cancel-in-progress"],
                             "${{ github.event_name == 'push' && github.ref == 'refs/heads/dev' }}")
            self.assertIn("inputs.migration", concurrency["group"])
            self.assertIn("github.event_name", concurrency["group"],
                          "Dev pushes must not cancel manually requested package builds")
        self.assertEqual(release["jobs"]["linux"]["strategy"]["fail-fast"], "false")
        for job in workflow("ui-tests.yml")["jobs"].values():
            if "runs-on" not in job or "concurrency" not in job:
                continue
            self.assertEqual(job["concurrency"]["cancel-in-progress"],
                             "${{ github.event_name == 'push' && github.ref == 'refs/heads/dev' }}")
            self.assertIn("github.event_name", job["concurrency"]["group"],
                          "Manual validation must not share a cancellable push group")

    def test_every_engine_integration_target_runs_on_linux_and_macos(self):
        jobs = workflow("ui-tests.yml")["jobs"]
        command = "cargo test --locked --no-fail-fast -p zeron-engine --tests"
        for name, runner in (("engine-integration", "ubuntu-24.04"),
                             ("macos-engine-integration", "macos-latest")):
            job = jobs[name]
            self.assertEqual(job["runs-on"], runner)
            self.assertIn("needs.changes.outputs.native == 'true'", job["if"])
            self.assertIn(command, [step.get("run") for step in job["steps"]])
        # Only the paid macOS leg skips fork PRs; Linux always validates them.
        self.assertIn("head.repo.full_name == github.repository", jobs["macos-engine-integration"]["if"])
        self.assertNotIn("head.repo.full_name", jobs["engine-integration"]["if"])

    def test_sync_gate_and_cache_warming_remain_explicit(self):
        ui = workflow("ui-tests.yml")
        job = ui["jobs"]["session-sync-regressions"]
        self.assertEqual(job["timeout-minutes"], "40")
        self.assertTrue(any(step.get("run") == "python3 scripts/ci/run-session-sync.py"
                            for step in job["steps"]))
        caches = [step["with"] for job in ui["jobs"].values() for step in job.get("steps", [])
                  if step.get("uses", "").startswith("Swatinem/rust-cache@")]
        for cache in caches:
            save = cache.get("save-if")
            if save != "false":
                self.assertIn("head.repo.full_name == github.repository", save)
        # Workflow policy changes cannot bypass their own lightweight checks.
        policy = workflow("ci-policy.yml")
        self.assertIn("pull_request", policy["on"])
        self.assertIn("scripts/ci/**", policy["on"]["push"]["paths"])


if __name__ == "__main__":
    unittest.main()
