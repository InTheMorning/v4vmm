#!/usr/bin/env python3
"""Isolated ADR 0066 fixture. Only run opens the GUI."""
import argparse
import base64
import hashlib
import http.server
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
import tomllib

KIND = "v4vmm-startup-recovery-v1"
REPO = Path(__file__).resolve().parents[2]
CASES = ("normal", "invalid-toml", "music-missing", "music-file", "db-locked", "long-path",
         "runtime-unavailable", "cache-worker-unavailable", "runtime-and-cache-unavailable",
         "endpoint-and-player-unavailable", "presentation-invalid", "producer-unavailable",
         "publisher-invalid", "partial-path-repair")
OPTIONAL_CASES = CASES[-5:]
REPAIR_CASES = ("repair-toml", "repair-paths", "repair-optional", "repair-unreadable", "repair-backup-failure", "repair-conflict")
RETRY_CASES = ("retry-actions",)
CONVERTER_CASES = ("converter-setup", "converter-recovery")
CONVERTER_MODES = ("missing", "working", "fallback", "nonzero", "timeout", "permission", "output-limit")
CONVERSION_CASES = ("conversion-retry",)
DATABASE_CASES = ("database-tools", "database-recovery")
MAINTENANCE_CASES = ("database-maintenance", "database-maintenance-recovery")
RESTORE_CASES = ("database-restore", "database-restore-recovery")
CONVERSION_MODES = ("encode-failure", "working", "fallback")
CASES += ("session-held-command",) + REPAIR_CASES + RETRY_CASES + CONVERTER_CASES + CONVERSION_CASES + DATABASE_CASES + MAINTENANCE_CASES + RESTORE_CASES


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(directory):
    root = Path(directory).resolve()
    try:
        manifest = json.loads((root / "fixture.json").read_text())
        if manifest.get("kind") != KIND or manifest.get("root") != str(root) or not root.name.startswith("v4vmm-startup-"):
            raise ValueError("identity mismatch")
        if Path(manifest["binary"]).resolve() != REPO / "target/debug/v4vmm":
            raise ValueError("binary does not belong to this checkout")
        for relative in ("config", "data", "music", "home", "bin", "music.saved", "database"):
            if (root / relative).is_symlink():
                raise ValueError(f"unexpected symlink: {relative}")
        if not Path(manifest["binary"]).is_file():
            raise ValueError("build the debug binary first")
        return root, manifest
    except (OSError, ValueError, KeyError) as error:
        raise SystemExit(f"Not an ADR 0066 startup fixture directory: {root}\n{error}\nUse this script with locate, or use the exact directory printed by setup.")


def environment(root):
    env = os.environ.copy()
    for name in tuple(env):
        if name.startswith("V4VMM_"):
            del env[name]
    env.update(HOME=str(root / "home"), XDG_CONFIG_HOME=str(root / "config"),
               XDG_DATA_HOME=str(root / "data"), XDG_CACHE_HOME=str(root / "cache"),
               PATH=str(root / "bin") + os.pathsep + os.defpath,
               V4VMM_STARTUP_FIXTURE=str(root))
    case_file = root / "case.json"
    if case_file.exists() and json.loads(case_file.read_text()).get("case") in CONVERTER_CASES + CONVERSION_CASES:
        # No installed converter can leak into the missing-tools case. Stub
        # interpreters and sleep use absolute paths; cargo builds before this env.
        env["PATH"] = str(root / "bin")
    # Keep XDG_RUNTIME_DIR/DISPLAY for the operator's desktop connection.
    return env


def normal_workspace_preferences_only(root, expected, current):
    """ADR 0066 permits existing workspace saves only after normal resumption."""
    last_exit = root / "last-exit.json"
    invalid_optional = expected["case"] in ("endpoint-and-player-unavailable", "presentation-invalid", "publisher-invalid")
    if invalid_optional or not last_exit.exists():
        return False
    try:
        record = json.loads(last_exit.read_text())
        if record != {"code": 0, "config_sha256": expected["config_sha256"]}:
            return False
        original = (root / "case.config").read_bytes()
        if hashlib.sha256(original).hexdigest() != expected["config_sha256"]:
            return False
        before = tomllib.loads(original.decode())
        after = tomllib.loads(current.decode())
    except (OSError, ValueError, UnicodeError):
        return False
    for key in ("workspace", "workspace_layout"):
        before.pop(key, None)
        after.pop(key, None)
    return before == after


def inspect(root, manifest, correction_preserved=False):
    cfg = root / "config/v4vmm/config.toml"
    expected = json.loads((root / "case.json").read_text())
    audio = root / ("music.saved" if (root / "music.saved").exists() else "music") / "unchanged-audio.bin"
    raw = cfg.read_bytes()
    unchanged = hashlib.sha256(raw).hexdigest() == expected["config_sha256"]
    normal_preferences_only = not unchanged and normal_workspace_preferences_only(root, expected, raw)
    result = {"case": expected["case"], "config_bytes_unchanged": unchanged,
              "normal_workspace_preferences_only": normal_preferences_only,
              "config_preserved": unchanged or normal_preferences_only or correction_preserved,
              "music_preserved": digest(audio) == manifest["audio_sha256"],
              "residual_music_probes": [str(p.relative_to(root)) for pattern in (".v4vmm-startup-probe-*", ".v4vmm-producer-probe-*") for p in root.rglob(pattern)]}
    music = audio.parent
    result["music_preserved"] &= all(digest(music / name) == expected_hash for name, expected_hash in manifest["track_sha256"].items())
    completed = subprocess.run([manifest["binary"], "startup-fixture", "inspect", str(root)],
                               env=environment(root), text=True, capture_output=True, check=True)
    result.update(json.loads(completed.stdout))
    result["migration_records_preserved"] = result["migration_versions"] == manifest["migration_versions"]
    expected_bindings = ["a.wav", "b.wav", "c.wav"]
    if expected["case"] == "partial-path-repair":
        # Before launch neither update has run; after launch only the first commits.
        expected_bindings = ["a.wav", "/old/music/b.wav", "c.wav"]
        result["repair_not_attempted"] = result["bindings"] == ["/old/music/a.wav", "/old/music/b.wav", "c.wav"]
    result["bindings_preserved"] = result["bindings"] == expected_bindings or result.get("repair_not_attempted", False)
    result["library_preserved"] = result["tracks"] == 3 and result["playlist_tracks"] == 3
    blockers = {"endpoint-and-player-unavailable": ("occupied-runtime", b"preserve runtime blocker\n"),
                "producer-unavailable": ("occupied-producer", b"preserve producer blocker\n")}
    blocker = blockers.get(expected["case"])
    result["tool_blockers_preserved"] = blocker is None or (root / blocker[0]).read_bytes() == blocker[1]
    print(json.dumps(result, indent=2))
    if not all(result[key] for key in ("config_preserved", "music_preserved", "migration_records_preserved", "bindings_preserved", "library_preserved", "tool_blockers_preserved")) or result["residual_music_probes"] or result["database_probes"] or result["playlists"] != 1:
        raise SystemExit("Fixture inspection failed. Keep the fixture for diagnosis.")


def owned_process(root, name):
    marker = root / name
    if not marker.exists():
        return None
    pid = int(marker.read_text())
    try:
        cmd = Path(f"/proc/{pid}/cmdline").read_bytes()
        env = Path(f"/proc/{pid}/environ").read_bytes()
        if str(root).encode() not in cmd and str(root).encode() not in env:
            raise SystemExit(f"Refusing unrelated process {pid} in {marker}")
        os.kill(pid, 0)
        return pid
    except ProcessLookupError:
        marker.unlink(missing_ok=True)
    except FileNotFoundError:
        marker.unlink(missing_ok=True)
    return None


def release_lock(root):
    pid = owned_process(root, "lock.pid")
    if pid:
        os.kill(pid, signal.SIGTERM)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and (root / "lock.ready").exists():
            time.sleep(0.05)
        if (root / "lock.ready").exists():
            raise SystemExit("Fixture lock did not exit; keep the directory and inspect the process.")
    (root / "lock.pid").unlink(missing_ok=True)


def hold_lock(root):
    conn = sqlite3.connect(root / "data/library.sqlite", timeout=5)
    conn.execute("BEGIN IMMEDIATE")
    (root / "lock.pid").write_text(str(os.getpid()))
    (root / "lock.ready").write_text("ready")
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    try:
        while True:
            time.sleep(1)
    finally:
        conn.rollback()
        conn.close()
        (root / "lock.ready").unlink(missing_ok=True)
        (root / "lock.pid").unlink(missing_ok=True)


