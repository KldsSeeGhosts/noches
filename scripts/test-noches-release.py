#!/usr/bin/env python3
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
# Loading the module must not drop __pycache__ into the worktree; build scripts
# refuse to build from a dirty one.
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("release", ROOT / "scripts/noches-release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def test_branch_and_ordering(self):
        self.assertRegex(release.display_version(), r'^\d+\.\d+\.\d+$')
        self.assertEqual(release.build_identity('main', 12, 1), ('stable', '0.1.12'))
        self.assertEqual(release.build_identity('dev', 12, 2), ('dev', '0.1.12-dev.2'))
        with self.assertRaises(ValueError): release.build_identity('feature/test', 12, 1)
        self.assertGreater(release.version_order('0.3.12-dev.10'), release.version_order('0.3.12-dev.9'))
        self.assertGreater((release.EPOCH, release.version_order('0.1.1')), (0, release.version_order('0.3.35-dev.1')))
        self.assertLess(release.version_order('0.1.12-dev.2'), release.version_order('0.1.12'))
        self.assertEqual(release.build_identity('dev', 12, 1, 'bridge', 'stable'), ('stable', '0.4.12'))
        self.assertEqual(release.build_identity('dev', 12, 1, 'seed', 'stable'), ('stable', '0.1.12'))
        with self.assertRaises(ValueError): release.build_identity('main', 12, 1, 'bridge', 'stable')
        with self.assertRaises(ValueError): release.build_identity('dev', 12, 1, 'normal', 'stable')

    def test_pr_prepare_uses_explicit_target_branch(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'output'
            env = dict(os.environ, GITHUB_REF_NAME='10/merge', NOCHES_SOURCE_BRANCH='dev',
                       GITHUB_RUN_NUMBER='12', GITHUB_RUN_ATTEMPT='1', GITHUB_OUTPUT=str(output))
            subprocess.run(['python3', str(ROOT / 'scripts/noches-release.py'), 'prepare'], env=env, check=True)
            self.assertEqual(output.read_text(), 'channel=dev\nversion=0.1.12-dev.1\n')

    def test_complete_manifest_uses_immutable_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            with self.assertRaises(ValueError): release.manifest_for(directory, 'owner/noches', 'dev', '0.4.1-dev.1', 'abc')
            for target in release.TARGETS:
                (directory / f'noches-0.4.1-dev.1-{target}').write_bytes(b'fixture')
            manifest = release.manifest_for(directory, 'owner/noches', 'dev', '0.4.1-dev.1', 'abc', epoch=0)
            self.assertEqual(manifest['epoch'], 0)
            self.assertEqual(manifest['display_version'], release.display_version())
            self.assertEqual(manifest['version'], '0.4.1-dev.1')
            for artifact in manifest['files'].values():
                self.assertIn('/download/v0.4.1-dev.1/', artifact['url'])
                self.assertEqual(artifact['size'], 7)
                self.assertEqual(len(artifact['sha256']), 64)

    def test_bridge_publication_requires_seed_and_pins_legacy(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            for target in release.TARGETS:
                (directory / f'noches-0.4.42-dev.1-{target}').write_bytes(b'bridge')
            legacy = dict(product='noches', channel='dev', version='0.3.35-dev.1',
                          files={'old': dict(url='https://github.com/owner/noches/releases/download/v0.3.35-dev.1/old',
                                             sha256='abc', size=1)})
            feeds = {'noches-dev': legacy}
            releases = {'noches-dev': dict(draft=False, assets=[dict(name='manifest.json')])}
            writes = []

            def fake_release(repo, tag):
                return releases.get(tag)

            def fake_gh(*args, **kwargs):
                if args[:2] == ('api', 'repos/owner/noches/git/ref/heads/dev'):
                    return 'abc123'
                if args[:2] == ('release', 'download'):
                    return json.dumps(feeds[args[2]])
                if args[:2] == ('release', 'create'):
                    releases[args[2]] = dict(draft=True, assets=[])
                if args[:2] == ('release', 'upload'):
                    tag = args[2]
                    feeds[tag] = json.loads((directory / 'manifest.json').read_text())
                    releases[tag]['assets'] = [dict(name='manifest.json')] + [dict(name=n) for n in feeds[tag]['files']]
                    writes.append(tag)
                if args[:2] == ('release', 'edit'):
                    releases[args[2]]['draft'] = False
                return ''

            env = dict(GITHUB_REPOSITORY='owner/noches', GITHUB_REF_NAME='dev',
                       GITHUB_SHA='abc123', GITHUB_RUN_NUMBER='42', GITHUB_RUN_ATTEMPT='1',
                       NOCHES_CHANNEL='dev', NOCHES_VERSION='0.4.42-dev.1',
                       NOCHES_RELEASE_MODE='bridge', NOCHES_MIGRATION_CONFIRM='bridge-dev')
            with patch.dict(os.environ, env), patch.object(release, 'gh', fake_gh), patch.object(release, 'release_for', fake_release):
                with self.assertRaisesRegex(ValueError, 'seed'):
                    release.publish(directory)
                self.assertEqual(writes, [])
                feeds['noches-epoch1-dev'] = dict(legacy, version='0.1.41-dev.1', epoch=1)
                releases['noches-epoch1-dev'] = dict(draft=False, assets=[dict(name='manifest.json')])
                release.publish(directory)
                self.assertEqual(writes, ['v0.4.42-dev.1', 'noches-dev'])
                self.assertEqual(feeds['v0.4.42-dev.1']['epoch'], 0)
                self.assertEqual(feeds['noches-dev']['epoch'], 1)
                self.assertEqual(feeds['noches-epoch1-dev']['version'], '0.1.41-dev.1')
                with self.assertRaisesRegex(ValueError, 'epoch'):
                    release.publish(directory)
                self.assertEqual(writes, ['v0.4.42-dev.1', 'noches-dev'])

    def test_missing_epoch1_manifest_recovers_from_published_branch_tip(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            version = '0.1.42-dev.1'
            for target in release.TARGETS:
                (directory / f'noches-{version}-{target}').write_bytes(b'complete')
            manifest = release.manifest_for(directory, 'owner/noches', 'dev', version, 'abc123')
            version_tag = f'v{version}'
            feeds = {version_tag: manifest}
            releases = {
                version_tag: dict(draft=False, assets=[dict(name='manifest.json')] +
                                  [dict(name=name) for name in manifest['files']]),
                # A failed --clobber deleted the old feed asset; only the feed
                # pointer is missing, not its immutable version release.
                'noches-epoch1-dev': dict(draft=False, assets=[]),
            }
            writes = []

            def fake_gh(*args, **kwargs):
                if args[:2] == ('api', 'repos/owner/noches/git/ref/heads/dev'):
                    return 'abc123'
                if args[:2] == ('release', 'download'):
                    return json.dumps(feeds[args[2]])
                if args[:2] == ('release', 'upload'):
                    self.assertNotIn('--clobber', args)
                    writes.append(args[2])
                    feeds[args[2]] = json.loads((directory / 'manifest.json').read_text())
                    releases[args[2]]['assets'] = [dict(name='manifest.json')]
                return ''

            env = dict(GITHUB_REPOSITORY='owner/noches', GITHUB_REF_NAME='dev',
                       GITHUB_SHA='abc123', GITHUB_RUN_NUMBER='42', GITHUB_RUN_ATTEMPT='1',
                       NOCHES_CHANNEL='dev', NOCHES_VERSION=version, NOCHES_RELEASE_MODE='normal')
            with patch.dict(os.environ, env), patch.object(release, 'gh', fake_gh), \
                 patch.object(release, 'release_for', lambda repo, tag: releases.get(tag)):
                release.publish(directory)
            self.assertEqual(writes, ['noches-epoch1-dev'])
            self.assertEqual(feeds['noches-epoch1-dev'], manifest)

    def test_bridge_requires_confirmation_and_rejects_existing_version(self):
        with tempfile.TemporaryDirectory() as tmp:
            env = dict(GITHUB_REPOSITORY='owner/noches', GITHUB_REF_NAME='dev',
                       GITHUB_SHA='abc', GITHUB_RUN_NUMBER='2', GITHUB_RUN_ATTEMPT='1',
                       NOCHES_CHANNEL='stable', NOCHES_VERSION='0.4.2',
                       NOCHES_RELEASE_MODE='bridge', NOCHES_MIGRATION_CONFIRM='')
            with patch.dict(os.environ, env):
                with self.assertRaisesRegex(ValueError, 'confirmation'):
                    release.publish(Path(tmp))

    @unittest.skipUnless(os.uname().sysname == 'Linux', 'Linux installer')
    def test_installer_keeps_channels_and_previous_versions_separate(self):
        with tempfile.TemporaryDirectory(prefix='noches install ') as tmp:
            root = Path(tmp)
            home = root / 'home with spaces'
            home.mkdir()
            env = dict(os.environ, HOME=str(home))
            for slug, channel, version in [('noches', 'stable', '0.3.1'), ('noches-dev', 'dev', '0.3.2-dev.1'), ('noches', 'stable', '0.3.3')]:
                package = root / (slug + version)
                package.mkdir()
                (package / 'zeron').write_text('#!/bin/sh\nprintf "%s" ' + version)
                (package / 'zeron').chmod(0o755)
                (package / 'browser').mkdir()
                (package / 'browser/noches-chromium').write_text(version)
                (package / 'browser/resources.pak').write_bytes(b'chromium resources')
                (package / f'{slug}.desktop').write_text('[Desktop Entry]\nType=Application\nName=Noches\nExec=zeron %u\nTryExec=zeron\n')
                (package / f'{slug}.png').write_bytes(b'icon')
                (package / 'install.json').write_text(json.dumps(dict(slug=slug, channel=channel, version=version)))
                shutil.copy(ROOT / 'scripts/install-linux.sh', package / 'install.sh')
                subprocess.run(['bash', str(package / 'install.sh')], env=env, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            self.assertEqual(subprocess.check_output([home / '.local/bin/noches'], text=True), '0.3.3')
            self.assertEqual(subprocess.check_output([home / '.local/bin/noches-dev'], text=True), '0.3.2-dev.1')
            self.assertEqual((home / '.local/share/noches/app/previous').resolve().name, '0.3.1')
            for slug, link, version in [('noches', 'current', '0.3.3'), ('noches', 'previous', '0.3.1'), ('noches-dev', 'current', '0.3.2-dev.1')]:
                browser = home / '.local/share' / slug / 'app' / link / 'browser'
                self.assertEqual((browser / 'noches-chromium').read_text(), version)
                self.assertEqual((browser / 'resources.pak').read_bytes(), b'chromium resources')
            desktop = (home / '.local/share/applications/noches.desktop').read_text()
            self.assertIn(f'Exec="{home}/.local/share/noches/app/current/zeron" %u', desktop)

    def test_linux_input_repair_is_in_the_shared_tree(self):
        cargo = (ROOT / 'Cargo.toml').read_text()
        self.assertIn('[patch."https://github.com/zeronsh/zui"]', cargo)
        self.assertIn('gpui_linux = { path = "vendor/gpui_linux" }', cargo)
        client = (ROOT / 'vendor/gpui_linux/src/linux/wayland/client.rs').read_text()
        self.assertIn('mod agent_seat;', client)
        self.assertIn('noches_seats.insert', client)
        self.assertNotIn('state.wl_seat.release()', client)
        for name in ('agent_seat.rs', 'seat_selection.rs', 'zui_seat_runtime.rs'):
            self.assertTrue((ROOT / 'vendor/gpui_linux/src/linux/wayland' / name).is_file())

    def test_cua_driver_patches_ship_with_the_app_bridge(self):
        script = (ROOT / 'scripts/cua/build_native.sh').read_text()
        self.assertNotIn('branch --show-current', script)
        self.assertNotIn('zui-c2d273d-cua', script)
        self.assertIn('.local/bin/cua-driver', script)
        self.assertTrue((ROOT / 'scripts/cua/native/apply_cua.py').is_file())
        self.assertTrue((ROOT / 'crates/harness/src/pi/noches-cua.ts').is_file())
        self.assertTrue((ROOT / 'crates/engine/src/computer_use/linux.rs').is_file())
        sessions = (ROOT / 'crates/engine/src/sessions.rs').read_text()
        self.assertIn('start_bridge', sessions)
        harness = (ROOT / 'crates/harness/src/acp/mod.rs').read_text()
        self.assertIn('noches-cua.ts', harness)
        self.assertIn('PI_ACP_PI_COMMAND', harness)
        self.assertNotIn('join("noches-cua-launch")', harness)
        self.assertNotIn('computer_use_socket.is_some()', harness)
        unsupported = (ROOT / 'crates/engine/src/computer_use/unsupported.rs').read_text()
        self.assertIn('fn turn_started', unsupported)
        self.assertIn('fn turn_ended', unsupported)


if __name__ == '__main__': unittest.main()
