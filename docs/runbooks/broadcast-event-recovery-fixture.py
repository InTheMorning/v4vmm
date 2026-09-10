#!/usr/bin/env python3
"""Isolated operator fixture for ADR 0059 tasks 016/017; never starts the desktop app."""

import json
import os
from pathlib import Path
import sqlite3
import sys
import time
import tomllib
from http.server import BaseHTTPRequestHandler, HTTPServer


JOURNAL_MODES = ("normal", "slow", "slow-fail")


def fixture_root(value):
    if not value.strip():
        raise SystemExit(
            "Fixture directory is empty: task016_dir is unset in this terminal. "
            "Run this script with 'locate' to recover the existing directory."
        )
    root = Path(value).resolve()
    if not (root / "task-016-fixture").is_file():
        raise SystemExit(
            f"Not a task 016 fixture directory: {root}\n"
            "Run this script with 'locate', or use the exact directory printed by setup."
        )
    return root


def locate_fixture(directory=Path("/tmp")):
    roots = sorted(
        root for root in directory.glob("v4vmm-task016.*")
        if (root / "task-016-fixture").is_file()
    )
    if not roots:
        raise SystemExit("No task 016 fixture found. Follow setup in the visual walkthrough first.")
    if len(roots) != 1:
        raise SystemExit(
            "Multiple task 016 fixtures found. Use the directory belonging to your running relay:\n"
            + "\n".join(str(root) for root in roots)
        )
    return roots[0]


def verify_fixture(root):
    """Check isolation before an operator launches the desktop app (ADR 0059)."""
    config_path = root / "config" / "v4vmm" / "config.toml"
    try:
        config = tomllib.loads(config_path.read_text())
    except (OSError, ValueError) as error:
        raise SystemExit(f"Cannot verify fixture config {config_path}: {error}") from error
    expected = {
        "musicindex_endpoint": "http://127.0.0.1:17863",
        "db_path": str(root / "app.sqlite"),
        "music_dir": str(root / "music"),
    }
    for key, value in expected.items():
        if config.get(key) != value:
            raise SystemExit(f"Fixture config mismatch: {key} must be {value!r}. App launch stopped.")
    broadcast = config.get("broadcast", {})
    host = {"name": "Task 016 fixture", "transport": "local", "instance_name": "task016-fixture"}
    if (broadcast.get("selected_host") != host["name"]
            or broadcast.get("hosts") != [host]):
        raise SystemExit("Fixture config mismatch: expected the Task 016 fixture host. App launch stopped.")
    for binary in ["systemctl", "musicindex-live-publisher", "journalctl"]:
        path = root / "bin" / binary
        if not path.is_file() or not os.access(path, os.X_OK):
            raise SystemExit(f"Fixture command stub is missing or not executable: {path}. App launch stopped.")


def setup(root):
    if any(root.iterdir()):
        raise SystemExit("Fixture directory must be empty")
    (root / "task-016-fixture").touch()
    (root / "config" / "v4vmm").mkdir(parents=True)
    (root / "bin").mkdir()
    (root / "music").mkdir()
    (root / "mode").write_text("fail")
    (root / "producer-state").write_text("active")
    (root / "publisher-state").write_text("active")
    (root / "create-mode").write_text("live")
    (root / "targets.json").write_text('{"targets": []}')
    (root / "calls.jsonl").touch()
    config = (
        f'music_dir = {json.dumps(str(root / "music"))}\n'
        f'db_path = {json.dumps(str(root / "app.sqlite"))}\n'
        'musicindex_endpoint = "http://127.0.0.1:17863"\n'
        '[broadcast]\nselected_host = "Task 016 fixture"\ndrop_file_target = "default"\n'
        '[[broadcast.hosts]]\nname = "Task 016 fixture"\ntransport = "local"\ninstance_name = "task016-fixture"\n'
    )
    (root / "config" / "v4vmm" / "config.toml").write_text(config)
    for binary, action in [("systemctl", "service"), ("musicindex-live-publisher", "publisher"), ("journalctl", "journal")]:
        wrapper = root / "bin" / binary
        wrapper.write_text(
            '#!/usr/bin/env python3\nimport os, sys\n'
            f'os.execv({sys.executable!r}, [{sys.executable!r}, {str(Path(__file__).resolve())!r}, '
            f'{action!r}, {str(root)!r}, *sys.argv[1:]])\n'
        )
        wrapper.chmod(0o700)
    print(root)


def record(root, operation, **fields):
    with (root / "calls.jsonl").open("a") as log:
        log.write(json.dumps({"operation": operation, **fields}) + "\n")


