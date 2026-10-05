#!/usr/bin/env python3
"""Build one Cargo feature graph, then run the existing sync-gate test selection.

Separate `cargo test -p ...` calls repeatedly rebuild serde/proto/doc/sync under
different feature sets. Cargo's JSON executable records let us build the union
once without broadening the gate to unrelated integration tests.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
PACKAGES = ("harness", "mcp", "engine", "sync", "update")
# (crate directory, target kind, Cargo target name, libtest filter)
CASES = (
    ("harness", "lib", "zeron_harness", "opencode::"),
    ("harness", "test", "opencode", None),
    ("mcp", "lib", "zeron_mcp", None),
    ("mcp", "bin", "t3-code", None),
    ("mcp", "test", "t3_stdio", None),
    ("engine", "test", "e2e", "start_failure_lands_in_the_transcript"),
    ("engine", "test", "device_routing", "mcp_standalone_session_executes_on_the_selected_device"),
    ("sync", "lib", "zeron_sync", None),
    ("update", "lib", "zeron_update", None),
    ("engine", "lib", "zeron_engine", None),
    ("engine", "test", "session_publication", None),
    ("engine", "test", "restart_resume", None),
    ("engine", "test", "codex_subagents", None),
    ("engine", "test", "local_profiles", None),
)


def cargo_selection():
    args = []
    for package in PACKAGES:
        args.extend(("-p", f"zeron-{package}"))
    return args


def build_command():
    args = ["cargo", "test", "--locked", "--no-run", "--message-format=json", *cargo_selection(), "--lib"]
    for _, kind, target, _ in CASES:
        if kind != "lib":
            args.extend((f"--{kind}", target))
    return args


def record_executable(record, binaries, root=ROOT):
    if record.get("reason") != "compiler-artifact" or not record.get("profile", {}).get("test"):
        return
    executable = record.get("executable")
    if not executable:
        return
    manifest = Path(record["manifest_path"]).resolve()
    for package in PACKAGES:
        if manifest != (root / "crates" / package / "Cargo.toml").resolve():
            continue
        target = record["target"]
        key = (package, target["kind"][0], target["name"])
        if key in binaries and binaries[key] != executable:
            raise RuntimeError(f"Ambiguous test executable for {key}")
        binaries[key] = executable
        break


def require_selection(binaries):
    missing = [case[:3] for case in CASES if case[:3] not in binaries]
    if missing:
        raise RuntimeError(f"Cargo did not build every required test target: {missing}")


def test_command(case, executable):
    # --include-ignored is intentionally absent: real-login/private tests stay
    # ignored exactly as with cargo test. Filters preserve the previous gate.
    return [executable] + ([case[3]] if case[3] else [])


def build():
    binaries = {}
    with subprocess.Popen(build_command(), cwd=ROOT, stdout=subprocess.PIPE, text=True) as cargo:
        for line in cargo.stdout:
            record = json.loads(line)
            if record.get("reason") == "compiler-message":
                rendered = record.get("message", {}).get("rendered")
                if rendered:
                    print(rendered, end="", file=sys.stderr, flush=True)
            record_executable(record, binaries)
        result = cargo.wait()
    if result:
        raise subprocess.CalledProcessError(result, build_command())
    require_selection(binaries)
    return binaries


def run_cases(binaries):
    require_selection(binaries)
    for case in CASES:
        package, _, target, test_filter = case
        print(f"\n=== {package}/{target} {test_filter or '(all tests)'} ===", flush=True)
        package_root = ROOT / "crates" / package
        # Cargo runs each test in its package directory, not the workspace root.
        env = dict(os.environ, CARGO_MANIFEST_DIR=str(package_root),
                   CARGO_MANIFEST_PATH=str(package_root / "Cargo.toml"))
        subprocess.run(test_command(case, binaries[case[:3]]), cwd=package_root, env=env, check=True)


def main():
    started = time.monotonic()
    binaries = build()
    built = time.monotonic()
    run_cases(binaries)
    # Keep MCP doctest coverage while retaining the same unified feature graph.
    # This also exercises the harness's compile-fail documentation regression.
    subprocess.run(["cargo", "test", "--locked", *cargo_selection(), "--doc"], cwd=ROOT, check=True)
    finished = time.monotonic()
    summary = (
        "### Session-sync validation\n\n"
        f"- Unified Cargo build: {built - started:.1f}s\n"
        f"- Selected tests and doctests: {finished - built:.1f}s\n"
        f"- Required test targets executed: {len(CASES)}\n"
    )
    print(summary, flush=True)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as output:
            output.write(summary)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError) as error:
        print(f"Session-sync validation failed: {error}", file=sys.stderr)
        sys.exit(1)