def mode(root, case):
    previous = json.loads((root / "case.json").read_text())["case"]
    if (case in OPTIONAL_CASES + REPAIR_CASES + RETRY_CASES + CONVERTER_CASES + CONVERSION_CASES + DATABASE_CASES + MAINTENANCE_CASES + RESTORE_CASES or previous in OPTIONAL_CASES + REPAIR_CASES + RETRY_CASES + CONVERTER_CASES + CONVERSION_CASES + DATABASE_CASES + MAINTENANCE_CASES + RESTORE_CASES or "session-held-command" in (case, previous)) and owned_process(root, "app.pid"):
        raise SystemExit("Close the fixture app before changing optional-tool or session cases.")
    if previous in CONVERTER_CASES + CONVERSION_CASES:
        for name in ("flac", "ffmpeg"):
            (root / "bin" / name).unlink(missing_ok=True)
    if previous in RETRY_CASES + CONVERSION_CASES:
        stop_retry_server(root)
        saved_stub = root / "systemctl.before-retry"
        if saved_stub.exists():
            (root / "bin/systemctl").write_bytes(saved_stub.read_bytes())
            saved_stub.unlink()
    release_lock(root)
    if case != "session-held-command":
        (root / "session.hold").unlink(missing_ok=True)
    if (root / "music.saved").exists():
        if (root / "music").is_file():
            (root / "music").unlink()
        (root / "music.saved").rename(root / "music")
    cfg = root / "config/v4vmm/config.toml"
    cfg.parent.chmod(0o700)
    cfg.chmod(0o600)
    (root / "repair.external").unlink(missing_ok=True)
    cfg.write_bytes((root / "config.baseline").read_bytes())
    if case in RESTORE_CASES:
        restore_setup(root, case)
        purpose = "Review the chosen backup before explicit Restore database. restore-status prints all paths. Use the task 012 runbook; no hardware or external services are needed."
    elif case in MAINTENANCE_CASES:
        maintenance_setup(root)
        if case == "database-maintenance-recovery":
            cfg.write_text(cfg.read_text().replace(json.dumps(str(root / "data/library.sqlite")), json.dumps(str(root / "database/invalid-header.sqlite"))))
        purpose = "Use Database tools for explicit session drain and file preservation. maintenance-status prints paths; maintenance-release stops only this fixture's writer. No audio hardware or external service is needed."
    elif case in DATABASE_CASES:
        database_setup(root)
        if case == "database-recovery":
            cfg.write_text(cfg.read_text().replace(json.dumps(str(root / "data/library.sqlite")), json.dumps(str(root / "database/invalid-header.sqlite"))))
        purpose = "Use Database tools in Settings > Diagnostics or core recovery. database-status prints source and destination paths. Keep the owned WAL/lock helper running for these checks. No audio hardware or real service is needed."
    elif case in CONVERSION_CASES:
        retry_tools(root)
        endpoint = (root / "retry-endpoint").read_text()
        cfg.write_text(cfg.read_text().replace('musicindex_endpoint = "http://127.0.0.1:9"', 'musicindex_endpoint = ' + json.dumps(endpoint)))
        subprocess.run([str(REPO / "target/debug/v4vmm"), "startup-fixture", "conversion-seed", str(root)], env=environment(root), check=True)
        (root / "converter-calls.jsonl").write_text("")
        conversion_tools(root, "encode-failure")
        purpose = "Open Startup fixture playlist in Music. Download Conversion retry and Conversion redownload. The isolated converters pass version checks but fail encoding until conversion-tools changes them. No audio hardware, installed converter, or external service is needed."
    elif case in CONVERTER_CASES:
        text = cfg.read_text().replace('musicindex_endpoint = "http://127.0.0.1:9"', 'musicindex_endpoint = 42')
        if case == "converter-recovery":
            text = text.replace(json.dumps(str(root / "music")), '""')
        cfg.write_text(text)
        (root / "converter-calls.jsonl").write_text("")
        converter_tools(root, "missing")
        purpose = "Open Converter setup in Settings or Configuration repair in core recovery. Test converters uses a stub-only PATH; converter-tools changes executable modes while the app remains open. No audio hardware or installed converter is needed."
    elif case in RETRY_CASES:
        text = cfg.read_text().replace('musicindex_endpoint = "http://127.0.0.1:9"', 'musicindex_endpoint = 42\nflac_path = false')
        text = text.replace('driver = "null"', 'driver = false')
        text += '\n[broadcast]\nselected_host = "Local"\n[[broadcast.hosts]]\nname = "Local"\ntransport = "local"\ninstance_name = "default"\n[[broadcast.hosts]]\nname = "Alternate"\ntransport = "local"\ninstance_name = "alternate"\n'
        cfg.write_text(text)
        retry_tools(root)
        purpose = "Use retained-action repair with the isolated Index server and service stubs. retry-status prints the endpoint and records; retry-release allows stub service commands. Playback uses the explicit Null driver after correction; no audio hardware is needed."
    elif case in OPTIONAL_CASES:
        text = cfg.read_text()
        if case == "endpoint-and-player-unavailable":
            text = text.replace('musicindex_endpoint = "http://127.0.0.1:9"', 'musicindex_endpoint = "invalid endpoint"')
            text = text.replace('driver = "null"', 'driver = "mpv"')
            (root / "occupied-runtime").write_bytes(b"preserve runtime blocker\n")
            purpose = "App must open normally, retain both setup reports, reject Index/playback commands and keep local search and playlists usable."
        elif case == "presentation-invalid":
            text = 'theme_profile = 42\nui_scale = "invalid"\n' + text
            text += '\n[workspace.layout]\ncontent_list_view_mode = "invalid"\n'
            purpose = "Resize and navigate with fallback presentation. Inspect must report config_bytes_unchanged after closing."
        elif case == "producer-unavailable":
            mpv = shutil.which("mpv")
            if not mpv:
                raise SystemExit("This audio check needs an installed mpv binary. Install it in the desktop session before selecting this case.")
            text = text.replace('driver = "null"', 'driver = "mpv"\nmpv_path = ' + json.dumps(mpv))
            (root / "occupied-producer").write_bytes(b"preserve producer blocker\n")
            text += '\n[broadcast]\ndrop_directory = ' + json.dumps(str(root / "occupied-producer")) + '\n'
            purpose = "Play a.wav in the fixture playlist: hear the quiet tone despite the producer setup report. External controls must still attempt the isolated service stub."
        elif case == "publisher-invalid":
            text += '\n[broadcast]\nhosts = "invalid"\ndrop_directory = ' + json.dumps(str(root / "drop")) + '\n[broadcast.encoder]\nbinary_path = "butt"\n'
            purpose = "Publisher setup must fail independently. Local playback and drop publication must work; encoder checks must reach the isolated butt stub."
        else:
            purpose = "App must report incomplete path repair and retain all three library tracks. a.wav and c.wav remain playable; b.wav must not execute its unvalidated binding."
        cfg.write_text(text)
    elif case in REPAIR_CASES:
        text = cfg.read_text()
        if case in ("repair-toml", "repair-backup-failure"):
            text += '\ninvalid = [\n# remove these final two lines in the app editor\n'
        elif case == "repair-paths":
            shutil.copytree(root / "music", root / "music-choice", dirs_exist_ok=True)
            text = text.replace(json.dumps(str(root / "music")), '\"\"')
            text = text.replace(json.dumps(str(root / "data/library.sqlite")), '\"\"')
        elif case in ("repair-optional", "repair-conflict"):
            text = text.replace('musicindex_endpoint = "http://127.0.0.1:9"', 'musicindex_endpoint = 42\nflac_path = false')
        cfg.write_text(text)
        purpose = "Use Configuration repair in recovery or Settings. Editing must keep a draft; Save must name its original backup. Use repair-inspect before cleanup."
    elif case == "invalid-toml":
        with cfg.open("a") as stream:
            stream.write('\ninvalid = [\n# intentional fixture syntax error\n')
        purpose = "App must explain the TOML error and preserve this file. Launch the fixture."
    elif case in ("music-missing", "music-file"):
        (root / "music").rename(root / "music.saved")
        if case == "music-file":
            (root / "music").write_bytes(b"fixture file where a directory is required\n")
        purpose = "App must name the unusable music location. Launch the fixture."
    elif case == "db-locked":
        with (root / "lock.log").open("w") as output:
            subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "hold-lock", str(root)], stdout=output, stderr=output, start_new_session=True)
        deadline = time.monotonic() + 5
        while not (root / "lock.ready").exists():
            if time.monotonic() > deadline:
                raise SystemExit("Fixture could not acquire its database lock. Inspect lock.log.")
            time.sleep(0.05)
        purpose = "App must report SQLite's locked write probe after five seconds. Launch the fixture."
    elif case == "long-path":
        text = cfg.read_text()
        long_path = root / ("long-music-directory-" + "x" * 140) / ("nested-" + "y" * 140)
        text = text.replace(json.dumps(str(root / "music")), json.dumps(str(long_path)))
        cfg.write_text(text)
        purpose = "App must wrap and copy the full missing path. Launch the fixture; inspect narrow and normal widths."
    elif case == "session-held-command":
        (root / "session.hold").write_text("Release this admitted command with session-release.\n")
        (root / "session-observations.jsonl").unlink(missing_ok=True)
        purpose = "App must report the held command during session drain. Use session-status, then session-release and Retry drain."
    elif case == "runtime-unavailable":
        purpose = "App must open Music and offer a background-runtime report and Check again. Navigation and local search must work."
    elif case == "cache-worker-unavailable":
        purpose = "App must report thumbnail cleanup failure and keep normal operations available."
    elif case == "runtime-and-cache-unavailable":
        purpose = "App must retain separate runtime and thumbnail issues. Repairing one must leave the other visible."
    else:
        purpose = "Fixture files are restored. Use Check again on the failed tool, or Check again and Open app in core recovery. Otherwise launch the fixture."
    (root / "case.json").write_text(json.dumps({"case": case, "config_sha256": digest(cfg)}))
    shutil.copyfile(cfg, root / "case.config")
    if case in OPTIONAL_CASES or previous in OPTIONAL_CASES:
        _, manifest = verify(root)
        subprocess.run([manifest["binary"], "startup-fixture", "paths", str(root)], env=environment(root), check=True)
    if case == "repair-unreadable":
        cfg.chmod(0)
        purpose += " Config mode is 000; expect a report without an editor. Use repair-access to restore access."
    elif case == "repair-backup-failure":
        cfg.parent.chmod(0o500)
        purpose += " Config directory is not writable; Save must fail to create a backup. Use repair-access, then Save again."
    print(purpose)


