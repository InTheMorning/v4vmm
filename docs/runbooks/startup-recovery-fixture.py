#!/usr/bin/env python3
"""Isolated ADR 0066 fixture. Only run opens the GUI."""
import argparse
import hashlib
import http.server
from datetime import datetime, timezone
import json
import os
from pathlib import Path
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
CASES += ("session-held-command",) + REPAIR_CASES + RETRY_CASES


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
        for relative in ("config", "data", "music", "home", "bin", "music.saved"):
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
    if (case in OPTIONAL_CASES + REPAIR_CASES + RETRY_CASES or previous in OPTIONAL_CASES + REPAIR_CASES + RETRY_CASES or "session-held-command" in (case, previous)) and owned_process(root, "app.pid"):
        raise SystemExit("Close the fixture app before changing optional-tool or session cases.")
    if previous in RETRY_CASES:
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
    if case in RETRY_CASES:
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
            body = b'{"data":[],"pagination":{"has_more":false}}'
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
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
    parser.add_argument("command", choices=("setup", "locate", "verify", "validate", "mode", "run", "inspect", "cleanup", "hold-lock", "session-status", "session-release", "repair-access", "repair-conflict", "repair-inspect", "retry-status", "retry-release", "retry-inspect", "retry-server"))
    parser.add_argument("directory", nargs="?")
    parser.add_argument("case", nargs="?", choices=CASES)
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
        if not args.case:
            parser.error("mode requires a case")
        mode(root, args.case)
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
        stop_retry_server(root)
        (root / "config/v4vmm").chmod(0o700)
        shutil.rmtree(root)
        print(f"Removed fixture: {root}")


if __name__ == "__main__":
    main()
