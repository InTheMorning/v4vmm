#!/usr/bin/env python3
"""Isolated service/encoder delays for Show action feedback 001 (ADRs 0059/0063).

Reuses the task 016 configuration boundary. Never launches the desktop app and
never invokes systemctl, butt, or a real publisher. See the task's visual check.
"""

import json
from pathlib import Path
import runpy
import sqlite3
import sys
import time

BASE = runpy.run_path(str(Path(__file__).with_name("broadcast-event-recovery-fixture.py")))
MODES = ("normal", "crash", "fail", "stuck")


def verify(root):
    BASE["verify_fixture"](root)
    if not (root / "show-action-feedback-fixture").is_file():
        raise SystemExit("Not a Show action feedback fixture")
    for binary in ("systemctl", "butt"):
        wrapper = root / "bin" / binary
        if not wrapper.is_file() or str(Path(__file__).resolve()) not in wrapper.read_text():
            raise SystemExit(f"Missing feedback stub: {wrapper}")


def setup(root):
    BASE["setup"](root)
    (root / "show-action-feedback-fixture").touch()
    (root / "feedback-mode").write_text("normal")
    (root / "publisher-state").write_text("active")
    (root / "encoder-state").write_text("0")
    config = root / "config" / "v4vmm" / "config.toml"
    with config.open("a") as output:
        output.write('\n[broadcast.encoder]\nbinary_path = ' + json.dumps(str(root / "bin" / "butt")) + '\n')
    for binary, action in (("systemctl", "service"), ("butt", "encoder")):
        wrapper = root / "bin" / binary
        wrapper.write_text(
            '#!/usr/bin/env python3\nimport os, sys\n'
            f'os.execv({sys.executable!r}, [{sys.executable!r}, {str(Path(__file__).resolve())!r}, '
            f'{action!r}, {str(root)!r}, *sys.argv[1:]])\n'
        )
        wrapper.chmod(0o700)
    verify(root)


def change(root, state_path, value, operation):
    mode = (root / "feedback-mode").read_text().strip()
    time.sleep(1)
    if mode == "fail":
        raise SystemExit("Fixture command failure")
    if mode == "crash" and operation == "start":
        value = "failed"
    if mode != "stuck":
        state_path.write_text(value)
    BASE["record"](root, operation, state_file=state_path.name, mode=mode)


def seed_feed_result(root):
    """One undownloaded track forces the full count message without network or tag writes."""
    database = root / "app.sqlite"
    if not database.is_file():
        raise SystemExit("Open and close the isolated app once to initialize its database")
    with sqlite3.connect(database, timeout=5) as conn:
        conn.execute("INSERT OR IGNORE INTO feeds (feed_url, title) VALUES (?, ?)",
                     ("https://feedback.invalid/feed", "Feedback fixture"))
        feed_id = conn.execute("SELECT id FROM feeds WHERE feed_url = ?",
                               ("https://feedback.invalid/feed",)).fetchone()[0]
        conn.execute("INSERT OR IGNORE INTO tracks (feed_id, item_guid, track_title, is_in_library) VALUES (?, ?, ?, 1)",
                     (feed_id, "feedback-undownloaded", "Feedback fixture: no downloaded file"))


def service(root, args):
    unit = next((arg for arg in args if arg.endswith(".service")), "")
    roles = {"mixxx-now-playing.service": "producer", "musicindex-live-publisher@task016-fixture.service": "publisher"}
    if unit not in roles:
        raise SystemExit("Unsupported fixture unit")
    state_path = root / f"{roles[unit]}-state"
    if "show" in args:
        state = state_path.read_text().strip()
        # Capture before waiting so a batch may finish after a command with old facts.
        time.sleep(1.5)
        print(f"LoadState=loaded\nActiveState={state}\nSubState={state}\nResult={'exit-code' if state == 'failed' else 'success'}")
        return
    operation = next((op for op in ("start", "stop", "reset-failed") if op in args), None)
    if operation is None:
        raise SystemExit("Unsupported fixture service command")
    change(root, state_path, "active" if operation == "start" else "inactive", operation)


def encoder(root, args):
    state_path = root / "encoder-state"
    if args == ["-S"]:
        state = state_path.read_text().strip()
        time.sleep(0.5)
        print(f"connected: {state}\nconnecting: 0\nrecording: 0\nsignal present: 1\nsignal absent: 0\nlisteners: 0")
    elif args in (["-s"], ["-d"]):
        change(root, state_path, "1" if args == ["-s"] else "0", "connect" if args == ["-s"] else "disconnect")
    else:
        raise SystemExit("Unsupported fixture encoder command")


def main():
    if len(sys.argv) < 3:
        raise SystemExit("Usage: show-action-feedback-fixture.py setup|verify|seed-feeds|mode DIRECTORY [normal|crash|fail|stuck]")
    action, value, *args = sys.argv[1:]
    if action == "setup":
        if not value.strip() or args:
            raise SystemExit("Setup requires one nonempty directory")
        setup(Path(value).resolve())
        return
    root = BASE["fixture_root"](value)
    verify(root)
    if action == "verify" and not args:
        print(f"Feedback fixture verified: {root}")
    elif action == "seed-feeds" and not args:
        seed_feed_result(root)
    elif action == "mode" and len(args) == 1 and args[0] in MODES:
        (root / "feedback-mode").write_text(args[0])
    elif action == "service":
        service(root, args)
    elif action == "encoder":
        encoder(root, args)
    else:
        raise SystemExit("Invalid fixture command or mode")


if __name__ == "__main__":
    main()