def repair_inspect(root, manifest):
    cfg = root / "config/v4vmm/config.toml"
    expected = json.loads((root / "case.json").read_text())
    if expected["case"] not in REPAIR_CASES:
        raise SystemExit("repair-inspect requires a repair case.")
    backups = sorted(cfg.parent.glob(".v4vmm-config-*.backup"))
    copies = [{"path": str(p), "sha256": digest(p), "owner_only": p.stat().st_mode & 0o777 == 0o600} for p in backups]
    raw = cfg.read_bytes()
    unchanged = hashlib.sha256(raw).hexdigest() == expected["config_sha256"]
    external = root / "repair.external"
    conflict_preserved = external.exists() and external.read_bytes() == raw
    before = tomllib.loads((root / "config.baseline").read_text())
    changed_fields = []
    try:
        after = tomllib.loads(raw.decode())
        for key in ("workspace", "workspace_layout"):
            before.pop(key, None)
            after.pop(key, None)
        changed_fields = [key for key in set(before) | set(after) if before.get(key) != after.get(key)]
        allowed = {"musicindex_endpoint", "flac_path"} if expected["case"] in ("repair-optional", "repair-conflict") else set()
        if expected["case"] == "repair-paths" and after.get("music_dir") == str(root / "music-choice"):
            allowed.add("music_dir")
        if expected["case"] == "repair-optional" and after.get("musicindex_endpoint") == "http://127.0.0.1:9" and "flac_path" not in after:
            if after.get("ui_scale") == "medium" and after.get("theme_profile") == "dark":
                allowed.update(("ui_scale", "theme_profile"))
        values_preserved = unchanged or set(changed_fields) <= allowed
        if expected["case"] in ("repair-optional", "repair-conflict"):
            values_preserved &= after.get("musicindex_endpoint") in (42, 99, "http://127.0.0.1:9") and after.get("flac_path") in (False, None)
    except (ValueError, UnicodeError):
        values_preserved = unchanged
    original_backed_up = any(item["sha256"] == expected["config_sha256"] for item in copies)
    original_preserved = unchanged or conflict_preserved or original_backed_up
    normal_preferences_only = (
        expected["case"] == "repair-unreadable" and not unchanged
        and normal_workspace_preferences_only(root, expected, raw)
    )
    config_preserved = original_preserved or normal_preferences_only
    selected_music_preserved = not (root / "music-choice").exists() or all(digest(root / "music-choice" / name) == expected_hash for name, expected_hash in manifest["track_sha256"].items())
    report = {"case": expected["case"], "original_preserved": original_preserved,
              "original_backed_up": original_backed_up, "external_revision_preserved": conflict_preserved,
              "normal_workspace_preferences_only": normal_preferences_only, "config_preserved": config_preserved,
              "selected_music_preserved": selected_music_preserved,
              "unedited_config_values_preserved": values_preserved, "changed_fields": sorted(changed_fields),
              "backups": copies, "residual_candidates": [str(p) for p in cfg.parent.glob(".v4vmm-config-*.candidate")]}
    print(json.dumps(report, indent=2))
    if not config_preserved or not values_preserved or not selected_music_preserved or not all(item["owner_only"] for item in copies) or report["residual_candidates"]:
        raise SystemExit("Repair preservation failed. Keep the fixture for diagnosis.")
    # Reuse correction evidence and the separately checked normal-workspace
    # allowance; neither permits changes to unrelated settings.
    inspect(root, manifest, correction_preserved=original_preserved)


def stop_retry_server(root):
    pid = owned_process(root, "retry-server.pid")
    if pid:
        os.kill(pid, signal.SIGTERM)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and (root / "retry-endpoint").exists():
            time.sleep(0.05)
        if (root / "retry-endpoint").exists():
            raise SystemExit("Fixture Index server did not exit. Keep its directory.")
    (root / "retry-server.pid").unlink(missing_ok=True)


def retry_server(root):
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            with (root / "retry-requests.jsonl").open("a") as output:
                output.write(json.dumps({"at": datetime.now(timezone.utc).isoformat(), "request": self.path}) + "\n")
            audio = self.path in ("/conversion-4.wav", "/conversion-5.wav")
            body = (root / "music/a.wav").read_bytes() if audio else b'{"data":[],"pagination":{"has_more":false}}'
            self.send_response(200)
            self.send_header("Content-Type", "audio/wav" if audio else "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_):
            pass

    server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    (root / "retry-server.pid").write_text(str(os.getpid()))
    (root / "retry-endpoint").write_text(f"http://127.0.0.1:{server.server_port}")
    try:
        server.serve_forever()
    finally:
        server.server_close()
        (root / "retry-endpoint").unlink(missing_ok=True)
        (root / "retry-server.pid").unlink(missing_ok=True)


def retry_service_stub(root):
    stub = root / "bin/systemctl"
    saved = root / "systemctl.before-retry"
    if not saved.exists():
        saved.write_bytes(stub.read_bytes())
    stub.write_text(f"#!{sys.executable}\n" + "import json, sys\nfrom pathlib import Path\n" + f"root = Path({str(root)!r})\n" +
        "args = sys.argv[1:]\n"
        "if 'show' in args:\n    print('LoadState=loaded\\nActiveState=inactive\\nSubState=dead\\nResult=success')\n    sys.exit(0)\n"
        "with (root / 'retry-service-commands.jsonl').open('a') as output:\n    output.write(json.dumps(args) + '\\n')\n"
        "if not (root / 'retry-services-ready').exists():\n    print('Fixture service command rejected; use retry-release.', file=sys.stderr)\n    sys.exit(1)\n")
    stub.chmod(0o700)


def retry_tools(root):
    stop_retry_server(root)
    (root / "retry-services-ready").unlink(missing_ok=True)
    (root / "retry-requests.jsonl").write_text("")
    (root / "retry-service-commands.jsonl").write_text("")
    retry_service_stub(root)
    with (root / "retry-server.log").open("w") as output:
        subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "retry-server", str(root)], stdout=output, stderr=output, start_new_session=True)
    deadline = time.monotonic() + 5
    while not (root / "retry-endpoint").exists():
        if time.monotonic() > deadline:
            raise SystemExit("Fixture Index server did not start. Inspect retry-server.log.")
        time.sleep(0.05)


def retry_status(root):
    if json.loads((root / "case.json").read_text())["case"] not in RETRY_CASES:
        raise SystemExit("retry-status requires retry-actions.")
    report = {"endpoint": (root / "retry-endpoint").read_text(),
              "service_commands_enabled": (root / "retry-services-ready").exists()}
    for name, file in (("index_requests", "retry-requests.jsonl"), ("service_commands", "retry-service-commands.jsonl")):
        report[name] = [json.loads(line) for line in (root / file).read_text().splitlines()]
    print(json.dumps(report, indent=2))


def changed_config_paths(before, after, path=""):
    """Report changed TOML field paths without printing configuration values."""
    if type(before) is not type(after):
        return [path]
    if isinstance(before, dict):
        changes = []
        for key in sorted(before.keys() | after.keys()):
            child = f"{path}.{key}" if path else key
            if key not in before or key not in after:
                changes.append(child)
            else:
                changes.extend(changed_config_paths(before[key], after[key], child))
        return changes
    if isinstance(before, list):
        if len(before) != len(after):
            return [path]
        return [change for index, (old, new) in enumerate(zip(before, after))
                for change in changed_config_paths(old, new, f"{path}[{index}]")]
    return [] if before == after else [path]


