#!/usr/bin/env python3
"""Finite, editable service journals for ADR 0063 shared-log acceptance.

Reuses the prepared ADR 0066 library and isolation boundary. This helper never
launches a GUI or contacts a service. Only its owned command stubs return logs.
"""

import contextlib
from datetime import datetime, timezone
import io
import json
from pathlib import Path
import runpy
import sys

BASE = runpy.run_path(str(Path(__file__).with_name("startup-recovery-fixture.py")))
UNITS = ("mixxx-now-playing.service", "musicindex-live-publisher@log-frames-fixture.service")


def verify(value):
    root, manifest = BASE["verify"](value)
    if not (root / "log-frame-fixture").is_file():
        raise SystemExit("Not a shared-log fixture")
    return root, manifest


def write_entries(root, count, replace=False):
    counter_path = root / "log-counter"
    first = int(counter_path.read_text()) if counter_path.exists() else 0
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
    for unit in UNITS:
        path = root / "logs" / unit
        text = "" if replace else path.read_text() if path.exists() else ""
        for number in range(first + 1, first + count + 1):
            detail = (" /long/path/segment" * 24 + "/END-OF-LONG-LINE") if number % 10 == 0 else "café 🦀"
            text += f"[{stamp}] Fixture {unit} recorded entry {number:06d}: {detail}\n"
        temporary = path.with_suffix(".next")
        temporary.write_text(text)
        temporary.replace(path)
    counter_path.write_text(str(first + count))


def setup():
    captured = io.StringIO()
    with contextlib.redirect_stdout(captured):
        BASE["setup"]()
    root = Path(captured.getvalue().strip())
    with contextlib.redirect_stdout(io.StringIO()):
        BASE["mode"](root, "session-held-command")
    (root / "log-frame-fixture").touch()
    (root / "logs").mkdir()
    cfg = root / "config/v4vmm/config.toml"
    text = cfg.read_text().replace("[playback]", 'ui_scale = "medium"\ntheme_profile = "dark"\n[playback]') + '''
[broadcast]
selected_host = "Log frame fixture"
[[broadcast.hosts]]
name = "Log frame fixture"
transport = "local"
instance_name = "log-frames-fixture"
'''
    cfg.write_text(text)
    (root / "case.config").write_text(text)
    (root / "case.json").write_text(json.dumps({"case": "session-held-command", "config_sha256": BASE["digest"](cfg)}))
    for binary, command in (("journalctl", "journal"), ("systemctl", "service")):
        wrapper = root / "bin" / binary
        wrapper.write_text(
            "#!/usr/bin/env python3\nimport os, sys\n"
            f"os.execv({sys.executable!r}, [{sys.executable!r}, {str(Path(__file__).resolve())!r}, "
            f"{command!r}, {str(root)!r}, *sys.argv[1:]])\n"
        )
        wrapper.chmod(0o700)
    write_entries(root, 80)
    verify(root)
    print(root)


def main():
    if sys.argv[1:] == ["setup"]:
        setup()
        return
    if len(sys.argv) < 3:
        raise SystemExit("Usage: log-frame-fixture.py setup | verify|append|trim|replace DIRECTORY")
    command, directory, *args = sys.argv[1:]
    root, _ = verify(directory)
    if command == "verify" and not args:
        print(f"Log fixture verified: {root}")
    elif command in ("append", "replace") and not args:
        write_entries(root, 12 if command == "append" else 80, replace=command == "replace")
        print("Fixture journals updated. The visible service log should receive the next snapshot.")
    elif command == "trim" and not args:
        for unit in UNITS:
            path = root / "logs" / unit
            temporary = path.with_suffix(".next")
            temporary.write_text("".join(path.read_text().splitlines(keepends=True)[-30:]))
            temporary.replace(path)
        print("Fixture journals now retain only their last 30 entries.")
    elif command in ("journal", "service"):
        unit = next((argument for argument in args if argument in UNITS), None)
        if unit is None:
            raise SystemExit("Unknown fixture unit")
        if command == "journal":
            text = (root / "logs" / unit).read_text()
            print("".join(text.splitlines(keepends=True)[-200:]), end="")
        elif "show" in args:
            print("LoadState=loaded\nActiveState=active\nSubState=running\nResult=success")
        else:
            raise SystemExit("Shared-log fixture permits observation only")
    else:
        raise SystemExit("Invalid shared-log fixture command")


if __name__ == "__main__":
    main()