def seed_target_scope(root):
    """Prepare task 017's configured-versus-unused target check (ADR 0059)."""
    verify_fixture(root)
    try:
        with sqlite3.connect((root / "app.sqlite").as_uri() + "?mode=ro",
                             uri=True, timeout=5) as conn:
            events = dict(conn.execute("SELECT event_id, token_path FROM broadcast_events"))
            selection = conn.execute(
                "SELECT event_id FROM broadcast_event_selection WHERE singleton = 1"
            ).fetchone()
    except sqlite3.Error as error:
        raise SystemExit(
            f"Could not read the fixture's saved events: {error}. "
            "Open the fixture app and select your replacement event first."
        ) from error
    older_id = "fixture-event-1"
    selected_id = selection[0] if selection else None
    if not selected_id or selected_id == older_id:
        raise SystemExit(
            "Select your replacement event in the app and wait for its check to finish. "
            "Keep fixture-event-1 as the older event."
        )
    if older_id not in events or selected_id not in events:
        raise SystemExit(
            "The fixture must contain fixture-event-1 and your selected replacement event. "
            "Complete the Create/Replace checks first."
        )
    payload = {"targets": [
        {"name": name, "event_id": event_id, "token_file": events[event_id],
         "stream_delay_secs": 0.0}
        for name, event_id in [("default", older_id), ("unused", selected_id)]
    ]}
    temporary = root / "targets.pending.json"
    try:
        temporary.write_text(json.dumps(payload))
        temporary.replace(root / "targets.json")
    except OSError as error:
        raise SystemExit(f"Could not save the fixture targets: {error}") from error
    record(root, "seed target scope", configured_event_id=older_id,
           unused_event_id=selected_id)
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S UTC", time.gmtime())
    print(f"[{timestamp}] Fixture target default now uses event {older_id}.")
    print(f"[{timestamp}] Fixture target unused now uses event {selected_id}.")
    print("In the app, choose More → Check again. Expect Not attached.")


def service(root, args):
    # This executable shadows systemctl only for the isolated app process.
    unit = next((arg for arg in args if arg.endswith(".service")), "")
    producer = unit == "mixxx-now-playing.service"
    state_file = root / ("producer-state" if producer else "publisher-state")
    state = state_file.read_text().strip()
    if "show" in args:
        print(f"LoadState=loaded\nActiveState={state}\nSubState={state}\nResult={'exit-code' if state == 'failed' else 'success'}")
    elif any(op in args for op in ["start", "stop", "restart", "reset-failed"]):
        state_file.write_text("inactive" if "stop" in args or "reset-failed" in args else "active")
        record(root, "service action", unit=unit)
    else:
        raise SystemExit("Unsupported fixture service command")


def publisher(root, args):
    if args[:2] == ["target", "list"]:
        record(root, "target list")
        print((root / "targets.json").read_text())
    elif args[:2] == ["target", "add"]:
        value = lambda key: args[args.index(key) + 1]
        event_id = value("--event-id")
        name = value("--name")
        targets = [target for target in json.loads((root / "targets.json").read_text())["targets"]
                   if target["name"] != name]
        targets.append({"name": name, "event_id": event_id,
                        "token_file": value("--token-file"), "stream_delay_secs": 0.0})
        (root / "targets.json").write_text(json.dumps({"targets": targets}))
        record(root, "target add", name=name, event_id=event_id)
    elif args[:2] == ["target", "remove"]:
        name = args[args.index("--name") + 1]
        targets = [target for target in json.loads((root / "targets.json").read_text())["targets"]
                   if target["name"] != name]
        (root / "targets.json").write_text(json.dumps({"targets": targets}))
        record(root, "target remove", name=name)
    else:
        raise SystemExit("Unsupported fixture publisher command")


def journal(root, args):
    unit = next((arg for arg in args if arg.endswith(".service")), "unknown service")
    # Existing fixture directories and wrappers keep working without setup again.
    mode_path = root / "journal-mode"
    mode = mode_path.read_text().strip() if mode_path.exists() else "normal"
    if mode not in JOURNAL_MODES:
        raise SystemExit("Invalid fixture journal mode. Run journal-mode DIRECTORY normal.")
    record(root, "journal", unit=unit, mode=mode)
    if mode != "normal":
        time.sleep(5)  # ADR 0063: allow switching or closing before the result arrives.
    # Use the mode captured at request start, even if the operator changed it.
    if mode == "slow-fail":
        record(root, "journal result", unit=unit, mode=mode, result="failed")
        raise SystemExit(f"Fixture could not read journal for {unit} (deliberate delayed failure).")
    record(root, "journal result", unit=unit, mode=mode, result="succeeded")
    print(f"Fixture journal: {unit}\n2026-09-10 12:00:00 service started\n2026-09-10 12:00:01 fixture observation complete")