def retry_inspect(root, manifest):
    if json.loads((root / "case.json").read_text())["case"] not in RETRY_CASES:
        raise SystemExit("retry-inspect requires retry-actions.")
    cfg = root / "config/v4vmm/config.toml"
    expected = json.loads((root / "case.json").read_text())
    backups = list(cfg.parent.glob(".v4vmm-config-*.backup"))
    original_preserved = digest(cfg) == expected["config_sha256"] or any(digest(path) == expected["config_sha256"] for path in backups)
    before = tomllib.loads((root / "case.config").read_text())
    after = tomllib.loads(cfg.read_text())
    endpoint = (root / "retry-endpoint").read_text()
    driver = after.get("playback", {}).get("driver")
    saved_endpoint = after.get("musicindex_endpoint")
    # ADR 0066: focused correction preserves the entered string; the app trims
    # surrounding whitespace and trailing slashes when loading the endpoint.
    # Accept only that spelling of this fixture's exact URL, not another target.
    endpoint_matches = (isinstance(saved_endpoint, str)
                        and saved_endpoint.strip().rstrip("/") == endpoint)
    checks = {
        "case_copy_matches_original": digest(root / "case.config") == expected["config_sha256"],
        "endpoint_is_original_or_fixture": (type(saved_endpoint) is int and saved_endpoint == 42) or endpoint_matches,
        "converter_is_unchanged": after.get("flac_path") is False,
        "player_is_original_or_null": driver is False or driver == "null",
        "publisher_is_fixture_host": after.get("broadcast", {}).get("selected_host") in ("Local", "Alternate"),
    }
    endpoint_format_only = endpoint_matches and saved_endpoint != endpoint
    for document in (before, after):
        for key in ("workspace", "workspace_layout", "musicindex_endpoint"):
            document.pop(key, None)
        document.get("playback", {}).pop("driver", None)
        document.get("broadcast", {}).pop("selected_host", None)
    changed_paths = changed_config_paths(before, after)
    checks["other_values_unchanged"] = not changed_paths
    allowed = all(checks.values())
    modes = all(path.stat().st_mode & 0o777 == 0o600 for path in backups)
    candidates = list(cfg.parent.glob(".v4vmm-config-*.candidate"))
    print(json.dumps({"original_preserved": original_preserved, "unedited_values_preserved": allowed,
                      "owner_only_backups": modes, "backups": [str(path) for path in backups],
                      "configuration_checks": checks,
                      "unexpected_setting_paths": changed_paths,
                      "endpoint_differs_only_by_whitespace_or_trailing_slash": endpoint_format_only,
                      "residual_candidates": [str(path) for path in candidates]}, indent=2))
    if not original_preserved or not allowed or not modes or candidates:
        raise SystemExit("Retry fixture preservation failed. Keep this fixture.")
    inspect(root, manifest, correction_preserved=True)


def converter_tools(root, tool_mode):
    """Only fixture executables change; the running app keeps its PATH/config."""
    for name in ("flac", "ffmpeg"):
        tool = root / "bin" / name
        tool.unlink(missing_ok=True)
        if tool_mode == "missing" or (tool_mode == "fallback" and name == "flac"):
            continue
        behavior = "exit 0"
        if tool_mode == "nonzero":
            behavior = "printf 'fixture secret must not enter report' >&2\nexit 7"
        elif tool_mode == "timeout":
            behavior = "exec /bin/sleep 30"
        elif tool_mode == "output-limit":
            behavior = "while :; do printf '012345678901234567890123456789'; done"
        # Python's absolute interpreter is used only to record arguments safely.
        # exec sleep retains the shell PID; the probe must reap it after timeout.
        log_code = "import json,os,sys;from datetime import datetime,timezone;open(sys.argv[1],'a').write(json.dumps({'tool':sys.argv[2],'arguments':sys.argv[3:],'pid':os.getppid(),'at':datetime.now(timezone.utc).isoformat()})+'\\n')"
        record = " ".join(shlex.quote(part) for part in (sys.executable, "-c", log_code, str(root / "converter-calls.jsonl"), name))
        version_arg = "--version" if name == "flac" else "-version"
        tool.write_text(f'#!/bin/sh\n{record} "$@"\n[ "$#" = 1 ] && [ "$1" = {version_arg} ] || exit 91\n{behavior}\n')
        tool.chmod(0o600 if tool_mode == "permission" else 0o700)
    (root / "converter-mode").write_text(tool_mode)
    print(f"Converter fixture mode: {tool_mode}. Press Test converters in the open app.")


def converter_status(root):
    calls = root / "converter-calls.jsonl"
    records = [json.loads(line) for line in calls.read_text().splitlines()]
    print(json.dumps({"mode": (root / "converter-mode").read_text(),
                      "path": environment(root)["PATH"],
                      "configured_test_path": str(root / "bin/flac"),
                      "missing_test_path": str(root / "missing-flac"),
                      "observations": records}, indent=2))
    return records


def conversion_tools(root, tool_mode):
    """Deterministic encoding stubs; existing WAV fixture remains untouched."""
    # Real 80-sample silent FLAC, encoded once. Stubs need no installed encoder.
    flac = (Path(__file__).parent / "fixtures/conversion.flac").read_bytes()
    for name in ("flac", "ffmpeg"):
        tool = root / "bin" / name
        code = f'''#!{sys.executable}
import base64, json, os, sys
from pathlib import Path
from datetime import datetime, timezone
root = Path({str(root)!r})
args = sys.argv[1:]
with (root / 'converter-calls.jsonl').open('a') as log:
    log.write(json.dumps({{'tool': {name!r}, 'arguments': args, 'pid': os.getpid(), 'at': datetime.now(timezone.utc).isoformat()}}) + '\\n')
if args == [{'--version' if name == 'flac' else '-version'!r}]:
    sys.exit(0)
target = Path(args[args.index('-o') + 1] if {name!r} == 'flac' else args[-1])
if {tool_mode!r} == 'encode-failure' or ({tool_mode!r} == 'fallback' and {name!r} == 'flac'):
    target.write_bytes(b'failed partial conversion')
    print('fixture secret must not enter report', file=sys.stderr)
    sys.exit(7)
target.write_bytes(base64.b64decode({base64.b64encode(flac).decode()!r}))
'''
        temporary = tool.with_suffix(".new")
        temporary.write_text(code)
        temporary.chmod(0o700)
        temporary.replace(tool)
    (root / "converter-mode").write_text(tool_mode)
    print(f"Conversion fixture mode: {tool_mode}. Saving Settings does not retry a track; use its explicit action.")


def conversion_status(root):
    conn = sqlite3.connect(f"file:{root / 'data/library.sqlite'}?mode=ro", uri=True)
    rows = conn.execute("SELECT t.id, t.track_title, t.is_in_library, lf.path FROM tracks t LEFT JOIN local_files lf ON lf.track_id=t.id ORDER BY t.id").fetchall()
    counts = {table: conn.execute(f"SELECT count(*) FROM {table}").fetchone()[0] for table in ("tracks", "local_files", "playlist_tracks")}
    conn.close()
    requests = [json.loads(line) for line in (root / "retry-requests.jsonl").read_text().splitlines()]
    report = {"tracks": rows, "counts": counts, "audio_requests": [record for record in requests if record["request"].startswith("/conversion-")], "staging": [str(path.relative_to(root)) for path in (root / "music/.v4vmm-staging").glob("*/*")]}
    print(json.dumps(report, indent=2))
    return report


def conversion_remove_input(root):
    conn = sqlite3.connect(f"file:{root / 'data/library.sqlite'}?mode=ro", uri=True)
    rows = conn.execute("SELECT path FROM local_files WHERE track_id=5").fetchall()
    conn.close()
    if len(rows) != 1 or not rows[0][0].endswith(".wav"):
        raise SystemExit("Download Conversion redownload with failed converters first.")
    source = root / "music" / rows[0][0]
    if not source.resolve().is_relative_to((root / "music").resolve()) or source.is_symlink():
        raise SystemExit("Refusing input outside fixture music.")
    preserved = root / "removed-conversion-input.wav"
    if preserved.exists():
        raise SystemExit("The removed input is already preserved; keep it for inspection.")
    source.rename(preserved)
    print(f"Preserved fixture input at {preserved}. Retry must explain redownload before requesting the enclosure again.")


