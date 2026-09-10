# Inherited UI Acceptance Checks

## Purpose

Walk only the requirements retained by the
[2026-09-10 governance review](../reviews/2026-09-10-governance-reconciliation.md).
Run one check at a time. A missing fixture is not a pass. These checks do not
reopen ADR 0059 task 017 or require recreating its broadcast fixture.

## Preparation

Use a Linux desktop session, Python 3.11 or later, and a built app. The agent
does not launch the app. The commands below copy the current database and audio
library into a disposable directory. They need space for that audio copy when
the filesystem cannot share its unchanged blocks. Close the normal app first.

Build from the repository root:

```bash
cargo build --offline
```

Create a private database copy and a minimal configuration. The configuration
uses the existing MusicIndex endpoint, a separate music directory, and no
configured playback driver or broadcast hosts from the normal configuration.
The source database is opened read-only. No token contents are copied.

```bash
gate_dir=$(mktemp -d /tmp/v4vmm-governance.XXXXXXXX)
```

```bash
python3 -c 'import json, os, pathlib, sqlite3, sys, tomllib; root=pathlib.Path(sys.argv[1]).resolve(); source=pathlib.Path(os.environ.get("XDG_CONFIG_HOME", str(pathlib.Path.home()/".config")))/"v4vmm/config.toml"; cfg=tomllib.loads(source.read_text()); music=pathlib.Path(cfg["music_dir"]).resolve(); assert music.is_dir(), "Source music directory is missing"; target=root/"config/v4vmm"; target.mkdir(parents=True); (root/"music").mkdir(); src=sqlite3.connect(pathlib.Path(cfg["db_path"]).resolve().as_uri()+"?mode=ro", uri=True, timeout=5); dst=sqlite3.connect(root/"app.sqlite", timeout=5); src.backup(dst); dst.close(); src.close(); settings={"music_dir":str(root/"music"), "db_path":str(root/"app.sqlite")}; settings.update({"musicindex_endpoint":cfg["musicindex_endpoint"]} if "musicindex_endpoint" in cfg else {}); (target/"config.toml").write_text("\n".join(k+" = "+json.dumps(v) for k,v in settings.items())+"\n"); (root/"source-music-path").write_text(str(music)); (root/".governance-fixture").write_text(str(root)); print("Database copy ready:", root)' "$gate_dir"
```

Copy audio files, including the contents of any symbolic links. Reflinks are
independent copies; a file change in this fixture must not change the source.
Wait for both commands to succeed before launching the fixture.

```bash
gate_music=$(cat "$gate_dir/source-music-path")
cp -aL --reflink=auto -- "$gate_music/." "$gate_dir/music/"
```

Launch the fixture from that terminal:

```bash
XDG_CONFIG_HOME="$gate_dir/config" XDG_DATA_HOME="$gate_dir/data" target/debug/v4vmm
```

Use Settings to select Light or Dark when a check names a theme. Perform the
check in both. Keep the default medium UI scale. A successful build is not
visual proof. Record which check and theme passed; do not use one unqualified
pass to close all sections.

## Search Toolbar — ADR 0043 Task 004

Purpose: confirm search stays readable and usable when the window narrows.
Needs a query with a known local result and a reachable MusicIndex endpoint
for the Index path.

1. Open Music. At about 1400 pixels wide, enter the known query in the toolbar and
   submit with Enter. Inspect the input, clear control, and Search action.
2. Use frame navigation to return, then submit using the toolbar Search
   action. Results must open in Music's content area. There is no Search tab.
3. Use the app's Find command and its keyboard shortcut to focus the same
   input. The current source binding is `cmd-f` in `src/app/keyboard.rs`;
   use the platform's mapping of that modifier, not an assumed Ctrl binding.
4. Narrow the window to about 560 pixels, as in task 017's inspection. The input
   and the available compact Search action must remain identifiable and
   clickable, with no partial labels or overlapping controls. Open any
   compact menu to inspect its contents.
5. Repeat in the other theme. A clipped input/action, lost focus, or a second
   global search field is wrong. Do not require a toolbar player, global
   source-scope buttons, or a separate Recent Feeds destination.

## Identity And Detail Parity — ADR 0037 Tasks 001 And 002

Purpose: confirm the same source facts produce consistent identity controls
through local and Index routes. Needs a feed with Website, Nostr, and RSS
facts, plus a downloaded track with Website and Nostr facts. Confirm the facts
are actually present before using an absent button as evidence of a defect.
The earlier `The Heycitizen Experience` feed is a candidate, not a guaranteed
current fixture. Record the feed and track identifiers used.

