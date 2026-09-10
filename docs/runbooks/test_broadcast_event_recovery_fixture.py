"""Situational ADRs 0059/0063: isolated recovery and log fixtures; no GUI or real units."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("broadcast-event-recovery-fixture.py")
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("fixture", SCRIPT)
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)


class RecoveryFixtureTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="v4vmm-task017-test.")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        with contextlib.redirect_stdout(io.StringIO()):
            fixture.setup(self.root)

    def test_isolation_and_invalid_paths(self):
        fixture.verify_fixture(self.root)
        for value in ["", str(self.root / "missing")]:
            with self.assertRaises(SystemExit):
                fixture.fixture_root(value)
        with self.assertRaises(SystemExit):
            fixture.setup(self.root)
        for action in ["mode", "create-mode", "producer-state", "publisher-state", "journal-mode"]:
            with patch.object(sys, "argv", [str(SCRIPT), action, str(self.root), "invalid"]):
                with self.assertRaises(SystemExit):
                    fixture.main()
        (self.root / "bin/journalctl").unlink()
        with self.assertRaises(SystemExit):
            fixture.verify_fixture(self.root)

    def test_named_target_replacement_and_removal_preserve_other_associations(self):
        def add(name, event):
            fixture.publisher(self.root, ["target", "add", "--name", name,
                              "--event-id", event, "--token-file", "/fixture/token", "--replace"])
        add("default", "event-one")
        add("unused", "event-two")
        add("duplicate", "event-two")
        add("default", "event-two")
        targets = lambda: json.loads((self.root / "targets.json").read_text())["targets"]
        self.assertEqual(len(targets()), 3)
        fixture.publisher(self.root, ["target", "remove", "--name", "default"])
        self.assertEqual({target["name"] for target in targets()}, {"unused", "duplicate"})
        self.assertTrue(all(target["event_id"] == "event-two" for target in targets()))
        calls = (self.root / "calls.jsonl").read_text()
        self.assertNotIn("token", calls)
        self.assertIn('"name": "default"', calls)

    def store_events(self, selected="fixture-event-3"):
        """Supply the saved-event columns used by the fixture, including a newer row."""
        with sqlite3.connect(self.root / "app.sqlite") as conn:
            conn.executescript("""
                CREATE TABLE broadcast_events (event_id TEXT PRIMARY KEY, token_path TEXT);
                CREATE TABLE broadcast_event_selection (singleton INTEGER PRIMARY KEY, event_id TEXT);
            """)
            for number in [1, 3, 4]:
                event_id = f"fixture-event-{number}"
                token = self.root / f"{event_id}.token"
                token.write_text(f"private-fixture-token-{number}")
                conn.execute("INSERT INTO broadcast_events VALUES (?, ?)", (event_id, str(token)))
            if selected is not None:
                conn.execute("INSERT INTO broadcast_event_selection VALUES (1, ?)", (selected,))

    def test_target_scope_command_uses_saved_choice_without_changing_registry_or_tokens(self):
        self.store_events()
        preserved = [self.root / "app.sqlite", self.root / "config/v4vmm/config.toml",
                     *self.root.glob("*.token")]
        before = {path: path.read_bytes() for path in preserved}
        result = subprocess.run([sys.executable, str(SCRIPT), "target-scope", str(self.root)],
                                capture_output=True, text=True, check=True)
        targets = json.loads((self.root / "targets.json").read_text())["targets"]
        self.assertEqual([(target["name"], target["event_id"]) for target in targets],
                         [("default", "fixture-event-1"), ("unused", "fixture-event-3")])
        for target in targets:
            self.assertEqual(target["token_file"], str(self.root / f'{target["event_id"]}.token'))
        self.assertEqual({path: path.read_bytes() for path in preserved}, before)
        self.assertFalse((self.root / "targets.pending.json").exists())
        self.assertIn("Fixture target default now uses event fixture-event-1.", result.stdout)
        self.assertIn("Fixture target unused now uses event fixture-event-3.", result.stdout)
        self.assertRegex(result.stdout, r"\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC\]")
        calls = (self.root / "calls.jsonl").read_text()
        self.assertEqual(json.loads(calls)["operation"], "seed target scope")
        self.assertNotIn("private-fixture-token", result.stdout + result.stderr + calls)

    def test_target_scope_rejects_unprepared_fixture_without_rewriting_targets(self):
        for case in ["no_database", "no_schema", "no_selection", "older_selected",
                     "missing_older", "missing_selected", "wrong_config"]:
            with self.subTest(case=case):
                database = self.root / "app.sqlite"
                database.unlink(missing_ok=True)
                if case == "no_schema":
                    database.touch()
                elif case != "no_database":
                    selected = {"no_selection": None,
                                "older_selected": "fixture-event-1"}.get(case, "fixture-event-3")
                    self.store_events(selected)
                    if case in ["missing_older", "missing_selected"]:
                        absent = "fixture-event-1" if case == "missing_older" else "fixture-event-3"
                        with sqlite3.connect(database) as conn:
                            conn.execute("DELETE FROM broadcast_events WHERE event_id = ?", (absent,))
                    if case == "wrong_config":
                        (self.root / "config/v4vmm/config.toml").write_text('db_path = "/wrong.sqlite"')
                before = (self.root / "targets.json").read_bytes()
                result = subprocess.run([sys.executable, str(SCRIPT), "target-scope", str(self.root)],
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("Traceback", result.stderr)
                self.assertEqual((self.root / "targets.json").read_bytes(), before)
                self.assertEqual((self.root / "calls.jsonl").read_text(), "")
                self.assertFalse((self.root / "targets.pending.json").exists())
                if case == "no_database":
                    self.assertFalse(database.exists())

    def test_registration_failure_does_not_allocate_identity_and_get_keeps_latency(self):
        handler_type = fixture.relay_handler(self.root)
        handler = handler_type.__new__(handler_type)
        handler.path = "/v1/liveitems"
        handler.headers = {"Content-Length": "0"}
        handler.rfile = io.BytesIO()
        replies = []
        handler.reply = lambda code, body: replies.append((code, body))
        (self.root / "create-mode").write_text("fail")
        handler.do_POST()
        self.assertEqual(handler_type.counter, 0)
        self.assertEqual(replies[-1][0], 503)
        (self.root / "create-mode").write_text("live")
        handler.do_POST()
        self.assertEqual(replies[-1][1]["event_id"], "fixture-event-1")
        handler.path = "/v1/liveitems/fixture-event-1/metadata"
        (self.root / "mode").write_text("live")
        with patch.object(fixture.time, "sleep") as sleep:
            handler.do_GET()
            self.assertEqual(replies[-1][0], 503)
            handler.do_GET()
            self.assertEqual(replies[-1][0], 200)
            self.assertEqual(sleep.call_args_list, [unittest.mock.call(2), unittest.mock.call(2)])
        self.assertNotIn("local-fixture-only", (self.root / "calls.jsonl").read_text())

    def test_service_states_are_independent_and_journal_stub_is_executable(self):
        for unit, state_file in [("mixxx-now-playing.service", "producer-state"),
                                 ("musicindex-live-publisher@task016-fixture.service", "publisher-state")]:
            fixture.service(self.root, ["stop", unit])
            self.assertEqual((self.root / state_file).read_text(), "inactive")
            (self.root / state_file).write_text("failed")
            fixture.service(self.root, ["reset-failed", unit])
            self.assertEqual((self.root / state_file).read_text(), "inactive")
            fixture.service(self.root, ["restart", unit])
            self.assertEqual((self.root / state_file).read_text(), "active")
            logs = subprocess.check_output([str(self.root / "bin/journalctl"), "--user", "-u", unit], text=True)
            self.assertIn(unit, logs)
            self.assertEqual(len(logs.splitlines()), 3)

    def test_journal_modes_capture_the_result_before_waiting(self):
        """Situational ADR 0063: changing fixture mode cannot rewrite a pending result."""
        unit = "mixxx-now-playing.service"
        mode_path = self.root / "journal-mode"
        for mode in ["slow", "slow-fail"]:
            with self.subTest(mode=mode):
                mode_path.write_text(mode)
                output = io.StringIO()
                with patch.object(fixture.time, "sleep",
                                  side_effect=lambda _: mode_path.write_text("normal")) as sleep:
                    with contextlib.redirect_stdout(output):
                        if mode == "slow-fail":
                            with self.assertRaisesRegex(SystemExit, "deliberate delayed failure"):
                                fixture.journal(self.root, ["-u", unit])
                        else:
                            fixture.journal(self.root, ["-u", unit])
                    sleep.assert_called_once_with(5)
                if mode == "slow-fail":
                    self.assertEqual(output.getvalue(), "")
                else:
                    self.assertIn(unit, output.getvalue())
                calls = [json.loads(line) for line in (self.root / "calls.jsonl").read_text().splitlines()]
                self.assertEqual(calls[-2], {"operation": "journal", "unit": unit, "mode": mode})
                self.assertEqual(calls[-1], {"operation": "journal result", "unit": unit,
                                            "mode": mode,
                                            "result": "failed" if mode == "slow-fail" else "succeeded"})

    def test_journal_mode_command_updates_existing_wrapper_and_restores_normal_reads(self):
        """Situational ADR 0063: the desktop fixture needs no reset for delayed failures."""
        wrapper = self.root / "bin/journalctl"
        original_wrapper = wrapper.read_bytes()
        unit = "musicindex-live-publisher@task016-fixture.service"
        mode_command = [sys.executable, str(SCRIPT), "journal-mode", str(self.root)]
        result = subprocess.run([*mode_command, "slow-fail"], capture_output=True,
                                text=True, check=True, timeout=10)
        self.assertIn("fail service-log reads after five seconds", result.stdout)
        self.assertFalse((self.root / "journal-mode.pending").exists())
        failed = subprocess.run([str(wrapper), "--user", "-u", unit], capture_output=True,
                                text=True, timeout=10)
        self.assertNotEqual(failed.returncode, 0)
        self.assertEqual(failed.stdout, "")
        self.assertIn(unit, failed.stderr)
        self.assertIn("deliberate delayed failure", failed.stderr)
        self.assertNotIn("Traceback", failed.stderr)
        subprocess.run([*mode_command, "normal"], capture_output=True, check=True, timeout=10)
        with patch.object(fixture.time, "sleep") as sleep:
            with contextlib.redirect_stdout(io.StringIO()) as output:
                fixture.journal(self.root, ["-u", unit])
            sleep.assert_not_called()
            self.assertIn(unit, output.getvalue())
        self.assertEqual(wrapper.read_bytes(), original_wrapper)


if __name__ == "__main__":
    unittest.main()