def conversion_inspect(root, manifest):
    if owned_process(root, "app.pid"):
        raise SystemExit("Close the fixture app before conversion-inspect.")
    report = conversion_status(root)
    checks = {
        "one_binding_per_track": report["counts"] == {"tracks": 5, "local_files": 5, "playlist_tracks": 5},
        "original_tracks_preserved": all(digest(root / "music" / name) == expected for name, expected in manifest["track_sha256"].items()),
        "original_bindings_preserved": [row[3] for row in report["tracks"][:3]] == ["a.wav", "b.wav", "c.wav"],
        "converted_bindings": all(row[2] == 1 and row[3].endswith(".flac") for row in report["tracks"][3:]),
        "one_reuse_and_one_explicit_redownload": [row["request"] for row in report["audio_requests"]].count("/conversion-4.wav") == 1 and [row["request"] for row in report["audio_requests"]].count("/conversion-5.wav") == 2,
        "staging_released": not report["staging"],
        "removed_wav_preserved": digest(root / "removed-conversion-input.wav") == manifest["track_sha256"]["a.wav"],
    }
    before = tomllib.loads((root / "case.config").read_text())
    after = tomllib.loads((root / "config/v4vmm/config.toml").read_text())
    for value in (before, after):
        value.pop("workspace", None)
        value.pop("workspace_layout", None)
    checks["configuration_preserved"] = before == after
    cfg = root / "config/v4vmm/config.toml"
    original_hash = json.loads((root / "case.json").read_text())["config_sha256"]
    backups = list(cfg.parent.glob(".v4vmm-config-*.backup"))
    checks["original_config_revision_preserved"] = digest(cfg) == original_hash or any(digest(path) == original_hash for path in backups)
    checks["private_backups"] = all(path.stat().st_mode & 0o777 == 0o600 for path in backups)
    checks["no_candidates_or_probes"] = not list(cfg.parent.glob(".v4vmm-config-*.candidate")) and not list((root / "music").rglob(".v4vmm-startup-probe-*"))
    saved = json.loads(subprocess.check_output([manifest["binary"], "startup-fixture", "inspect", str(root)], env=environment(root), text=True))
    checks["migrations_and_database_preserved"] = saved["migration_versions"] == manifest["migration_versions"] and saved["database_probes"] == 0 and saved["playlists"] == 1
    checks["unrelated_audio_preserved"] = digest(root / "music/unchanged-audio.bin") == manifest["audio_sha256"]
    retained_source = root / "music" / report["tracks"][3][3]
    checks["original_usable_wav_preserved"] = digest(retained_source.with_suffix(".wav")) == manifest["track_sha256"]["a.wav"]
    calls = [json.loads(line) for line in (root / "converter-calls.jsonl").read_text().splitlines()]
    checks["converter_children_reaped"] = all(not Path(f'/proc/{record["pid"]}').exists() for record in calls)
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit("Conversion preservation failed. Keep this fixture.")


def converter_inspect(root, manifest):
    if owned_process(root, "app.pid"):
        raise SystemExit("Close the fixture app before converter-inspect.")
    cfg = root / "config/v4vmm/config.toml"
    expected = json.loads((root / "case.json").read_text())
    backups = list(cfg.parent.glob(".v4vmm-config-*.backup"))
    original_preserved = digest(cfg) == expected["config_sha256"] or any(digest(path) == expected["config_sha256"] for path in backups)
    before = tomllib.loads((root / "case.config").read_text())
    after = tomllib.loads(cfg.read_text())
    path = after.pop("flac_path", None)
    before.pop("flac_path", None)
    records = converter_status(root)
    checks = {
        "original_preserved": original_preserved,
        "unedited_values_preserved": not changed_config_paths(before, after),
        "configured_path_restored": path is None,
        "only_version_probes": all(record["arguments"] == (["--version"] if record["tool"] == "flac" else ["-version"]) for record in records),
        "probe_children_reaped": all(not Path(f'/proc/{record["pid"]}').exists() for record in records),
        "owner_only_backups": all(path.stat().st_mode & 0o777 == 0o600 for path in backups),
        "no_candidates": not list(cfg.parent.glob(".v4vmm-config-*.candidate")),
        "case_copy_matches_original": digest(root / "case.config") == expected["config_sha256"],
    }
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit("Converter fixture preservation failed. Keep this fixture.")
    inspect(root, manifest, correction_preserved=True)



def database_setup(root):
    directory = root / "database"
    if not directory.exists():
        subprocess.run([str(REPO / "target/debug/v4vmm"), "startup-fixture", "database-seed", str(root)], env=environment(root), check=True)
        (directory / "readonly.sqlite").chmod(0o400)
    if not owned_process(root, "database.pid"):
        if (root / "database-baseline.json").exists():
            raise SystemExit("The previous WAL helper stopped. Keep its baseline for diagnosis and create a fresh fixture; do not rebaseline changed sources.")
        (root / "database.ready").unlink(missing_ok=True)
        with (root / "database.log").open("w") as output:
            subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "database-hold", str(root)], stdout=output, stderr=output, start_new_session=True)
        deadline = time.monotonic() + 5
        while not (root / "database.ready").exists():
            if time.monotonic() > deadline:
                raise SystemExit("Database fixture helper did not start. Inspect database.log and retain the fixture.")
            time.sleep(0.05)
    database_status(root)


def database_hold(root):
    directory = root / "database"
    wal = sqlite3.connect(directory / "wal.sqlite")
    locked = sqlite3.connect(directory / "locked.sqlite")
    wal.execute("PRAGMA journal_mode=WAL")
    wal.execute("PRAGMA wal_autocheckpoint=0")
    main_before = digest(directory / "wal.sqlite")
    wal.execute("INSERT INTO playlists(name) VALUES ('Committed WAL-only fixture row')")
    wal.commit()
    if digest(directory / "wal.sqlite") != main_before:
        raise SystemExit("WAL-only fixture row reached the main file unexpectedly.")
    state = {"source_hashes": {p.name: digest(p) for p in directory.glob("*.sqlite") if not p.name.endswith("backup.sqlite")},
             "wal_sha256": digest(directory / "wal.sqlite-wal")}
    # ADR 0066: closing a raw descriptor in this process releases SQLite's
    # POSIX locks on that file. Finish checksum reads before taking the lock.
    locked.execute("BEGIN EXCLUSIVE")
    (root / "database-baseline.json").write_text(json.dumps(state))
    (root / "database.pid").write_text(str(os.getpid()))
    (root / "database.ready").write_text("ready")
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    try:
        while True:
            time.sleep(1)
    finally:
        locked.rollback()
        locked.close()
        wal.close()
        (root / "database.ready").unlink(missing_ok=True)
        (root / "database.pid").unlink(missing_ok=True)


def database_stop(root):
    maintenance_release(root)
    for name in ("database-lock", "database"):
        pid = owned_process(root, name + ".pid")
        if pid:
            os.kill(pid, signal.SIGTERM)
            deadline = time.monotonic() + 5
            while (root / (name + ".ready")).exists():
                if time.monotonic() > deadline:
                    raise SystemExit("Database helper did not exit. Keep the fixture for diagnosis.")
                time.sleep(0.05)


def database_reader_blocked(root):
    """ADR 0066: a live helper is insufficient evidence of a held SQLite lock."""
    path = root / "database/locked.sqlite"
    conn = None
    try:
        conn = sqlite3.connect(path.as_uri() + "?mode=ro", uri=True, timeout=0.05)
        conn.execute("SELECT count(*) FROM sqlite_schema").fetchone()
        return False
    except sqlite3.DatabaseError as error:
        return getattr(error, "sqlite_errorcode", 0) & 0xff in (sqlite3.SQLITE_BUSY, sqlite3.SQLITE_LOCKED)
    finally:
        if conn is not None:
            conn.close()


def database_lock(root):
    """Restore a pre-fix fixture's lock without restarting its WAL helper."""
    if not owned_process(root, "database.pid") or not (root / "database.ready").exists():
        raise SystemExit("Keep the original WAL helper running; its baseline must not be replaced.")
    baseline = json.loads((root / "database-baseline.json").read_text())
    path = root / "database/locked.sqlite"
    if digest(path) != baseline["source_hashes"][path.name]:
        raise SystemExit("The lock source changed. Keep the fixture for diagnosis.")
    if database_reader_blocked(root):
        print("The fixture database already blocks readers; continue the app check.")
        return
    conn = sqlite3.connect(path.as_uri() + "?mode=rw", uri=True, timeout=0.5)
    registered = False
    try:
        conn.execute("BEGIN EXCLUSIVE")
        signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
        (root / "database-lock.pid").write_text(str(os.getpid()))
        registered = True
        (root / "database-lock.ready").write_text("ready")
        print("Fixture database lock held. Leave this terminal running through the checks; fixture cleanup will stop it.", flush=True)
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        pass
    finally:
        conn.rollback()
        conn.close()
        if registered:
            (root / "database-lock.ready").unlink(missing_ok=True)
            (root / "database-lock.pid").unlink(missing_ok=True)


