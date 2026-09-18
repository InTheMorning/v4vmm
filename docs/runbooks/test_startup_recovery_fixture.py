"""Situational ADR 0066: preserve recovery bytes; allow normal workspace saves only."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import sqlite3
import sys
import subprocess
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location(
    "startup_fixture", Path(__file__).with_name("startup-recovery-fixture.py")
)
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)


class DesktopBuildTests(unittest.TestCase):
    """Situational ADR 0066: desktop acceptance must use the normal binary."""

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="v4vmm-desktop-build-test-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.manifest = {"binary": str(fixture.REPO / "target/debug/v4vmm")}
        (self.root / "case.json").write_text(json.dumps({"config_sha256": "preserved"}))

    def test_adr_0066_desktop_build_precedes_isolated_app_launch(self):
        calls = []

        def build(command, **kwargs):
            calls.append("build")
            self.assertEqual(command, ["cargo", "build", "--locked", "--offline", "--quiet",
                                       "--bin", "v4vmm", "--target-dir", str(fixture.REPO / "target")])
            self.assertEqual(kwargs, {"cwd": fixture.REPO, "check": True})
            self.assertFalse((self.root / "app.pid").exists())

        def launch(command, *, env):
            calls.append("launch")
            self.assertEqual(calls, ["build", "launch"])
            self.assertEqual(command, [self.manifest["binary"]])
            self.assertEqual(env["HOME"], str(self.root / "home"))
            self.assertTrue(env["PATH"].startswith(str(self.root / "bin") + fixture.os.pathsep))
            return Mock(pid=123, wait=Mock(return_value=0), poll=Mock(return_value=0))

        with patch.object(fixture.subprocess, "run", side_effect=build), \
                patch.object(fixture.subprocess, "Popen", side_effect=launch), \
                contextlib.redirect_stdout(io.StringIO()):
            fixture.run_app(self.root, self.manifest)
        self.assertEqual(calls, ["build", "launch"])
        self.assertFalse((self.root / "app.pid").exists())
        self.assertEqual(json.loads((self.root / "last-exit.json").read_text()),
                         {"code": 0, "config_sha256": "preserved"})

    def test_adr_0066_failed_desktop_build_never_launches_stale_binary(self):
        for error in (subprocess.CalledProcessError(1, "cargo"), FileNotFoundError("cargo")):
            with self.subTest(error=type(error).__name__), \
                    patch.object(fixture.subprocess, "run", side_effect=error), \
                    patch.object(fixture.subprocess, "Popen") as launch:
                with self.assertRaisesRegex(SystemExit, "fixture app was not opened"):
                    fixture.run_app(self.root, self.manifest)
                launch.assert_not_called()
                self.assertFalse((self.root / "app.pid").exists())
                self.assertFalse((self.root / "last-exit.json").exists())


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


class RetryPreservationTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="v4vmm-retry-inspector-test-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.config = self.root / "config/v4vmm/config.toml"
        self.config.parent.mkdir(parents=True)
        self.original = ('music_dir = "/music"\ndb_path = "/library.sqlite"\n'
                         'musicindex_endpoint = 42\nflac_path = false\n'
                         '[playback]\ndriver = false\n'
                         '[broadcast]\nselected_host = "Local"\n')
        self.config.write_text(self.original)
        (self.root / "case.config").write_text(self.original)
        (self.root / "case.json").write_text(json.dumps({
            "case": "retry-actions", "config_sha256": fixture.digest(self.config)}))
        (self.root / "retry-endpoint").write_text("http://127.0.0.1:18089")

    def inspect(self, accepted):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), patch.object(fixture, "inspect") as library:
            if accepted:
                fixture.retry_inspect(self.root, {})
                library.assert_called_once()
            else:
                with self.assertRaisesRegex(SystemExit, "Retry fixture preservation failed"):
                    fixture.retry_inspect(self.root, {})
                library.assert_not_called()
        return json.loads(output.getvalue())

    def backup(self):
        path = self.config.parent / ".v4vmm-config-original.backup"
        path.write_text(self.original)
        path.chmod(0o600)
        return path

    def test_adr_0066_retry_corrections_need_private_original_backup(self):
        self.inspect(True)
        corrected = self.original.replace("endpoint = 42", 'endpoint = "http://127.0.0.1:18089"').replace("driver = false", 'driver = "null"')
        self.config.write_text(corrected)
        self.inspect(False)
        backup = self.backup()
        self.inspect(True)
        backup.chmod(0o644)
        self.inspect(False)

    def test_adr_0066_retry_inspector_rejects_unrelated_changes(self):
        self.backup()
        for old, new in [("/music", "/different"), ("flac_path = false", "flac_path = true"),
                         ("driver = false", "driver = 0"), ("Local", "Unknown")]:
            with self.subTest(setting=old):
                self.config.write_text(self.original.replace(old, new).replace(
                    "endpoint = 42", 'endpoint = " http://127.0.0.1:18089/ "'))
                self.inspect(False)
        self.config.write_text(self.original)
        (self.root / "case.config").write_text(self.original + "\n# replaced\n")
        self.inspect(False)

    def test_adr_0066_retry_inspector_accepts_fixture_endpoint_format_without_changing_evidence(self):
        backup = self.backup()
        for endpoint in ["http://127.0.0.1:18089", "http://127.0.0.1:18089/",
                         " http://127.0.0.1:18089 ", "\t http://127.0.0.1:18089///\r\n"]:
            with self.subTest(endpoint=endpoint):
                self.config.write_text(self.original.replace("endpoint = 42", f"endpoint = {json.dumps(endpoint)}"))
                evidence = {path: path.read_bytes() for path in (self.config, backup, self.root / "case.config")}
                report = self.inspect(True)
                self.assertTrue(report["configuration_checks"]["endpoint_is_original_or_fixture"])
                self.assertTrue(report["configuration_checks"]["other_values_unchanged"])
                self.assertEqual(report["endpoint_differs_only_by_whitespace_or_trailing_slash"],
                                 endpoint != "http://127.0.0.1:18089")
                self.assertEqual(report["unexpected_setting_paths"], [])
                self.assertEqual(evidence, {path: path.read_bytes() for path in evidence})

    def test_adr_0066_retry_endpoint_format_still_requires_private_original_backup(self):
        self.config.write_text(self.original.replace("endpoint = 42", 'endpoint = "http://127.0.0.1:18089/"'))
        self.inspect(False)
        backup = self.backup()
        self.inspect(True)
        backup.chmod(0o644)
        self.inspect(False)

    def test_adr_0066_retry_inspector_rejects_changed_endpoint_target_or_type(self):
        self.backup()
        for endpoint in ["http://user:private-value@127.0.0.1:18089", "http://127.0.0.1:18090",
                         "http://127.0.0.2:18089", "https://127.0.0.1:18089",
                         "http://127.0.0.1:18089/path/", "http://127.0.0.1:18089?query=private-value",
                         "http://127.0.0.1:18089#fragment", "http://127.0.0.1:18089 /",
                         "http://127.0.0.1:18089@other.invalid", "42", 42.0, 41, False]:
            with self.subTest(endpoint=endpoint):
                self.config.write_text(self.original.replace("endpoint = 42", f"endpoint = {json.dumps(endpoint)}"))
                report = self.inspect(False)
                self.assertFalse(report["configuration_checks"]["endpoint_is_original_or_fixture"])
                self.assertFalse(report["endpoint_differs_only_by_whitespace_or_trailing_slash"])
                self.assertNotIn("private-value", json.dumps(report))

    def test_adr_0066_retry_inspector_reports_changed_fields_and_residual_candidates(self):
        self.backup()
        self.config.write_text(self.original.replace("/music", "/private-value"))
        report = self.inspect(False)
        self.assertEqual(report["unexpected_setting_paths"], ["music_dir"])
        self.assertNotIn("private-value", json.dumps(report))
        self.config.write_text(self.original)
        candidate = self.config.parent / ".v4vmm-config-test.candidate"
        candidate.write_text("private-value")
        report = self.inspect(False)
        self.assertEqual(report["residual_candidates"], [str(candidate)])
        self.assertEqual(candidate.read_text(), "private-value")

    def test_adr_0066_configuration_difference_paths_preserve_toml_types(self):
        before = {"broadcast": {"hosts": [{"instance_name": "default"}]}, "unknown": False}
        after = {"broadcast": {"hosts": [{"instance_name": "alternate"}]}, "unknown": 0}
        self.assertEqual(fixture.changed_config_paths(before, after),
                         ["broadcast.hosts[0].instance_name", "unknown"])
        self.backup()
        for path in (self.config, self.root / "case.config"):
            path.write_text("unknown = false\n" + self.original)
        (self.root / "case.json").write_text(json.dumps({
            "case": "retry-actions", "config_sha256": fixture.digest(self.config)}))
        self.backup().write_bytes(self.config.read_bytes())
        self.config.write_text("unknown = 0\n" + self.original)
        report = self.inspect(False)
        self.assertEqual(report["unexpected_setting_paths"], ["unknown"])

    def test_adr_0066_service_stub_records_only_explicit_mutations(self):
        (self.root / "bin").mkdir()
        stub = self.root / "bin/systemctl"
        original = "#!/bin/sh\nexit 1\n"
        stub.write_text(original)
        fixture.retry_service_stub(self.root)
        shown = subprocess.run([str(stub), "--user", "show", "musicindex-live-publisher@default.service"], capture_output=True, text=True)
        self.assertEqual(shown.returncode, 0)
        self.assertIn("ActiveState=inactive", shown.stdout)
        log = self.root / "retry-service-commands.jsonl"
        self.assertFalse(log.exists())
        command = ["--user", "start", "musicindex-live-publisher@default.service"]
        self.assertEqual(subprocess.run([str(stub), *command], capture_output=True).returncode, 1)
        (self.root / "retry-services-ready").touch()
        self.assertEqual(subprocess.run([str(stub), *command], capture_output=True).returncode, 0)
        self.assertEqual([json.loads(line) for line in log.read_text().splitlines()], [command, command])
        self.assertEqual((self.root / "systemctl.before-retry").read_text(), original)


class ConverterFixtureTests(unittest.TestCase):
    """Situational ADR 0066: controlled tools and strict preservation evidence."""

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="v4vmm-converter-test-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        (self.root / "bin").mkdir()
        self.config = self.root / "config/v4vmm/config.toml"
        self.config.parent.mkdir(parents=True)
        self.original = 'music_dir = "/music"\ndb_path = "/library.sqlite"\nmusicindex_endpoint = 42\n'
        self.config.write_text(self.original)
        (self.root / "case.config").write_text(self.original)
        (self.root / "case.json").write_text(json.dumps({"case": "converter-setup", "config_sha256": fixture.digest(self.config)}))
        (self.root / "converter-calls.jsonl").write_text("")

    def tools(self, mode):
        with contextlib.redirect_stdout(io.StringIO()):
            fixture.converter_tools(self.root, mode)

    def test_adr_0066_converter_path_is_isolated_and_modes_change_without_config_edits(self):
        outer_path = fixture.os.environ.get("PATH")
        env = fixture.environment(self.root)
        self.assertEqual(env["PATH"], str(self.root / "bin"))
        self.tools("missing")
        with self.assertRaises(FileNotFoundError):
            subprocess.run(["flac", "--version"], env=env, check=True)
        self.tools("working")
        for name, arg in [("flac", "--version"), ("ffmpeg", "-version")]:
            self.assertEqual(subprocess.run([name, arg], env=env, capture_output=True, timeout=5).returncode, 0)
        self.tools("fallback")
        self.assertFalse((self.root / "bin/flac").exists())
        self.assertEqual(subprocess.run(["ffmpeg", "-version"], env=env, capture_output=True, timeout=5).returncode, 0)
        self.tools("nonzero")
        self.assertEqual(subprocess.run(["flac", "--version"], env=env, capture_output=True, timeout=5).returncode, 7)
        self.tools("permission")
        with self.assertRaises(PermissionError):
            subprocess.run(["flac", "--version"], env=env, check=True)
        self.tools("timeout")
        child = subprocess.Popen(["flac", "--version"], env=env)
        try:
            with self.assertRaises(subprocess.TimeoutExpired):
                child.wait(timeout=0.2)
        finally:
            child.kill()
            child.wait()
        self.assertEqual(self.config.read_text(), self.original)
        self.assertEqual(fixture.os.environ.get("PATH"), outer_path)

    def inspect(self, accepted):
        with contextlib.redirect_stdout(io.StringIO()), patch.object(fixture, "inspect") as library:
            if accepted:
                fixture.converter_inspect(self.root, {})
                library.assert_called_once()
            else:
                with self.assertRaisesRegex(SystemExit, "Converter fixture preservation failed"):
                    fixture.converter_inspect(self.root, {})
                library.assert_not_called()

    def test_adr_0066_converter_inspection_requires_restoration_and_preserved_values(self):
        self.tools("missing")
        self.inspect(True)
        backup = self.config.parent / ".v4vmm-config-original.backup"
        backup.write_text(self.original)
        backup.chmod(0o600)
        self.config.write_text(self.original + 'flac_path = "/fixture/flac"\n')
        self.inspect(False)
        self.config.write_text(self.original.replace("endpoint = 42", "endpoint = 42.0"))
        self.inspect(False)
        self.config.write_text(self.original)
        (self.root / "converter-calls.jsonl").write_text(json.dumps({"tool": "flac", "arguments": ["track.wav"], "pid": 999999999}) + "\n")
        self.inspect(False)
        (self.root / "converter-calls.jsonl").write_text("")
        self.inspect(True)
        backup.chmod(0o644)
        self.inspect(False)


class ConversionFixtureTests(unittest.TestCase):
    """Situational ADR 0066: deterministic actual encoding and owned input removal."""

    def test_adr_0066_conversion_modes_encode_without_installed_tools_or_config_edits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "bin").mkdir()
            (root / "case.json").write_text(json.dumps({"case": "conversion-retry"}))
            (root / "converter-calls.jsonl").write_text("")
            original = root / "input.wav"
            original.write_bytes(b"RIFFfixtureWAVE")
            target = root / "output.flac"
            for mode in fixture.CONVERSION_MODES:
                with contextlib.redirect_stdout(io.StringIO()):
                    fixture.conversion_tools(root, mode)
                self.assertEqual(fixture.environment(root)["PATH"], str(root / "bin"))
                for name, version in (("flac", "--version"), ("ffmpeg", "-version")):
                    binary = root / "bin" / name
                    self.assertEqual(subprocess.run([str(binary), version], capture_output=True).returncode, 0)
                    arguments = ["-o", str(target), str(original)] if name == "flac" else ["-i", str(original), str(target)]
                    result = subprocess.run([str(binary), *arguments], capture_output=True)
                    failed = mode == "encode-failure" or (mode == "fallback" and name == "flac")
                    self.assertEqual(result.returncode, 7 if failed else 0)
                    self.assertEqual(target.read_bytes()[:4], b"fail" if failed else b"fLaC")
                    self.assertEqual(original.read_bytes(), b"RIFFfixtureWAVE")

    def test_adr_0066_conversion_missing_input_command_preserves_only_named_fixture_subject(self):
        import sqlite3
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "data").mkdir()
            (root / "music").mkdir()
            source = root / "music/original.wav"
            source.write_bytes(b"preserved fixture input")
            conn = sqlite3.connect(root / "data/library.sqlite")
            conn.execute("CREATE TABLE local_files (track_id INTEGER, path TEXT)")
            conn.execute("INSERT INTO local_files VALUES (5, 'original.wav')")
            conn.commit()
            with contextlib.redirect_stdout(io.StringIO()):
                fixture.conversion_remove_input(root)
            self.assertFalse(source.exists())
            self.assertEqual((root / "removed-conversion-input.wav").read_bytes(), b"preserved fixture input")
            self.assertEqual(conn.execute("SELECT path FROM local_files").fetchone(), ("original.wav",))
            conn.execute("UPDATE local_files SET path = '../outside.wav'")
            conn.commit()
            with self.assertRaises(SystemExit):
                fixture.conversion_remove_input(root)
            conn.close()


if __name__ == "__main__":
    unittest.main()


class DatabaseLockTests(unittest.TestCase):
    """Situational ADR 0066: real external readers must observe fixture locks."""

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="v4vmm-database-lock-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        data = self.root / "database"
        data.mkdir()
        for name in ("wal", "locked"):
            conn = sqlite3.connect(data / (name + ".sqlite"))
            conn.execute("CREATE TABLE playlists (name TEXT)")
            conn.commit()
            conn.close()

    def start_helper(self, operation, ready):
        program = '''import importlib.util, sys
from pathlib import Path
spec = importlib.util.spec_from_file_location("fixture", sys.argv[1])
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
root = Path(sys.argv[2])
if sys.argv[3] == "legacy":
    sleep = fixture.time.sleep
    def legacy_sleep(seconds):
        (root / "database/locked.sqlite").read_bytes()
        (root / "legacy.ready").write_text("ready")
        sleep(seconds)
    fixture.time.sleep = legacy_sleep
    fixture.database_hold(root)
else:
    getattr(fixture, sys.argv[3])(root)
'''
        child = subprocess.Popen([sys.executable, "-B", "-c", program,
                                  str(Path(fixture.__file__)), str(self.root), operation],
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

        def stop():
            if child.poll() is None:
                child.terminate()
            child.communicate(timeout=5)
        self.addCleanup(stop)
        deadline = time.monotonic() + 5
        while not (self.root / ready).exists():
            if child.poll() is not None or time.monotonic() > deadline:
                stop()
                self.fail(f"Database helper did not reach {ready}")
            time.sleep(0.01)
        return child

    def test_adr_0066_checksums_do_not_release_the_exclusive_lock(self):
        helper = self.start_helper("database_hold", "database.ready")
        self.assertTrue(fixture.database_reader_blocked(self.root))
        baseline = json.loads((self.root / "database-baseline.json").read_text())
        self.assertEqual(fixture.digest(self.root / "database/locked.sqlite"),
                         baseline["source_hashes"]["locked.sqlite"])
        with contextlib.redirect_stdout(io.StringIO()) as output:
            fixture.database_status(self.root)
        self.assertTrue(json.loads(output.getvalue())["exclusive_lock_blocks_reader"])
        self.assertTrue(fixture.database_reader_blocked(self.root))
        fixture.database_stop(self.root)
        self.assertEqual(helper.wait(timeout=5), 0)
        self.assertFalse(fixture.database_reader_blocked(self.root))

    def test_adr_0066_restore_lock_keeps_wal_and_existing_baseline(self):
        original = self.start_helper("legacy", "legacy.ready")
        self.assertFalse(fixture.database_reader_blocked(self.root))
        baseline = (self.root / "database-baseline.json").read_bytes()
        wal = (self.root / "database/wal.sqlite-wal").read_bytes()
        lock = self.start_helper("database_lock", "database-lock.ready")
        self.assertTrue(fixture.database_reader_blocked(self.root))
        self.assertIsNone(original.poll())
        self.assertEqual((self.root / "database-baseline.json").read_bytes(), baseline)
        self.assertEqual((self.root / "database/wal.sqlite-wal").read_bytes(), wal)
        with contextlib.redirect_stdout(io.StringIO()) as output:
            fixture.database_lock(self.root)
        self.assertIn("already blocks readers", output.getvalue())
        self.assertTrue(fixture.database_reader_blocked(self.root))
        fixture.database_stop(self.root)
        self.assertEqual(original.wait(timeout=5), 0)
        self.assertEqual(lock.wait(timeout=5), 0)
        self.assertFalse((self.root / "database-lock.pid").exists())
        self.assertFalse((self.root / "database.pid").exists())

    def test_adr_0066_maintenance_writer_release_restores_exclusive_access(self):
        before = fixture.digest(self.root / "database/locked.sqlite")
        helper = self.start_helper("maintenance_hold", "maintenance.ready")
        self.assertTrue(fixture.maintenance_writer_blocked(self.root))
        self.assertEqual(fixture.digest(self.root / "database/locked.sqlite"), before)
        self.assertTrue(fixture.maintenance_writer_blocked(self.root))
        fixture.maintenance_release(self.root)
        self.assertEqual(helper.wait(timeout=5), 0)
        self.assertFalse(fixture.maintenance_writer_blocked(self.root))
        self.assertEqual(fixture.digest(self.root / "database/locked.sqlite"), before)
        self.assertFalse((self.root / "maintenance.pid").exists())


class DatabasePreservationTests(unittest.TestCase):
    """Situational ADR 0066: snapshots and preservation require positive evidence."""

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="v4vmm-database-inspect-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        data = self.root / "database"
        data.mkdir()
        (data / "readonly.sqlite").write_bytes(b"source")
        (data / "readonly.sqlite").chmod(0o400)
        (data / "wal.sqlite-wal").write_bytes(b"committed WAL")
        (self.root / "case.json").write_text(json.dumps({"case": "database-tools"}))
        (self.root / "database.ready").write_text("ready")
        (self.root / "database-baseline.json").write_text(json.dumps({
            "source_hashes": {"readonly.sqlite": fixture.digest(data / "readonly.sqlite")},
            "wal_sha256": fixture.digest(data / "wal.sqlite-wal"),
        }))

    def test_missing_snapshots_cannot_pass_preservation(self):
        output = io.StringIO()
        with patch.object(fixture, "owned_process", return_value=123), \
                patch.object(fixture, "inspect") as shared, contextlib.redirect_stdout(output):
            with self.assertRaisesRegex(SystemExit, "preservation inspection failed"):
                fixture.database_inspect(self.root, {"migration_versions": [1]})
        result = json.loads(output.getvalue())
        self.assertTrue(result["database_sources_preserved"])
        self.assertFalse(result["normal_backup_exists"])
        self.assertFalse(result["wal_backup_exists"])
        self.assertFalse(result["readonly_backup_exists"])
        shared.assert_not_called()

    def test_stopped_wal_helper_cannot_pass_inspection(self):
        with patch.object(fixture, "owned_process", return_value=None):
            with self.assertRaisesRegex(SystemExit, "must remain running"):
                fixture.database_inspect(self.root, {})

    def test_reentering_case_does_not_reseed_or_replace_baseline(self):
        baseline = (self.root / "database-baseline.json").read_bytes()
        with patch.object(fixture, "owned_process", return_value=123), \
                patch.object(fixture.subprocess, "run") as seed, \
                patch.object(fixture.subprocess, "Popen") as spawn, \
                patch.object(fixture, "database_status"):
            fixture.database_setup(self.root)
        seed.assert_not_called()
        spawn.assert_not_called()
        self.assertEqual((self.root / "database-baseline.json").read_bytes(), baseline)

    def test_stopped_helper_cannot_replace_the_preservation_baseline(self):
        baseline = (self.root / "database-baseline.json").read_bytes()
        with patch.object(fixture, "owned_process", return_value=None), \
                patch.object(fixture.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(SystemExit, "do not rebaseline"):
                fixture.database_setup(self.root)
        spawn.assert_not_called()
        self.assertEqual((self.root / "database-baseline.json").read_bytes(), baseline)


class MaintenanceCopyTests(unittest.TestCase):
    """Situational ADR 0066: a completed manifest must account for every copied byte."""

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="v4vmm-maintenance-inspect-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.data = self.root / "database"
        self.data.mkdir()
        hashes = {}
        for copy, original in (("locked", "locked"), ("damaged", "integrity")):
            source = self.data / (original + ".sqlite")
            source.write_bytes(b"source evidence " + original.encode())
            hashes[source.name] = fixture.digest(source)
            destination = self.data / (copy + "-copy")
            destination.mkdir(mode=0o700)
            copied = destination / "database.sqlite"
            copied.write_bytes(source.read_bytes())
            copied.chmod(0o600)
            manifest = {"kind": "database_file_preservation_not_verified_backup", "source": str(source),
                        "journal_mode": "delete", "sqlite_version": "fixture", "sqlite_access": "acquired",
                        "acquired_at_utc": "2026-09-17T10:00:00+00:00", "copied_at_utc": "2026-09-17T10:00:01+00:00",
                        "files": [{"source": str(source), "copied_name": copied.name,
                                   "length": copied.stat().st_size, "sha256": fixture.digest(copied)}]}
            receipt = destination / "manifest.json"
            receipt.write_text(json.dumps(manifest))
            receipt.chmod(0o600)
        (self.root / "maintenance-baseline.json").write_text(json.dumps({"source_hashes": hashes}))
        (self.root / "case.json").write_text(json.dumps({"case": "database-maintenance-recovery"}))

    def inspect(self, expected):
        with patch.object(fixture, "inspect") as shared, contextlib.redirect_stdout(io.StringIO()):
            if expected:
                fixture.maintenance_inspect(self.root, {})
                shared.assert_called_once()
            else:
                with self.assertRaisesRegex(SystemExit, "Maintenance preservation inspection failed"):
                    fixture.maintenance_inspect(self.root, {})
                shared.assert_not_called()

    def test_adr_0066_complete_private_copy_passes_and_changed_bytes_fail(self):
        self.inspect(True)
        (self.data / "locked-copy/database.sqlite").write_bytes(b"changed")
        self.inspect(False)

    def test_adr_0066_empty_manifest_cannot_claim_preservation(self):
        receipt = self.data / "locked-copy/manifest.json"
        manifest = json.loads(receipt.read_text())
        manifest["files"] = []
        receipt.write_text(json.dumps(manifest))
        self.inspect(False)

    def test_adr_0066_busy_artifacts_and_changed_originals_fail_inspection(self):
        destination = self.data / "busy-copy"
        destination.mkdir()
        self.inspect(False)
        destination.rmdir()
        (self.data / "integrity.sqlite").write_bytes(b"changed original")
        self.inspect(False)

    def test_adr_0066_maintenance_setup_cannot_replace_existing_evidence(self):
        baseline = (self.root / "maintenance-baseline.json").read_bytes()
        with patch.object(fixture.subprocess, "run") as seed:
            with self.assertRaisesRegex(SystemExit, "already has a baseline"):
                fixture.maintenance_setup(self.root)
        seed.assert_not_called()
        self.assertEqual((self.root / "maintenance-baseline.json").read_bytes(), baseline)


class RestorePreservationTests(unittest.TestCase):
    """Situational ADR 0066: the restore inspector cannot accept missing evidence."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="v4vmm-restore-inspector-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name in ("data", "database", "config/v4vmm", "music"):
            (self.root / name).mkdir(parents=True)
        source = self.root / "data/library.sqlite"
        source.write_bytes(b"restored fixture database")
        backup = self.root / "database/restore-backup.sqlite"
        backup.write_bytes(b"chosen backup")
        config = self.root / "config/v4vmm/config.toml"
        config.write_text("fixture configuration")
        (self.root / "broadcaster-token.fixture").write_bytes(b"token sentinel")
        (self.root / "broadcaster-token.fixture").chmod(0o600)
        (self.root / "music/unchanged-audio.bin").write_bytes(b"music sentinel")
        folder = self.root / "database/restored-preservation"
        folder.mkdir(mode=0o700)
        preserved = folder / "database.sqlite"
        preserved.write_bytes(b"damaged original")
        preserved.chmod(0o600)
        receipt = {"kind": "database_file_preservation_not_verified_backup", "source": str(source),
                   "files": [{"copied_name": "database.sqlite", "length": preserved.stat().st_size,
                              "sha256": fixture.digest(preserved)}]}
        (folder / "manifest.json").write_text(json.dumps(receipt))
        (folder / "manifest.json").chmod(0o600)
        baseline = {"backup_hashes": {backup.name: fixture.digest(backup)},
                    "destination_inode": source.stat().st_ino, "destination_device": source.stat().st_dev,
                    "original_sha256": fixture.digest(preserved),
                    "token_sha256": fixture.digest(self.root / "broadcaster-token.fixture")}
        (self.root / "restore-baseline.json").write_text(json.dumps(baseline))
        (self.root / "case.json").write_text(json.dumps({"case": "database-restore-recovery", "config_sha256": fixture.digest(config)}))
        (self.root / "session-observations.jsonl").write_text(json.dumps({"state": "opened", "generation": 2}) + "\n")
        self.manifest = {"audio_sha256": fixture.digest(self.root / "music/unchanged-audio.bin"), "track_sha256": {}}
        self.facts = {"integrity": [("ok",)], "foreign_keys": [], "probes": 0}

    def inspect(self, accepted):
        with patch.object(fixture, "restore_database_facts", return_value=self.facts), contextlib.redirect_stdout(io.StringIO()):
            if accepted:
                fixture.restore_inspect(self.root, self.manifest)
            else:
                with self.assertRaisesRegex(SystemExit, "Restore preservation inspection failed"):
                    fixture.restore_inspect(self.root, self.manifest)

    def test_adr_0066_restore_inspection_accepts_complete_recovery_evidence(self):
        self.inspect(True)

    def test_adr_0066_restore_inspection_rejects_damaged_preservation_and_missing_manifest(self):
        (self.root / "database/restored-preservation/database.sqlite").write_bytes(b"changed")
        self.inspect(False)
        (self.root / "database/restored-preservation/manifest.json").unlink()
        self.inspect(False)

    def test_adr_0066_restore_inspection_rejects_changed_backup_or_destination_inode(self):
        (self.root / "database/restore-backup.sqlite").write_bytes(b"changed")
        self.inspect(False)
        (self.root / "database/restore-backup.sqlite").write_bytes(b"chosen backup")
        source = self.root / "data/library.sqlite"
        source.rename(source.with_suffix(".retained"))
        source.write_bytes(b"restored fixture database")
        self.inspect(False)

    def test_adr_0066_restore_inspection_rejects_duplicate_or_missing_session(self):
        report = self.root / "session-observations.jsonl"
        report.write_text(report.read_text() * 2)
        self.inspect(False)
        report.unlink()
        self.inspect(False)
