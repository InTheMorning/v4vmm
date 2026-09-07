# ADR 0059 Task 015: Stream Encoder Section

## Goal

Add the `Stream` section for the `butt` encoder. Report the connection state
and the recording state, and offer connect and disconnect.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `src/broadcast/control.rs` (the command runner and the state pattern)
- `src/broadcast/transport.rs`
- `src/runtime/broadcast_service_watch.rs`
- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/broadcast/encoder.rs` (new)
- `src/broadcast/mod.rs`
- `src/config.rs`
- `src/runtime/broadcast_service_watch.rs`
- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `src/app/broadcast.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/producer.rs`
- `src/broadcast/registry.rs`
- `src/api.rs`, `src/db.rs`
- The drop-file contract

## Constraints

- `butt` has its own control interface. Use it. Do not use `systemctl` for the
  encoder, and do not use `ssh` for it.
  - `-S` requests status
  - `-s [name]` connects to a server
  - `-d` disconnects
  - `-r` starts recording and `-t` stops recording
  - `-a <addr>` and `-p <port>` address a running instance over the network
- **A local encoder and a remote encoder use one code path.** The only
  difference is whether the address and port options are present. This is why
  the section does not need the `ssh` transport of task 010.
- **The app never sends a song title to the encoder.** `-u` exists and is
  deliberately unused. The producer already writes the text file that the
  encoder reads, and a second writer would fight it.
- `butt` absent from the host is an empty state, not an error, in the shape of
  the publisher section.
- Encoder state is separate from publisher state. A connected encoder with a
  stopped publisher is a real and useful condition to see.
- Parse the status output defensively. It is a human-facing text format that
  can change between versions. An unparsed line becomes `Unknown`, never a
  panic.

## Implementation Steps

1. Add `src/broadcast/encoder.rs` with an `EncoderTarget` that holds an
   optional address and port, and the binary path.
2. Add `status()`, `connect(server)`, `disconnect()`, `start_recording()`, and
   `stop_recording()`, each through the existing command runner.
3. Define `EncoderState`:
   - `Connected`, with the server name when the status reports one
   - `Disconnected`
   - `NotInstalled` when the binary is absent
   - `NotReachable` when an addressed instance does not answer
   - `Unknown` when the output does not parse
4. Define `RecordingState` with `Recording`, `Stopped`, and `Unknown`.
5. Add an `[broadcast.encoder]` config group with the binary path, an optional
   address, an optional port, and a default server name. A missing group means
   the section reports `NotInstalled` and offers nothing.
6. Extend the service watch actor to poll the encoder status on the same
   interval as the publisher units.
7. Extend the view model with a `StreamSectionDisplay` that holds the encoder
   state, the recording state, the server label, and typed connect and
   disconnect actions.
8. Render the fourth section in the shell, after `Event`, with the shared
   section composite and the shared button primitive.
9. Add unit tests with recorded `butt -S` output for connected, disconnected,
   recording, and an unparsable line. Add a test for a missing binary and for
   an addressed instance that does not answer.
10. Add a guard: only `src/broadcast/encoder.rs` runs the encoder binary, and
    no file passes the `-u` option.
11. Capture a screenshot of the section connected, disconnected, and with the
    encoder not installed.

## Acceptance Criteria

- The section reports the connection state and the recording state.
- Connect and disconnect work for a local instance and for an addressed
  instance.
- A missing binary shows an empty state and no error text.
- Unparsable status output becomes `Unknown` and never panics.
- No code path passes `-u` to the encoder.
- The section is independent of the publisher state.
- Screenshots exist for the three states above.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast::encoder --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` for the visual check

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Encoder version tested against
5. Screenshots captured
6. Deviations from task
7. Unresolved concerns

## Escalation Triggers

- The installed `butt` version does not accept the control options this task
  names. Report the version and the actual option list before you adapt.
- The status output cannot be parsed into a connection state at all.
- An operator needs the encoder to receive the song title. That reverses a
  decision in ADR 0059 and needs an amendment first.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `src/broadcast/control.rs`, `src/view_models/broadcast.rs`,
  `src/ui/shells/broadcast.rs`

Goal:
- Add the `Stream` section for the `butt` encoder, with status, connect, and
  disconnect.

Constraints:
- Use the `butt` control options: `-S`, `-s`, `-d`, `-r`, `-t`, and `-a`/`-p`
  for a networked instance. Never `systemctl` and never `ssh` for the encoder.
- One code path for local and remote. The address options are the only
  difference.
- Never pass `-u`. The app does not send the song title.
- A missing binary is an empty state. Unparsable output is `Unknown`, never a
  panic.

Do not touch:
- the producer, the registry, API, database, the drop-file contract

Acceptance criteria:
- Four states parse from recorded output, connect and disconnect work.
- Guard blocks the encoder binary outside `src/broadcast/encoder.rs` and blocks
  `-u`.
- Screenshots for connected, disconnected, and not installed.

Test commands:
- `cargo fmt -- --check`
- `cargo test broadcast::encoder --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. encoder version tested against
5. screenshots captured
6. deviations from task
7. unresolved concerns