def database_status(root):
    directory = root / "database"
    print(json.dumps({"helper_running": bool(owned_process(root, "database.pid")),
                      "exclusive_lock_blocks_reader": database_reader_blocked(root),
                      "configured_normal_database": str(root / "data/library.sqlite"),
                      "sources": {name: str(directory / (name + ".sqlite")) for name in ("wal", "readonly", "locked", "integrity", "newer", "older", "foreign-key", "invalid-header")},
                      "normal_backup": str(directory / "normal-backup.sqlite"),
                      "wal_backup": str(directory / "wal-backup.sqlite"),
                      "readonly_backup": str(directory / "readonly-backup.sqlite"),
                      "occupied_destination": str(directory / "occupied.sqlite")}, indent=2))


def database_inspect(root, manifest):
    directory = root / "database"
    if not owned_process(root, "database.pid") or not (root / "database.ready").exists():
        raise SystemExit("The WAL fixture helper must remain running through preservation inspection.")
    state = json.loads((root / "database-baseline.json").read_text())
    checks = {"exclusive_lock_blocks_reader": database_reader_blocked(root),
              "database_sources_preserved": all(digest(directory / name) == value for name, value in state["source_hashes"].items()),
              "wal_bytes_preserved": digest(directory / "wal.sqlite-wal") == state["wal_sha256"],
              "no_incomplete_candidates": not list(directory.glob(".v4vmm-database-*")),
              "readonly_permissions_preserved": (directory / "readonly.sqlite").stat().st_mode & 0o777 == 0o400}
    names = ("normal", "wal", "readonly")
    if json.loads((root / "case.json").read_text())["case"] == "database-recovery":
        names += ("recovery",)
    checks["no_failed_backup_published"] = not (directory / "locked-backup.sqlite").exists()
    for name in names:
        path = directory / (name + "-backup.sqlite")
        checks[name + "_backup_exists"] = path.is_file() and not path.is_symlink()
        if not checks[name + "_backup_exists"]:
            continue
        with sqlite3.connect(path.as_uri() + "?mode=ro", uri=True) as conn:
            checks[name + "_backup_integrity"] = conn.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
            checks[name + "_backup_foreign_keys"] = conn.execute("PRAGMA foreign_key_check").fetchall() == []
            checks[name + "_backup_schema"] = [r[0] for r in conn.execute("SELECT version FROM schema_migrations ORDER BY version")] == manifest["migration_versions"]
            if name == "wal":
                checks["committed_wal_row_in_backup"] = conn.execute("SELECT count(*) FROM playlists WHERE name='Committed WAL-only fixture row'").fetchone()[0] == 1
            if name == "normal":
                checks["normal_backup_library"] = conn.execute("SELECT count(*) FROM tracks").fetchone()[0] == 3
        checks[name + "_backup_private"] = path.stat().st_mode & 0o777 == 0o600
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit("Database preservation inspection failed. Keep the fixture for diagnosis.")
    inspect(root, manifest)


def maintenance_setup(root):
    if (root / "maintenance-baseline.json").exists():
        raise SystemExit("Maintenance fixture already has a baseline. Keep its evidence; create a fresh fixture instead of reseeding.")
    directory = root / "database"
    if directory.exists():
        raise SystemExit("Use a fresh fixture for database maintenance; existing database evidence must not be replaced.")
    subprocess.run([str(REPO / "target/debug/v4vmm"), "startup-fixture", "database-seed", str(root)], env=environment(root), check=True)
    state = {"source_hashes": {p.name: digest(p) for p in directory.glob("*.sqlite")}}
    (root / "maintenance-baseline.json").write_text(json.dumps(state, indent=2))
    with (root / "maintenance.log").open("w") as output:
        subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "maintenance-hold", str(root)], stdout=output, stderr=output, start_new_session=True)
    deadline = time.monotonic() + 5
    while not (root / "maintenance.ready").exists():
        if time.monotonic() > deadline:
            raise SystemExit("Maintenance writer did not start. Retain the fixture and inspect maintenance.log.")
        time.sleep(0.05)
    maintenance_status(root)


def maintenance_hold(root):
    """A real external writer; checksum work belongs to the parent before this lock."""
    path = root / "database/locked.sqlite"
    conn = sqlite3.connect(path.as_uri() + "?mode=rw", uri=True, timeout=0.5)
    conn.execute("BEGIN IMMEDIATE")
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    (root / "maintenance.pid").write_text(str(os.getpid()))
    (root / "maintenance.ready").write_text("ready")
    try:
        while True:
            time.sleep(1)
    finally:
        conn.rollback()
        conn.close()
        (root / "maintenance.ready").unlink(missing_ok=True)
        (root / "maintenance.pid").unlink(missing_ok=True)


def maintenance_release(root):
    pid = owned_process(root, "maintenance.pid")
    if pid:
        os.kill(pid, signal.SIGTERM)
        deadline = time.monotonic() + 5
        while (root / "maintenance.ready").exists():
            if time.monotonic() > deadline:
                raise SystemExit("Fixture writer did not release. Retain the fixture for diagnosis.")
            time.sleep(0.05)


def maintenance_writer_blocked(root):
    conn = None
    try:
        path = root / "database/locked.sqlite"
        conn = sqlite3.connect(path.as_uri() + "?mode=rw", uri=True, timeout=0.05)
        conn.execute("BEGIN EXCLUSIVE")
        return False
    except sqlite3.DatabaseError as error:
        return getattr(error, "sqlite_errorcode", 0) & 0xff in (sqlite3.SQLITE_BUSY, sqlite3.SQLITE_LOCKED)
    finally:
        if conn is not None:
            conn.rollback()
            conn.close()


def maintenance_status(root):
    directory = root / "database"
    print(json.dumps({"writer_running": bool(owned_process(root, "maintenance.pid")),
                      "exclusive_access_blocked": maintenance_writer_blocked(root),
                      "sources": {name: str(directory / (name + ".sqlite")) for name in ("locked", "integrity", "invalid-header")},
                      "configured_normal_database": str(root / "data/library.sqlite"),
                      "destinations": {name: str(directory / (name + "-copy")) for name in ("busy", "locked", "damaged", "invalid", "normal")}}, indent=2))


def maintenance_inspect(root, manifest):
    """Validate copied evidence without opening any copy with SQLite (which could recover journals)."""
    directory = root / "database"
    state = json.loads((root / "maintenance-baseline.json").read_text())
    checks = {"writer_released": not owned_process(root, "maintenance.pid"),
              "original_database_files_preserved": all(digest(directory / name) == value for name, value in state["source_hashes"].items()),
              "busy_attempt_created_no_copy": not (directory / "busy-copy").exists(),
              "invalid_header_created_no_copy": not (directory / "invalid-copy").exists()}
    copies = {"locked": directory / "locked.sqlite", "damaged": directory / "integrity.sqlite"}
    if json.loads((root / "case.json").read_text())["case"] == "database-maintenance":
        copies["normal"] = root / "data/library.sqlite"
    for name, source in copies.items():
        destination = directory / (name + "-copy")
        try:
            receipt_path = destination / "manifest.json"
            receipt = json.loads(receipt_path.read_text())
            records = receipt["files"]
            expected_sources = {str(source)} | {str(Path(str(source) + suffix)) for suffix in ("-wal", "-shm", "-journal") if Path(str(source) + suffix).exists()}
            checks[name + "_preservation_manifest"] = (receipt["kind"] == "database_file_preservation_not_verified_backup"
                and receipt["source"] == str(source) and receipt["journal_mode"] == "delete"
                and bool(receipt["sqlite_version"]) and bool(receipt["sqlite_access"])
                and datetime.fromisoformat(receipt["acquired_at_utc"]).utcoffset().total_seconds() == 0
                and datetime.fromisoformat(receipt["copied_at_utc"]).utcoffset().total_seconds() == 0
                and {entry["source"] for entry in records} == expected_sources)
            checks[name + "_private_directory"] = not destination.is_symlink() and destination.stat().st_mode & 0o777 == 0o700
            checks[name + "_private_manifest"] = not receipt_path.is_symlink() and receipt_path.stat().st_mode & 0o777 == 0o600
            checks[name + "_exact_files"] = {p.name for p in destination.iterdir()} == {"manifest.json"} | {entry["copied_name"] for entry in records}
            for index, entry in enumerate(records):
                copied = destination / entry["copied_name"]
                if Path(entry["copied_name"]).name != entry["copied_name"] or copied.is_symlink() or entry["source"] not in expected_sources:
                    raise ValueError("Manifest path escaped its source or destination")
                checks[f"{name}_file_{index}_private"] = copied.stat().st_mode & 0o777 == 0o600
                checks[f"{name}_file_{index}_length"] = copied.stat().st_size == entry["length"]
                checks[f"{name}_file_{index}_checksum"] = digest(copied) == entry["sha256"] == digest(Path(entry["source"]))
        except (OSError, ValueError, KeyError, TypeError, AttributeError):
            checks[name + "_preservation_manifest"] = False
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit("Maintenance preservation inspection failed. Keep the fixture for diagnosis.")
    inspect(root, manifest)


