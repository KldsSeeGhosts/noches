#!/usr/bin/env python3
"""Publish immutable builds, permanent epoch-1 feeds, and gated one-time legacy bridges."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

EPOCH = 1
TARGETS = ("linux-x86_64.tar.gz", "linux-aarch64.tar.gz",
           "macos-arm64.dmg", "macos-arm64-app.tar.gz")


def display_version():
    """The intentionally bumped product version, not the CI build identity."""
    manifest = (Path(__file__).resolve().parent.parent / "Cargo.toml").read_text()
    section = manifest.split("[workspace.package]\n", 1)[-1].split("\n[", 1)[0]
    match = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"$', section, re.M)
    if not match:
        raise ValueError("Workspace product version must be plain numeric SemVer")
    return match[1]


def build_identity(branch, run, attempt, mode="normal", channel=None):
    if branch not in ("dev", "main") or int(run) <= 0 or int(attempt) <= 0:
        raise ValueError("Releases require dev/main and positive build numbers")
    if mode == "normal":
        if channel:
            raise ValueError("Ordinary releases cannot override the branch channel")
        channel = "stable" if branch == "main" else "dev"
        series = "0.1"
    elif mode in ("seed", "bridge"):
        if branch != "dev" or channel not in ("dev", "stable"):
            raise ValueError("Manual migration builds must originate on dev with an explicit channel")
        series = "0.1" if mode == "seed" else "0.4"
    else:
        raise ValueError("Invalid release mode")
    version = f"{series}.{int(run)}"
    if channel == "dev":
        version += f"-dev.{int(attempt)}"
    return channel, version


def manifest_for(directory, repository, channel, version, commit, epoch=EPOCH):
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Invalid GitHub repository")
    if channel not in ("dev", "stable") or not re.fullmatch(r"0\.[14]\.\d+(?:-dev\.\d+)?", version):
        raise ValueError("Invalid release identity")
    if (channel == "dev") != ("-dev." in version):
        raise ValueError("Version does not match channel")
    required = [f"noches-{version}-{target}" for target in TARGETS]
    if not all((directory / name).is_file() for name in required):
        raise ValueError("Cannot publish an incomplete platform release")
    root = f"https://github.com/{repository}/releases"
    return dict(product="noches", channel=channel, version=version,
                display_version=display_version(), epoch=epoch, commit=commit,
                notes_url=f"{root}/tag/v{version}", files={
                    name: dict(sha256=hashlib.sha256((directory / name).read_bytes()).hexdigest(),
                               size=(directory / name).stat().st_size,
                               url=f"{root}/download/v{version}/{name}") for name in required})


def gh(*args, **kwargs):
    return subprocess.check_output(["gh", *args], text=True, **kwargs).strip()


def release_for(repo, tag):
    try:
        return json.loads(gh("api", f"repos/{repo}/releases/tags/{tag}", stderr=subprocess.PIPE))
    except subprocess.CalledProcessError as error:
        if "404" in error.stderr:
            return None
        raise


def version_order(version):
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-dev\.(\d+))?", version)
    if not match:
        raise ValueError("Invalid channel version")
    major, minor, patch, dev = match.groups()
    # prereleases sort below the corresponding stable version
    return (int(major), int(minor), int(patch), 0 if dev else 1, int(dev or 0))


def feed_manifest(repo, tag, *, required=False, recover_missing=False):
    entry = release_for(repo, tag)
    if not entry:
        if required:
            raise ValueError(f"Required feed {tag} does not exist")
        return None, None
    if entry.get("draft"):
        raise ValueError(f"Feed {tag} is still a draft")
    if not any(a["name"] == "manifest.json" for a in entry["assets"]):
        # GitHub's --clobber deletes the previous asset before uploading the
        # replacement. Only the current branch-tip ordinary publisher may
        # repair a missing epoch-1 pointer from a complete version release.
        if recover_missing:
            return entry, None
        raise ValueError(f"Feed {tag} is missing its manifest; refusing replacement")
    return entry, json.loads(gh("release", "download", tag, "--repo", repo,
                                "--pattern", "manifest.json", "--output", "-"))


def check_feed(manifest, channel, series, epoch):
    if (manifest.get("product"), manifest.get("channel"), manifest.get("epoch", 0)) != ("noches", channel, epoch):
        raise ValueError("Unexpected product, channel or epoch in existing feed")
    version = manifest["version"]
    if not version.startswith(series) or (channel == "dev") != ("-dev." in version):
        raise ValueError("Unexpected version series in existing feed")
    version_order(version)
    if not manifest.get("files") or not all(
        file.get("url", "").startswith(f"https://github.com/") and
        file.get("sha256") and file.get("size") is not None
        for file in manifest["files"].values()
    ):
        raise ValueError("Feed has incomplete artifact metadata")


def branch_is_current(repo, branch, commit):
    return gh("api", f"repos/{repo}/git/ref/heads/{branch}", "--jq", ".object.sha") == commit


def publish(directory):
    repo = os.environ["GITHUB_REPOSITORY"]
    channel = os.environ["NOCHES_CHANNEL"]
    version = os.environ["NOCHES_VERSION"]
    commit = os.environ["GITHUB_SHA"]
    branch = os.environ["GITHUB_REF_NAME"]
    mode = os.environ.get("NOCHES_RELEASE_MODE", "normal")
    # A failed-job rerun retains prepare's outputs and successful packages.
    # Its workflow attempt increments, but the packages' identity must not.
    attempt = os.environ.get("NOCHES_BUILD_ATTEMPT", os.environ["GITHUB_RUN_ATTEMPT"])
    if not 0 < int(attempt) <= int(os.environ["GITHUB_RUN_ATTEMPT"]):
        raise ValueError("Invalid prepared build attempt")
    if (channel, version) != build_identity(branch, os.environ["GITHUB_RUN_NUMBER"],
                                            attempt, mode,
                                            channel if mode != "normal" else None):
        raise ValueError("Release identity differs from the workflow run")
    if mode != "normal" and os.environ.get("NOCHES_MIGRATION_CONFIRM") != f"{mode}-{channel}":
        raise ValueError("Manual migration requires the exact confirmation phrase")
    if not branch_is_current(repo, branch, commit):
        raise ValueError("Branch advanced during build; start a new run")

    feed = f"noches-epoch1-{channel}"
    feed_release, current = feed_manifest(repo, feed, recover_missing=mode != "bridge")
    if current:
        check_feed(current, channel, "0.1.", EPOCH)
    if mode == "bridge":
        # This check must precede *any* write: otherwise legacy clients can land
        # on a bridge that has no destination feed.
        if not current:
            raise ValueError("Publish an epoch-1 seed for this channel before the bridge")
        legacy = f"noches-{channel}"
        _, old = feed_manifest(repo, legacy, required=True)
        check_feed(old, channel, "0.3.", 0)
        if version_order(version) <= version_order(old["version"]):
            raise ValueError("Bridge version must outrank the legacy release")
    else:
        legacy = None
        if current and version_order(current["version"]) > version_order(version):
            raise ValueError("Newer epoch-1 feed already published")

    manifest = manifest_for(directory, repo, channel, version, commit,
                            epoch=EPOCH if mode != "bridge" else 0)
    path = directory / "manifest.json"
    path.write_text(json.dumps(manifest, indent=2) + "\n")
    tag = f"v{version}"
    existing = release_for(repo, tag)
    if existing:
        # Never clobber a published release or resume an untrusted draft.
        if existing.get("draft"):
            raise ValueError(f"Version tag {tag} already has a draft; resolve it manually")
        saved = json.loads(gh("release", "download", tag, "--repo", repo,
                              "--pattern", "manifest.json", "--output", "-"))
        if saved != manifest or not all(name in {a["name"] for a in existing["assets"]}
                                       for name in manifest["files"]):
            raise ValueError(f"Version tag {tag} is already published with different assets")
    else:
        files = [str(directory / name) for name in manifest["files"]] + [str(path)]
        gh("release", "create", tag, "--repo", repo, "--target", commit, "--draft", "--title",
           f"Noches {'Dev ' if channel == 'dev' else ''}{display_version()} ({version})", "--generate-notes")
        gh("release", "upload", tag, *files, "--repo", repo)  # no --clobber
        gh("release", "edit", tag, "--repo", repo, "--draft=false",
           f"--prerelease={'true' if channel == 'dev' or mode == 'bridge' else 'false'}",
           f"--latest={'true' if channel == 'stable' and mode == 'normal' else 'false'}")
    if not branch_is_current(repo, branch, commit):
        raise ValueError("Branch advanced before feed publication")
    if mode == "bridge":
        # Already-installed 0.1.x clients that still read the legacy URL must
        # accept this 0.4.x hop too; old 0.3.x clients ignore the epoch field.
        manifest["epoch"] = EPOCH
        path.write_text(json.dumps(manifest, indent=2) + "\n")
        _, latest_old = feed_manifest(repo, legacy, required=True)
        if latest_old != old:
            raise ValueError("Legacy feed changed since preflight")
        gh("release", "upload", legacy, str(path), "--repo", repo, "--clobber")
    else:
        _, latest = feed_manifest(repo, feed, recover_missing=True)
        if latest:
            check_feed(latest, channel, "0.1.", EPOCH)
            if version_order(latest["version"]) > version_order(version):
                raise ValueError("Newer epoch-1 feed published during build")
        else:
            gh("release", "create", feed, "--repo", repo, "--target", commit,
               "--prerelease", "--latest=false", "--title", f"Noches epoch-1 {channel} feed",
               "--notes", "Permanent epoch-1 update metadata; installers are in versioned releases.")
        if latest:
            gh("release", "upload", feed, str(path), "--repo", repo, "--clobber")
        else:
            gh("release", "upload", feed, str(path), "--repo", repo)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["prepare", "publish"])
    parser.add_argument("--directory", type=Path, default=Path("artifacts"))
    args = parser.parse_args()
    if args.action == "prepare":
        branch = os.environ.get("NOCHES_SOURCE_BRANCH", os.environ["GITHUB_REF_NAME"])
        channel, version = build_identity(branch, os.environ["GITHUB_RUN_NUMBER"],
                                          os.environ["GITHUB_RUN_ATTEMPT"],
                                          os.environ.get("NOCHES_RELEASE_MODE", "normal"),
                                          os.environ.get("NOCHES_MIGRATION_CHANNEL") or None)
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"channel={channel}\nversion={version}\n"
                         f"build_attempt={int(os.environ['GITHUB_RUN_ATTEMPT'])}\n")
    else:
        publish(args.directory)


if __name__ == "__main__":
    main()