def relay_handler(root):
    class Relay(BaseHTTPRequestHandler):
        counter = 0
        read_ids = set()

        def log_message(self, *_):
            pass

        def reply(self, code, body):
            data = json.dumps(body).encode()
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_POST(self):
            self.rfile.read(int(self.headers.get("Content-Length", 0)))
            if self.path != "/v1/liveitems":
                self.reply(404, {})
                return
            if (root / "create-mode").read_text().strip() == "fail":
                record(root, "registration failed")
                self.reply(503, {"error": "Fixture registration failure"})
                return
            Relay.counter += 1
            event_id = f"fixture-event-{Relay.counter}"
            record(root, "register", event_id=event_id)
            self.reply(200, {"event_id": event_id, "broadcaster_token": "local-fixture-only",
                             "metadata_url": f"/v1/liveitems/{event_id}/metadata", "events_url": "/events"})

        def do_GET(self):
            if not self.path.startswith("/v1/liveitems/") or not self.path.endswith("/metadata"):
                self.reply(200, {"ok": True})
                return
            event_id = self.path.split("/")[3]
            mode = (root / "mode").read_text().strip()
            first = event_id not in Relay.read_ids
            Relay.read_ids.add(event_id)
            time.sleep(2)  # Visible check progress, including the first failed read.
            code = 503 if first or mode == "fail" else 404 if mode == "dead" else 200
            record(root, "check", event_id=event_id, status=code)
            self.reply(code, {"event_id": event_id, "seq": 1,
                              "updated_at": "2026-09-09T00:00:00Z", "metadata": {}})

    return Relay


def serve(root):
    Relay = relay_handler(root)
    print(f"Fixture directory: {root}", flush=True)
    print("Fixture relay at http://127.0.0.1:17863; Ctrl+C stops it", flush=True)
    try:
        with HTTPServer(("127.0.0.1", 17863), Relay) as server:
            server.serve_forever()
    except KeyboardInterrupt:
        pass


def main():
    if sys.argv[1:] == ["locate"]:
        print(locate_fixture())
        return
    if len(sys.argv) < 3:
        raise SystemExit("Usage: fixture.py locate | setup|verify|serve|mode|create-mode|producer-state|publisher-state|target-scope|journal-mode DIRECTORY [VALUE]")
    action = sys.argv[1]
    if action == "setup":
        if not sys.argv[2].strip():
            raise SystemExit("Setup directory is empty. Create it with mktemp as shown in the walkthrough.")
        setup(Path(sys.argv[2]).resolve())
        return
    root = fixture_root(sys.argv[2])
    if action == "verify":
        verify_fixture(root)
        print(f"Fixture verified: {root}\nRelay: http://127.0.0.1:17863\nSource must show: Task 016 fixture")
    elif action == "target-scope":
        if len(sys.argv) != 3:
            raise SystemExit("Usage: fixture.py target-scope DIRECTORY")
        seed_target_scope(root)
    elif action == "journal-mode":
        if len(sys.argv) != 4 or sys.argv[3] not in JOURNAL_MODES:
            raise SystemExit(f"Expected one of: {JOURNAL_MODES}")
        verify_fixture(root)
        mode = sys.argv[3]
        pending = root / "journal-mode.pending"
        pending.write_text(mode)
        pending.replace(root / "journal-mode")
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S UTC", time.gmtime())
        response = {"normal": "return service logs immediately",
                    "slow": "return service logs after five seconds",
                    "slow-fail": "fail service-log reads after five seconds"}[mode]
        print(f"[{timestamp}] Fixture will {response} for new requests.")
    elif action in ("mode", "create-mode", "producer-state", "publisher-state"):
        allowed = {"mode": ["fail", "live", "dead"], "create-mode": ["live", "fail"]}.get(action, ["active", "inactive", "failed"])
        if len(sys.argv) != 4 or sys.argv[3] not in allowed:
            raise SystemExit(f"Expected one of: {allowed}")
        (root / action).write_text(sys.argv[3])
    elif action == "serve":
        serve(root)
    elif action == "service":
        service(root, sys.argv[3:])
    elif action == "journal":
        journal(root, sys.argv[3:])
    elif action == "publisher":
        publisher(root, sys.argv[3:])
    else:
        raise SystemExit("Unknown fixture action")


if __name__ == "__main__":
    main()