def restore_setup(root, case):
    directory = root / "database"
    if directory.exists():
        raise SystemExit("Use a fresh restore fixture; existing recovery artifacts cannot be replaced.")
    subprocess.run([str(REPO / "target/debug/v4vmm"), "startup-fixture", "restore-seed", str(root)], env=environment(root), check=True)
    source = root / "data/library.sqlite"
    if case == "database-restore-recovery":
        damaged = bytearray(source.read_bytes())
        damaged[32:36] = (0x7fffffff).to_bytes(4, "big")
        damaged[36:40] = (1).to_bytes(4, "big")
        source.write_bytes(damaged)
    token = root / "broadcaster-token.fixture"
    token.write_bytes(b"isolated fixture token sentinel\n")
    token.chmod(0o600)
    state = {"backup_hashes": {p.name: digest(p) for p in directory.glob("*.sqlite")},
             "destination_inode": source.stat().st_ino, "destination_device": source.stat().st_dev,
             "original_sha256": digest(source), "token_sha256": digest(token)}
    (root / "restore-baseline.json").write_text(json.dumps(state, indent=2))
    restore_status(root)


def restore_status(root):
    directory = root / "database"
    print(json.dumps({"configured_destination": str(root / "data/library.sqlite"),
                      "chosen_backup": str(directory / "restore-backup.sqlite"),
                      "invalid_backup": str(directory / "invalid-header.sqlite"),
                      "newer_backup": str(directory / "newer.sqlite"),
                      "preservation_directories": {name: str(directory / (name + "-preservation")) for name in ("busy", "blocked", "failed", "restored")},
                      "installation_interruption": (root / "restore.interrupt").exists(),
                      "writer_running": bool(owned_process(root, "lock.pid"))}, indent=2))


def restore_database_facts(path):
    with sqlite3.connect(path.as_uri() + "?mode=ro", uri=True) as conn:
        return {"integrity": conn.execute("PRAGMA integrity_check").fetchall(),
                "foreign_keys": conn.execute("PRAGMA foreign_key_check").fetchall(),
                "playlists": conn.execute("SELECT id,name,description FROM playlists ORDER BY id").fetchall(),
                "versions": conn.execute("SELECT * FROM schema_migrations ORDER BY version").fetchall(),
                "bindings": conn.execute("SELECT path,track_id FROM local_files ORDER BY track_id").fetchall(),
                "tracks": conn.execute("SELECT * FROM tracks ORDER BY id").fetchall(),
                "memberships": conn.execute("SELECT * FROM playlist_tracks ORDER BY playlist_id,position").fetchall(),
                "probes": conn.execute("SELECT count(*) FROM sqlite_schema WHERE name LIKE 'v4vmm_startup_probe_%'").fetchone()[0]}

def restore_inspect(root, manifest):
    if owned_process(root, "app.pid") or owned_process(root, "lock.pid"):
        raise SystemExit("Close the fixture app and release its writer before restore inspection.")
    directory = root / "database"
    state = json.loads((root / "restore-baseline.json").read_text())
    case = json.loads((root / "case.json").read_text())
    source = root / "data/library.sqlite"
    config = root / "config/v4vmm/config.toml"
    current = config.read_bytes()
    checks = {"chosen_and_rejected_backups_unchanged": all(digest(directory / name) == value for name, value in state["backup_hashes"].items()),
              "configured_database_inode_preserved": (source.stat().st_ino, source.stat().st_dev) == (state["destination_inode"], state["destination_device"]),
              "music_files_unchanged": digest(root / "music/unchanged-audio.bin") == manifest["audio_sha256"] and all(digest(root / "music" / name) == value for name, value in manifest["track_sha256"].items()),
              "token_file_unchanged": digest(root / "broadcaster-token.fixture") == state["token_sha256"] and (root / "broadcaster-token.fixture").stat().st_mode & 0o777 == 0o600,
              "config_preserved": digest(config) == case["config_sha256"] or normal_workspace_preferences_only(root, case, current),
              "interruption_removed": not (root / "restore.interrupt").exists(),
              "busy_created_no_preservation": not (directory / "busy-preservation").exists()}
    restored = restore_database_facts(source)
    checks["restored_database_matches_chosen_records_and_ledger"] = restored == restore_database_facts(directory / "restore-backup.sqlite")
    checks["restored_database_valid"] = restored["integrity"] == [("ok",)] and not restored["foreign_keys"] and not restored["probes"]
    names = ("failed", "restored") if case["case"] == "database-restore" else ("restored",)
    if case["case"] == "database-restore":
        checks["occupied_preservation_file_unchanged"] = (directory / "blocked-preservation").read_text() == "Retain this occupied preservation destination.\n"
    for name in names:
        folder = directory / (name + "-preservation")
        try:
            receipt_path = folder / "manifest.json"
            receipt = json.loads(receipt_path.read_text())
            checks[name + "_private_preservation"] = not folder.is_symlink() and folder.stat().st_mode & 0o777 == 0o700 and receipt_path.stat().st_mode & 0o777 == 0o600
            checks[name + "_manifest"] = receipt["kind"] == "database_file_preservation_not_verified_backup" and receipt["source"] == str(source) and bool(receipt["files"])
            for index, entry in enumerate(receipt["files"]):
                path = folder / entry["copied_name"]
                if Path(entry["copied_name"]).name != entry["copied_name"] or path.is_symlink():
                    raise ValueError("Preservation file escaped its directory")
                checks[f"{name}_file_{index}"] = path.stat().st_size == entry["length"] and digest(path) == entry["sha256"] and path.stat().st_mode & 0o777 == 0o600
            snapshots = list(folder.glob(".v4vmm-database-*/candidate.sqlite"))
            if case["case"] == "database-restore":
                checks[name + "_verified_original"] = len(snapshots) == 1
                if snapshots:
                    original = restore_database_facts(snapshots[0])
                    checks[name + "_original_records"] = len(original["playlists"]) == 1 and original["playlists"][0][1] == "Startup fixture playlist" and original["versions"] == restored["versions"] and original["tracks"] == restored["tracks"] and original["bindings"] == restored["bindings"] and original["integrity"] == [("ok",)] and not original["probes"]
            else:
                checks[name + "_damaged_original_bytes"] = digest(folder / "database.sqlite") == state["original_sha256"] and not snapshots
        except (OSError, ValueError, KeyError, sqlite3.Error):
            checks[name + "_preservation"] = False
    observations = root / "session-observations.jsonl"
    if observations.exists():
        sessions = [json.loads(line) for line in observations.read_text().splitlines()]
        opened = [entry["generation"] for entry in sessions if entry["state"] == "opened"]
        checks["fresh_session_observed"] = len(opened) == len(set(opened)) and len(opened) >= (2 if case["case"] == "database-restore" else 1)
    else:
        checks["fresh_session_observed"] = False
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit("Restore preservation inspection failed. Keep the fixture for diagnosis.")


def setup():
    binary = REPO / "target/debug/v4vmm"
    if not binary.is_file():
        raise SystemExit("Run cargo build --quiet first.")
    root = Path(tempfile.mkdtemp(prefix="v4vmm-startup-"))
    manifest = {"kind": KIND, "root": str(root), "binary": str(binary)}
    (root / "fixture.json").write_text(json.dumps(manifest))
    for part in ("home", "bin", "cache"):
        (root / part).mkdir()
    # Every external control command fails locally; none can reach real services.
    for name in ("systemctl", "journalctl", "musicindex-live-publisher", "butt", "mpv"):
        stub = root / "bin" / name
        stub.write_text("#!/bin/sh\nprintf '%s\\n' 'Startup fixture: optional service unavailable' >&2\nexit 1\n")
        stub.chmod(0o700)
    subprocess.run([str(binary), "startup-fixture", "seed", str(root)], env=environment(root), check=True, stdout=subprocess.DEVNULL)
    cfg = root / "config/v4vmm/config.toml"
    shutil.copyfile(cfg, root / "config.baseline")
    manifest["audio_sha256"] = digest(root / "music/unchanged-audio.bin")
    manifest["track_sha256"] = {name: digest(root / "music" / name) for name in ("a.wav", "b.wav", "c.wav")}
    output = subprocess.check_output([str(binary), "startup-fixture", "inspect", str(root)], env=environment(root), text=True)
    manifest["migration_versions"] = json.loads(output)["migration_versions"]
    (root / "fixture.json").write_text(json.dumps(manifest, indent=2))
    (root / "case.json").write_text(json.dumps({"case": "normal", "config_sha256": digest(cfg)}))
    shutil.copyfile(cfg, root / "case.config")
    print(root)
    print("Fixture created. Use verify, mode, run and inspect with this exact directory.", file=sys.stderr)


