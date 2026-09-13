"""Situational ADR 0066: preserve recovery bytes; allow normal workspace saves only."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location(
    "startup_fixture", Path(__file__).with_name("startup-recovery-fixture.py")
)
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)


class RepairPreservationTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="v4vmm-repair-inspector-test-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.config = self.root / "config/v4vmm/config.toml"
        self.config.parent.mkdir(parents=True)
        self.original = (
            '# retain settings\nmusic_dir = "/music"\ndb_path = "/library.sqlite"\n'
            'musicindex_endpoint = "http://127.0.0.1:9"\nfuture = "keep"\n'
        )
        self.config.write_text(self.original)
        self.expected = {
            "case": "repair-unreadable", "config_sha256": fixture.digest(self.config)
        }
        (self.root / "case.json").write_text(json.dumps(self.expected))
        (self.root / "case.config").write_text(self.original)
        (self.root / "config.baseline").write_text(self.original)
        self.with_workspace = self.original + (
            '\n[workspace.layout]\ncontent_pane_width = 640.0\n'
            '\n[workspace_layout]\nfocused_frame_id = 1\n'
        )

    def exited(self, code=0, revision=None):
        (self.root / "last-exit.json").write_text(json.dumps({
            "code": code,
            "config_sha256": revision or self.expected["config_sha256"],
        }))

    def inspect(self, accepted):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), patch.object(fixture, "inspect") as library:
            if accepted:
                fixture.repair_inspect(self.root, {})
                library.assert_called_once()
            else:
                with self.assertRaisesRegex(SystemExit, "Repair preservation failed"):
                    fixture.repair_inspect(self.root, {})
                library.assert_not_called()
        return json.loads(output.getvalue())

    def test_adr_0066_unchanged_recovery_document_needs_no_exit_or_backup(self):
        report = self.inspect(accepted=True)
        self.assertTrue(report["original_preserved"])
        self.assertTrue(report["config_preserved"])
        self.assertFalse(report["normal_workspace_preferences_only"])
        self.assertEqual(report["backups"], [])

    def test_adr_0066_workspace_save_after_normal_exit_is_reported_separately(self):
        self.config.write_text(self.with_workspace)
        self.exited()
        report = self.inspect(accepted=True)
        self.assertTrue(report["normal_workspace_preferences_only"])
        self.assertTrue(report["config_preserved"])
        self.assertFalse(report["original_preserved"])
        self.assertFalse(report["original_backed_up"])
        self.assertEqual(report["changed_fields"], [])
        self.assertEqual(report["backups"], [])

    def test_adr_0066_workspace_change_before_normal_exit_is_rejected(self):
        self.config.write_text(self.with_workspace)
        self.inspect(accepted=False)
        for code, revision in [(1, None), (0, "different-case")]:
            with self.subTest(code=code, revision=revision):
                self.exited(code, revision)
                self.inspect(accepted=False)

    def test_adr_0066_non_workspace_edits_remain_rejected(self):
        self.exited()
        for old, new in [('/music', '/elsewhere'), ('/library.sqlite', '/other.sqlite'),
                         ('127.0.0.1:9', '127.0.0.1:10'), ('"keep"', '"changed"')]:
            with self.subTest(setting=old):
                self.config.write_text(self.with_workspace.replace(old, new))
                self.inspect(accepted=False)

    def test_adr_0066_untrusted_case_copy_and_invalid_toml_are_rejected(self):
        self.config.write_text(self.with_workspace)
        self.exited()
        (self.root / "case.config").write_text(self.with_workspace)
        self.inspect(accepted=False)
        (self.root / "case.config").write_text(self.original)
        self.config.write_text(self.with_workspace + '\ninvalid = [\n')
        self.inspect(accepted=False)

    def test_adr_0066_other_corrections_still_need_original_preservation(self):
        self.config.write_text(self.with_workspace)
        self.exited()
        for case in fixture.REPAIR_CASES:
            if case == "repair-unreadable":
                continue
            with self.subTest(case=case):
                expected = dict(self.expected, case=case)
                (self.root / "case.json").write_text(json.dumps(expected))
                self.inspect(accepted=False)


if __name__ == "__main__":
    unittest.main()
