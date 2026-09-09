# ADR 0064: Local File Addressing

## Status

Accepted - 2026-09-08.

## Context

`local_files.path` holds an absolute path. Every writer passes one, and every
reader uses the column without change.

An operator moved the configured music folder on 2026-09-08. The broadcast
readiness report then counted 54 tracks as `Missing file` while every file was
present under the new folder. Two tracks were ready and 17 more were readable,
because the app downloaded those after the move.

The library is tied to one folder on one machine. A move orphans every row that
came before it, and nothing in the app repairs that.

The stored value also says nothing about what it means. A reader can not tell a
path that is in the library from one that is not.

## Decision

### A Downloaded File Lives Under The Music Folder

Every file that this app downloads lives under `music_dir`. There is no second
location.

An operator who wants a file from another place copies it into `music_dir`
first. The app does not link to a file outside that folder.

### The Stored Path Is Relative To The Music Folder

`local_files.path` holds a path relative to `music_dir`, with no leading
separator and no `..` segment.

A reader resolves it with `music_dir.join(path)`. **No reader uses the column
without resolving it.** A writer takes an absolute path, checks that it is under
`music_dir`, and stores the remainder.

A path that is not under `music_dir` is a write error. The writer reports it and
stores nothing.

A folder move then costs one configuration edit and nothing else.

### One Resolver, No Second Site

One function resolves a stored path against the configured folder, and one
function makes a stored path from an absolute one. Every read and every write
goes through them.

This is the same rule ADR 0058 applies to the HTTP client. A second
construction site is where the two shapes drift apart.

### The Conversion Is A Repair Step, Not A Schema Migration

`migrate_schema` takes a `Connection` and nothing else. It does not see
`music_dir`, so it can not convert a path.

The conversion runs after the configuration loads. It is idempotent, and it runs
on every start until no absolute row is left.

For each absolute row it takes the longest trailing part of the path that names
an existing file under `music_dir`, and stores that. That repairs a move which
kept the folder layout, which is the common case.

A row it can not resolve is **reported, not deleted silently**. The app removes
the `local_files` row so the track reads as not downloaded, and it records the
old path in the repair report, so the operator can move the file and download
again.

## Invariants

- `local_files.path` is relative, with no leading separator and no `..`.
- Every read resolves through the one resolver. No caller joins by hand.
- A write of a path outside `music_dir` fails and stores nothing.
- The repair step is idempotent and needs no operator action to run.
- An unresolved row is reported before it is removed.

## Alternatives Considered

### Keep Absolute Paths And Add A Relocate Command

Rejected. It repairs one move and leaves the model unchanged, so the next move
breaks the library again. It also asks the operator to know that a command
exists at the moment the app looks broken.

### Relative Under The Music Folder, Absolute Outside It

Rejected. Two shapes in one column mean every reader tests which shape it holds,
and a reader that forgets the test fails only for the operators who keep files
in two places.

### Store A Content Checksum And Search For The File

Rejected for now. It repairs any move, including one that changes the layout,
and it costs a scan of the whole folder. Reconsider it if the trailing-part
match proves too weak.

## Consequences

- `local_files.path` changes meaning. Every reader and writer changes with it.
- `mark_track_downloaded` and its by-match variant take a path under `music_dir`
  and fail otherwise.
- Playback, readiness, the queue, and the drop-file producer all resolve.
- The repair step needs a report surface, so an operator learns which files to
  move.
- Changing `music_dir` becomes a supported action rather than a silent break.

## Follow-Up Work

- Decide whether an operator can change `music_dir` from `Settings`, and what
  the app does with the old folder.
- Decide whether a checksum column is worth adding for a layout-changing move.

## References

- ADR 0058, outbound HTTP client policy, for the one-construction-site rule
- ADR 0059, broadcast control surface, for the readiness report that found this
- `src/db.rs`, `local_files` and `migrate_schema`
- `docs/pending-human-checks.md`
