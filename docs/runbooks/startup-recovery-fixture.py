#!/usr/bin/env python3
"""Isolated operator fixture for ADR 0066. The run command opens the GUI."""
import argparse
import hashlib
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
CASES = ("normal", "invalid-toml", "music-missing", "music-file", "db-locked", "long-path")


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
               PATH=str(root / "bin") + os.pathsep + os.defpath)
    # Keep XDG_RUNTIME_DIR/DISPLAY for the operator's desktop connection.
    return env


def inspect(root, manifest):
    cfg = root / "config/v4vmm/config.toml"
    expected = json.loads((root / "case.json").read_text())
    audio = root / ("music.saved" if (root / "music.saved").exists() else "music") / "unchanged-audio.bin"
    unchanged = digest(cfg) == expected["config_sha256"]
    normal_preferences_only = False
    last_exit = root / "last-exit.json"
    if not unchanged and last_exit.exists():
        record = json.loads(last_exit.read_text())
        if record == {"code": 0, "config_sha256": expected["config_sha256"]}:
            before = tomllib.loads((root / "config.baseline").read_text())
            after = tomllib.loads(cfg.read_text())
            for key in ("workspace", "workspace_layout"):
                before.pop(key, None)
                after.pop(key, None)
            normal_preferences_only = before == after
    result = {"case": expected["case"], "config_bytes_unchanged": unchanged,
              "normal_workspace_preferences_only": normal_preferences_only,
              "config_preserved": unchanged or normal_preferences_only,
              "music_preserved": digest(audio) == manifest["audio_sha256"],
              "residual_music_probes": [str(p.relative_to(root)) for p in root.rglob(".v4vmm-startup-probe-*")]}
    completed = subprocess.run([manifest["binary"], "startup-fixture", "inspect", str(root)],
                               env=environment(root), text=True, capture_output=True, check=True)
    result.update(json.loads(completed.stdout))
    result["migration_records_preserved"] = result["migration_versions"] == manifest["migration_versions"]
    print(json.dumps(result, indent=2))
    if not all(result[key] for key in ("config_preserved", "music_preserved", "migration_records_preserved")) or result["residual_music_probes"] or result["database_probes"] or result["playlists"] != 1:
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
    release_lock(root)
    if (root / "music.saved").exists():
        if (root / "music").is_file():
            (root / "music").unlink()
        (root / "music.saved").rename(root / "music")
    cfg = root / "config/v4vmm/config.toml"
    cfg.write_bytes((root / "config.baseline").read_bytes())
    if case == "invalid-toml":
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
    else:
        purpose = "Fixture files are restored. If recovery is open, press Check again, then Open app. Otherwise launch the fixture."
    (root / "case.json").write_text(json.dumps({"case": case, "config_sha256": digest(cfg)}))
    print(purpose)


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
    output = subprocess.check_output([str(binary), "startup-fixture", "inspect", str(root)], env=environment(root), text=True)
    manifest["migration_versions"] = json.loads(output)["migration_versions"]
    (root / "fixture.json").write_text(json.dumps(manifest, indent=2))
    (root / "case.json").write_text(json.dumps({"case": "normal", "config_sha256": digest(cfg)}))
    print(root)
    print("Fixture created. Use verify, mode, run and inspect with this exact directory.", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("setup", "locate", "verify", "validate", "mode", "run", "inspect", "cleanup", "hold-lock"))
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
    elif args.command == "inspect":
        inspect(root, manifest)
    elif args.command == "hold-lock":
        hold_lock(root)
    elif args.command == "run":
        if owned_process(root, "app.pid"):
            raise SystemExit("This fixture app is already open.")
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
    elif args.command == "cleanup":
        if owned_process(root, "app.pid"):
            raise SystemExit("Close the fixture app before cleanup.")
        release_lock(root)
        shutil.rmtree(root)
        print(f"Removed fixture: {root}")


if __name__ == "__main__":
    main()
