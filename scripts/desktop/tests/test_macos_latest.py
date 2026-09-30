"""Deletion-policy regressions; only disposable fixtures are ever removed."""
import importlib.util
import os
from pathlib import Path
import plistlib
import tempfile
import unittest
import subprocess
import json
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('macos_latest', Path(__file__).parents[1] / 'macos_latest.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class PolicyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        subprocess.run(['git', 'init', '--quiet', str(self.root)], check=True)
        self.app = self.root / 'target/debug/bundle/macos/OrdinConn.app'
        self.raw = self.root / 'target/debug/ordinconn-desktop'
        self.make_app(self.app)
        self.raw.write_bytes(self.exe(self.app).read_bytes())
        self.latest_ns = max(p.stat().st_mtime_ns for p in self.app.rglob('*')) + 1_000_000

    def exe(self, app):
        return app / 'Contents/MacOS/ordinconn-desktop'

    def make_app(self, app):
        (app / 'Contents/MacOS').mkdir(parents=True)
        (app / 'Contents/Resources').mkdir()
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
            'CFBundleIdentifier': 'ai.ordinconn.desktop', 'CFBundleName': 'OrdinConn',
            'CFBundleExecutable': 'ordinconn-desktop', 'CFBundlePackageType': 'APPL',
            'CFBundleShortVersionString': '0.1.0', 'CFBundleVersion': '0.1.0'}))
        self.exe(app).write_bytes(b'\xcf\xfa\xed\xfe' + b'fixture-mach-o')
        self.exe(app).chmod(0o755)

    def approve(self, path=None, known=None):
        return m.approve_stale(path or self.app, self.root, known or set(), self.latest_ns)

    def test_same_named_verified_project_artifact_can_be_removed(self):
        digest = self.approve()
        self.assertEqual(len(digest), 64)
        m.remove_verified(self.app, digest)
        self.assertFalse(self.app.exists())
        self.assertTrue(self.raw.is_file())

    def test_unrecognized_location_is_preserved(self):
        other = self.root / 'unrelated/OrdinConn.app'
        self.make_app(other)
        with self.assertRaises(m.Blocked): self.approve(other)
        self.assertTrue(other.is_dir())

    def test_matching_name_with_wrong_identifier_is_preserved(self):
        p = self.app / 'Contents/Info.plist'
        data = plistlib.loads(p.read_bytes()); data['CFBundleIdentifier'] = 'unrelated.application'
        p.write_bytes(plistlib.dumps(data))
        with self.assertRaises(m.Blocked): self.approve()

    def test_missing_raw_build_and_receipt_is_not_proven_stale(self):
        self.raw.unlink()
        with self.assertRaises(m.Blocked): self.approve()

    def test_different_raw_executable_is_not_proven_stale(self):
        self.raw.write_bytes(b'another-build')
        with self.assertRaises(m.Blocked): self.approve()

    def test_receipt_can_prove_a_previously_signed_project_bundle(self):
        digest = m.bundle_digest(self.app)
        self.raw.write_bytes(b'newer-raw-build')
        self.assertEqual(self.approve(known={digest}), digest)

    def test_timestamp_alone_never_proves_origin(self):
        self.raw.unlink()
        for p in self.app.rglob('*'): os.utime(p, ns=(1, 1))
        with self.assertRaises(m.Blocked): self.approve()

    def test_newer_bundle_is_preserved(self):
        self.latest_ns = 1
        with self.assertRaises(m.Blocked): self.approve()

    def test_symlink_bundle_is_preserved(self):
        link = self.root / 'linked.app'; link.symlink_to(self.app, target_is_directory=True)
        with self.assertRaises(m.Blocked): self.approve(link)
        self.assertTrue(self.app.exists())

    def test_symlink_inside_bundle_blocks_cleanup(self):
        (self.app / 'Contents/Resources/linked').symlink_to(self.raw)
        with self.assertRaises(m.Blocked): self.approve()

    def test_symlink_ancestor_blocks_cleanup(self):
        actual = self.root / 'actual-target'
        (self.root / 'target').rename(actual)
        (self.root / 'target').symlink_to(actual, target_is_directory=True)
        with self.assertRaises(m.Blocked): self.approve()

    def test_user_database_inside_app_blocks_cleanup(self):
        (self.app / 'Contents/Resources/user.sqlite3').write_bytes(b'protected')
        with self.assertRaises(m.Blocked): self.approve()

    def test_git_tracked_files_inside_build_bundle_are_preserved(self):
        subprocess.run(['git', 'add', '--force', str(self.app)], cwd=self.root, check=True)
        with self.assertRaises(m.Blocked): self.approve()

    def test_unknown_resource_without_build_receipt_is_preserved(self):
        (self.app / 'Contents/Resources/notes.txt').write_text('owner notes')
        self.latest_ns += 1_000_000_000
        with self.assertRaises(m.Blocked): self.approve()

    def test_unexpected_top_level_payload_blocks_cleanup(self):
        (self.app / 'screenshots').mkdir()
        with self.assertRaises(m.Blocked): self.approve()

    def test_changed_bundle_between_review_and_removal_is_preserved(self):
        digest = self.approve()
        self.exe(self.app).write_bytes(b'changed-after-review')
        with self.assertRaises(m.Blocked): m.remove_verified(self.app, digest)
        self.assertTrue(self.app.exists())

    def test_already_removed_registration_does_not_fail_cleanup(self):
        missing = self.root / 'already-removed/OrdinConn.app'
        with patch.object(m, 'ls_paths', return_value=set()), patch.object(m, 'run', side_effect=AssertionError('must not unregister an absent entry twice')):
            m.unregister_missing({missing})
        self.assertTrue(self.app.exists())

    def test_identical_bundle_rebuilds_keep_distinct_provenance_records(self):
        digest = m.bundle_digest(self.app)
        directory = m.receipt_directory(self.root)
        directory.mkdir()
        for stamp in ['1000', '2000']:
            (directory / f'{digest}.{stamp}.json').write_text(json.dumps({'repository': m.REPOSITORY, 'bundle_digest': digest}))
        self.assertEqual(m.known_receipts(self.root), {digest})
        self.assertEqual(len(list(directory.glob('*.json'))), 2)

    def freeze_marker(self, contents=None):
        marker = self.root / 'target/final-m3-gate/FINAL_M3_BINARY_FROZEN.json'
        marker.parent.mkdir(parents=True, exist_ok=True)
        state = {
            'schema_version': 1, 'state': 'FROZEN',
            'app_path': 'target/release/bundle/macos/OrdinConn.app',
            'executable_sha256': 'a' * 64, 'bundle_fingerprint': 'b' * 64,
            'source_head': 'c' * 40, 'source_fingerprint': 'd' * 64,
            'dirty_worktree_fingerprint': 'e' * 64,
            'built_at_utc': '2026-09-30T10:00:00+00:00',
            'frozen_at_utc': '2026-09-30T10:00:01+00:00',
            'signing_identity': 'adhoc',
        }
        marker.write_text(json.dumps(state) if contents is None else contents)
        return marker

    def test_frozen_binary_blocks_build_before_external_commands_or_app_changes(self):
        self.freeze_marker()
        original = m.bundle_digest(self.app)
        with patch.object(m, 'run', side_effect=AssertionError('must not run build commands')):
            with self.assertRaisesRegex(m.Blocked, 'FINAL_M3_BINARY_FROZEN'):
                m.build(self.root, False)
        self.assertEqual(m.bundle_digest(self.app), original)

    def test_malformed_freeze_marker_blocks_build(self):
        self.freeze_marker('{not valid json')
        with self.assertRaisesRegex(m.Blocked, 'Unknown final acceptance freeze marker'):
            m.assert_build_unfrozen(self.root)

    def test_unknown_or_incomplete_freeze_state_blocks_build(self):
        for state in [{'schema_version': 1, 'state': 'FINISHED'}, {'schema_version': 1, 'state': 'FROZEN'}, []]:
            with self.subTest(state=state):
                self.freeze_marker(json.dumps(state))
                with self.assertRaisesRegex(m.Blocked, 'Unknown final acceptance freeze marker'):
                    m.assert_build_unfrozen(self.root)

    def test_symlink_freeze_marker_is_preserved_and_blocks_build(self):
        marker = self.freeze_marker()
        original = self.root / 'retained-freeze-record.json'
        marker.rename(original)
        marker.symlink_to(original)
        with self.assertRaisesRegex(m.Blocked, 'Symlink path preserved'):
            m.assert_build_unfrozen(self.root)
        self.assertTrue(marker.is_symlink())
        self.assertTrue(original.is_file())

    def test_missing_freeze_marker_keeps_existing_workflow_available(self):
        m.assert_build_unfrozen(self.root)
        self.assertTrue(self.app.is_dir())

if __name__ == '__main__': unittest.main()