def run_app(root, manifest):
    if owned_process(root, "app.pid"):
        raise SystemExit("This fixture app is already open.")
    # ADR 0066 fixture guard: cargo test can replace target/debug/v4vmm with
    # a binary linked to GPUI test-support. Its synchronous test drawing loop
    # must not be used for desktop acceptance (ADR 0044 drag-pause capture).
    # Build before applying the fixture's isolated HOME and stub-only PATH.
    try:
        subprocess.run(
            ["cargo", "build", "--locked", "--offline", "--quiet", "--bin", "v4vmm",
             "--target-dir", str(REPO / "target")],
            cwd=REPO, check=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Desktop build failed; fixture app was not opened.\n{error}") from error
    child = subprocess.Popen([manifest["binary"]], env=environment(root))
    (root / "app.pid").write_text(str(child.pid))
    try:
        try:
            code = child.wait()
        except KeyboardInterrupt:
            child.terminate()
            code = child.wait()
        expected = json.loads((root / "case.json").read_text())
        (root / "last-exit.json").write_text(json.dumps({"code": code, "config_sha256": expected["config_sha256"]}))
        print(f"Fixture app exit code: {code}")
    finally:
        if child.poll() is not None:
            (root / "app.pid").unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("setup", "locate", "verify", "validate", "mode", "run", "inspect", "cleanup", "hold-lock", "session-status", "session-release", "repair-access", "repair-conflict", "repair-inspect", "retry-status", "retry-release", "retry-inspect", "retry-server", "converter-tools", "converter-status", "converter-inspect", "conversion-tools", "conversion-status", "conversion-remove-input", "conversion-inspect", "database-hold", "database-lock", "database-status", "database-inspect", "maintenance-hold", "maintenance-release", "maintenance-status", "maintenance-inspect", "restore-status", "restore-inspect", "restore-lock", "restore-release", "restore-block", "restore-interrupt", "restore-ready"))
    parser.add_argument("directory", nargs="?")
    parser.add_argument("case", nargs="?", choices=CASES + CONVERTER_MODES + CONVERSION_MODES)
    args = parser.parse_args()
    if args.command == "setup":
        setup()
        return
    if args.command == "locate":
        found = []
        for path in Path(tempfile.gettempdir()).glob("v4vmm-startup-*/fixture.json"):
            try:
                root, _ = verify(path.parent)
                found.append(root)
            except SystemExit:
                continue
        if not found:
            raise SystemExit("No verified startup fixture found. Run setup in this desktop session.")
        print(max(found, key=lambda p: p.stat().st_mtime))
        return
    if not args.directory:
        parser.error("directory is required; use locate if you lost the path")
    root, manifest = verify(args.directory)
    if args.command in ("verify", "validate"):
        print(f"Fixture verified: {root}\nConfig: {root / 'config/v4vmm/config.toml'}\nDatabase: {root / 'data/library.sqlite'}\nMusic: {root / 'music'}\nBinary: {manifest['binary']}")
    elif args.command == "mode":
        if args.case not in CASES:
            parser.error("mode requires a case")
        mode(root, args.case)
    elif args.command == "database-hold":
        database_hold(root)
    elif args.command == "database-lock":
        database_lock(root)
    elif args.command == "database-status":
        database_status(root)
    elif args.command == "database-inspect":
        database_inspect(root, manifest)
    elif args.command.startswith("restore-"):
        if json.loads((root / "case.json").read_text())["case"] not in RESTORE_CASES:
            raise SystemExit("Select a fresh database-restore or database-restore-recovery fixture first.")
        if args.command == "restore-status":
            restore_status(root)
        elif args.command == "restore-inspect":
            restore_inspect(root, manifest)
        elif args.command == "restore-lock":
            if owned_process(root, "lock.pid"):
                raise SystemExit("Fixture writer is already running.")
            with (root / "restore-lock.log").open("w") as output:
                subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "hold-lock", str(root)], stdout=output, stderr=output, start_new_session=True)
            deadline = time.monotonic() + 5
            while not (root / "lock.ready").exists():
                if time.monotonic() > deadline:
                    raise SystemExit("Restore writer did not start. Inspect restore-lock.log.")
                time.sleep(0.05)
            print("Fixture writer holds the configured database. Restore must refuse exclusive access.")
        elif args.command == "restore-release":
            release_lock(root)
            print("Fixture database writer released.")
        elif args.command == "restore-block":
            with (root / "database/blocked-preservation").open("x") as output:
                output.write("Retain this occupied preservation destination.\n")
            print("Preservation destination occupied after review. Restore must stop before installation.")
        elif args.command == "restore-interrupt":
            (root / "restore.interrupt").write_text("Interrupt only the verified debug fixture's restore.\n")
            print("Fixture restore will fail between SQLite page batches. Review with failed-preservation.")
        elif args.command == "restore-ready":
            (root / "restore.interrupt").unlink(missing_ok=True)
            print("Fixture installation interruption removed. Review with restored-preservation.")
    elif args.command == "maintenance-hold":
        maintenance_hold(root)
    elif args.command == "maintenance-release":
        maintenance_release(root)
        maintenance_status(root)
    elif args.command == "maintenance-status":
        maintenance_status(root)
    elif args.command == "maintenance-inspect":
        maintenance_inspect(root, manifest)
    elif args.command.startswith("conversion-"):
        if json.loads((root / "case.json").read_text())["case"] not in CONVERSION_CASES:
            raise SystemExit("Select conversion-retry first.")
        if args.command == "conversion-tools":
            if args.case not in CONVERSION_MODES:
                parser.error("conversion-tools requires encode-failure, working or fallback")
            conversion_tools(root, args.case)
        elif args.command == "conversion-status":
            conversion_status(root)
        elif args.command == "conversion-remove-input":
            conversion_remove_input(root)
        else:
            conversion_inspect(root, manifest)
    elif args.command in ("converter-tools", "converter-status", "converter-inspect"):
        if json.loads((root / "case.json").read_text())["case"] not in CONVERTER_CASES:
            raise SystemExit("Select converter-setup or converter-recovery first.")
        if args.command == "converter-tools":
            if args.case not in CONVERTER_MODES:
                parser.error("converter-tools requires a converter mode")
            converter_tools(root, args.case)
        elif args.command == "converter-status":
            converter_status(root)
        else:
            converter_inspect(root, manifest)
    elif args.command == "session-status":
        report = root / "session-observations.jsonl"
        observations = [json.loads(line) for line in report.read_text().splitlines()] if report.exists() else []
        print(json.dumps({"held": (root / "session.hold").exists(), "observations": observations}, indent=2))
    elif args.command == "session-release":
        (root / "session.hold").unlink(missing_ok=True)
        print("Released the fixture session command; use Retry drain in the app.")
    elif args.command == "repair-access":
        cfg = root / "config/v4vmm/config.toml"
        cfg.parent.chmod(0o700)
        if cfg.stat().st_mode & 0o777 != 0o600:
            cfg.chmod(0o600)
        print("Restored fixture configuration read/write access. App may reload or explicitly save its retained draft.")
    elif args.command == "repair-conflict":
        if json.loads((root / "case.json").read_text())["case"] != "repair-conflict":
            raise SystemExit("Select the repair-conflict case before the external edit.")
        cfg = root / "config/v4vmm/config.toml"
        text = cfg.read_text().replace('musicindex_endpoint = 42', 'musicindex_endpoint = 99')
        cfg.write_text(text)
        (root / "repair.external").write_bytes(cfg.read_bytes())
        print("External fixture editor changed configuration. Save in the app must retain this revision and its unsaved draft.")
    elif args.command == "retry-server":
        retry_server(root)
    elif args.command == "retry-status":
        retry_status(root)
    elif args.command == "retry-release":
        if json.loads((root / "case.json").read_text())["case"] not in RETRY_CASES:
            raise SystemExit("Select retry-actions before changing its service stub.")
        (root / "retry-services-ready").write_text("Only fixture service commands may now succeed.\n")
        retry_status(root)
    elif args.command == "retry-inspect":
        retry_inspect(root, manifest)
    elif args.command == "repair-inspect":
        repair_inspect(root, manifest)
    elif args.command == "inspect":
        inspect(root, manifest)
    elif args.command == "hold-lock":
        hold_lock(root)
    elif args.command == "run":
        run_app(root, manifest)
    elif args.command == "cleanup":
        if owned_process(root, "app.pid"):
            raise SystemExit("Close the fixture app before cleanup.")
        release_lock(root)
        database_stop(root)
        stop_retry_server(root)
        (root / "config/v4vmm").chmod(0o700)
        shutil.rmtree(root)
        print(f"Removed fixture: {root}")


if __name__ == "__main__":
    main()