1. Open that release from Music's local library. Record its Website, Nostr,
   and RSS controls and their targets. Return through frame navigation.
2. Search for the same release and open its Index result. Compare the
   identity controls. Website/RSS must open the recorded addresses; Nostr
   must copy the recorded value. Paste into a scratch editor to compare.
3. Repeat with the same track through local and Index routes. Compare title,
   summary, identity controls, and section order. Compare only facts present
   in both source records; do not merge different source claims into one truth.
4. Expand the downloaded track's advanced panels. Their contextual
   availability may differ for a non-downloaded Index track under ADR 0047.
   Shared identity controls must not disappear when disclosure changes.
5. Repeat in the other theme. A locally stored identity fact missing only
   from the local route, unreadable controls, or a route-specific duplicate
   layout fails the relevant task. Capture feed and track evidence separately.

## Playlist Reordering — ADR 0044 Task 003

Purpose: confirm a drag moves the intended row and updates the open playlist.
Needs a fixture playlist with at least four downloaded tracks, plus a row
whose audio is unavailable. Use the disposable copy for removals.

1. Open the playlist in Music. Drag from its handle upward and downward.
   Before dropping, the insertion line must show the destination edge. On
   drop, the order must change in place without a navigation round trip or
   a placeholder flash that needs mouse motion to clear.
2. Drop in the original slot and outside the playlist. Neither changes the
   order. The row body must keep its normal selection behavior.
3. Use row Actions → Move Up and Move Down. The first/last boundary action
   must be unavailable. Inspect handle, menu, and unavailable-row legibility.
4. Open a track, remove it from the fixture library, then return using frame
   Back or the playlist breadcrumb. Its availability must reflect the change
   without leaving and reopening the playlist. There is no inspector-owned
   Back to Playlist button to test.
5. Repeat in the other theme. Wrong insertion edges, unintended moves,
   invisible feedback, or stale mounted rows fail the check. If the fixture
   has no unavailable row, that part remains open.

## Metadata Hydration — ADR 0054 Tasks 004 And 005

Purpose: confirm stored metadata remains useful when the Index cannot answer.
Needs known persisted feed facts (publisher, release kind/date, language,
explicit state, description) and track facts (publisher, description,
publication date, explicit state). Use records with those facts already
imported; an empty field is not evidence of lost data unless the source fact
is known to exist.

1. In Music, open the local feed and track. Record the visible values and
   their source labels. Compare the corresponding Index details while the
   endpoint is reachable. Distinct source claims must remain distinct.
2. Record the fixture's MusicIndex endpoint from Settings. Change only that
   fixture setting to `http://127.0.0.1:9`, with no listener on that port.
   Close and relaunch the fixture with the same launch command above.
3. Open the local feed and track again. Stored facts must remain readable.
   A failed Index request must not replace known local facts with blank or
   invented content. Inspect the expanded metadata and description sections.
4. Restore the recorded endpoint. Repeat in the other theme. Record feed
   and track outcomes separately. This checks hydration, not payment-tag
   repair or the Show Event diagnostics accepted under later packets.

## Scroll Containers — ADR 0030 Task 006

Purpose: confirm every long Music detail and Settings pane can reach its end.
Needs enough rows or text to overflow each pane. A pane that does not overflow
does not prove scrolling.

1. Open an overflowing artist, release, playlist, and track detail in Music.
   Also inspect an overflowing Index-origin detail and Settings pane.
2. Use the wheel, scrollbar, and the focused pane's supported keyboard
   scrolling. Reach the first and last item or line, then return upward.
3. Resize the pane and repeat in both themes. A trapped inner scrollbar,
   unreachable end, lost focus, or a parent scrolling instead of the intended
   content pane fails. Do not recreate retired Discovery/Recent Feeds panes.

## Cleanup And Recording

Close the fixture app and any browser pages or scratch editor opened by the
checks. Only after the app has closed, verify the directory marker and remove
the disposable database/config/audio copy:

```bash
python3 -c 'import pathlib, sys; root=pathlib.Path(sys.argv[1]).resolve(); assert root.parent==pathlib.Path("/tmp") and root.name.startswith("v4vmm-governance."); assert (root/".governance-fixture").read_text()==str(root); print("Fixture verified:", root)' "$gate_dir" && rm -r -- "$gate_dir"
```

For each accepted check, update its task Status, review checklist, and delivery
row, then remove only that entry from pending human checks. Keep unmet fixture
requirements and unwalked themes open. No real service or relay change is
needed for this runbook.
