# ADR 0046 Task 015: Forward Navigation

Status: Implemented - 2026-10-02. Mechanical checks Green. Visual gate open and paused.

## Goal

The frame chrome shows a Forward control adjacent to Back. Forward returns to the page that Back left.
Back and Forward each have a keyboard shortcut.

## Authority

- [ADR 0046](../adr/0046-workspace-frame-architecture.md) Architectural Invariant 2: the frame chrome owns Back, Forward, close and history.
- [ADR 0081](../adr/0081-remove-the-staged-frame-model.md) Decision 3.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, button and action discipline, token discipline, and "Current-view state must update in place".

## Recorded Facts - 2026-10-02

- `FrameNavigationState` in `src/view_models/workspace/nav.rs` has `go_back`, `can_go_back`, `go_forward` and `can_go_forward`. A push clears the forward history. Unit tests in `src/view_models/workspace/tests.rs` cover `go_forward`.
- Dead code removal task 002 moved `go_forward` into a `#[cfg(test)]` block, because no production path called it.
- `FrameChromeDisplay` in `src/view_models/workspace/chrome.rs` already builds a `forward` button display, with its accessibility label and its availability from `!nav.can_go_forward()`.
- `FrameShellSlots::on_forward` in `src/ui/composites/frame_shell.rs` is in a `#[cfg(test)]` block since task 002. `on_back` is live.
- `src/ui/shells/workspace.rs` wires Back through `back_select_handler_for` and `on_content_list_back_select`. The app restores the page of the destination entry, for example through `restore_publisher_page_for_nav` and `restore_name_match_page_for_nav`.
- `src/app/menu.rs` binds the app menu keys through `APP_MENU_BINDING_SPECS`, with a separate key for macOS. Back has no key binding.

## Required Changes

1. Move `go_forward` and `FrameShellSlots::on_forward` back into production code.
2. Wire the Forward control in the frame chrome with the pattern of Back. A Forward press calls `go_forward` on the frame and restores the page of the destination entry, as Back does.
3. Render Forward adjacent to Back, from its typed display. It is unavailable when the forward history is empty. Use a named icon and named tokens. The screen decides no availability.
4. Add key bindings: Back is `cmd-[` on macOS and `alt-left` on other platforms. Forward is `cmd-]` on macOS and `alt-right` on other platforms. A key does nothing when its control is unavailable.
   The operator decided on 2026-10-02: the macOS pair binds with the negated `Input` context, so a text box keeps its own key.
5. A page that the app restores on Forward updates in place, with the stale-result checks that Back already uses.

## Mechanical Acceptance Criteria

Use the prefix `adr_0046_forward_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R15-01 | After a push and a Back, the chrome display exposes Forward as available. Before a Back, it is unavailable |
| R15-02 | A Forward press restores the entry that Back left, and the chrome then exposes Forward as unavailable |
| R15-03 | A push after a Back clears the forward history, and Forward becomes unavailable |
| R15-04 | The key bindings for each platform hold the four keys of Required Change 4 |
| R15-05 | `go_forward` and `on_forward` have a production caller. No `#[cfg(test)]` block holds them |
| R15-06 | A Forward to a publisher page or a name-match page restores that page with the present stale-result check |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the frame chrome shows Forward adjacent to Back, in the same style. Forward is unavailable until the operator goes Back.
- V2: Back and then Forward return to the same page, with its scroll position and content. The keys do the same.
- V3: normal and narrow widths, Light and Dark themes, and the larger type sizes show both controls with no clipped element.

## Exclusions

- No other frame command, no close action, and no menu.
- No change to the history model, except the move out of `#[cfg(test)]`.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/view_models/workspace/nav.rs`, `chrome.rs`, `tests.rs`.
- `src/ui/composites/frame_shell.rs`, `src/ui/shells/workspace.rs`.
- `src/app.rs`, `src/app/breadcrumb.rs`, `src/app/publisher_dispatch.rs`, `src/app/name_match_dispatch.rs`, `src/app/search_dispatch.rs`: the Back path and the page restore functions.
- `src/app/menu.rs`: the key binding specs and their tests.

## Checks

```bash
cargo test --lib adr_0046_forward_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-10-02

Required Changes 1, 2, 3 and 5 are done. Required Change 4 is blocked. The section "Required Change 4 Is Blocked" gives the cause.

### Back Path Traced

A click on the Back button calls `on_content_list_back_select` in `src/ui/shells/workspace.rs`. This calls
`handle_content_list_back_select` in `src/app.rs`. That method calls `workspace_layout.pop_nav`. It then
calls a new shared method, `restore_content_list_nav_entry`. This shared method calls each of these
functions: `sync_search_results_detail_with_nav`, `library.hydrate_detail_from_nav`,
`restore_publisher_page_for_nav`, `restore_name_match_page_for_nav`, `restore_index_feed_detail_for_nav`,
and `restore_index_track_detail_for_nav`. For a `Search` entry, it also calls
`start_index_search_for_query`.

Forward uses the same shared method. A click on the Forward button calls the new
`on_content_list_forward_select` slot, then the new `handle_content_list_forward_select` method, then
`workspace_layout.pop_nav_forward`, then the same `restore_content_list_nav_entry` method Back uses.

The breadcrumb path in `src/app/breadcrumb.rs` calls `pop_nav_until`, not one pop. It calls
`restore_publisher_page_for_nav` and `restore_name_match_page_for_nav` on its own. Forward does not
connect to the breadcrumb path. Forward only calls the one-step restore Back uses, and it shares that
one method with Back.

### Files Changed

- `src/view_models/workspace/nav.rs`: `go_forward` moves out of its `#[cfg(test)]` block, into the live
  `impl FrameNavigationState` block.
- `src/view_models/workspace/mod.rs`: `WorkspaceModelError::CannotNavigateForward` and its `Display` line
  drop their `#[cfg(test)]` mark. A new method, `pop_nav_forward`, sits with `pop_nav`.
- `src/ui/composites/frame_shell.rs`: `FrameShellSlots::on_forward` moves out of its `#[cfg(test)]`
  block, into the live `impl FrameShellSlots` block, with `on_back`.
- `src/ui/shells/workspace.rs`: `WorkspaceSlots` gets a new field and a new method for Forward, matching
  its existing field and method for Back. A new `forward_select_handler_for` lookup matches
  `back_select_handler_for`. `WorkspaceShell::render` calls `shell_slots.on_forward`, matching its
  existing call to `shell_slots.on_back`.
- `src/app.rs`: a new `handle_content_list_forward_select` method. A new shared method,
  `restore_content_list_nav_entry`, carries the body that Back and Forward each call. The content-list
  frame wires `on_content_list_forward_select`, matching its existing wire for
  `on_content_list_back_select`.
- `src/app/menu.rs`: two new actions, `NavigateBack` and `NavigateForward`, and a new type,
  `AppMenuKeystroke`. `Shared` keeps one keystroke for each first bind. The other shape carries a macOS
  keystroke and context, with a different keystroke and context for each other system.
- `src/app/menu.rs` also gets a new constant, `TEXT_BOX_EXCLUDED_CONTEXT`, holding the text `!Input`.
- Two new entries in `APP_MENU_BINDING_SPECS` carry the Back and Forward binds. Two new methods on
  `TopApp` send each key to its button method.
- `src/app.rs`: the render tree wires `handle_navigate_back` and `handle_navigate_forward`, adjacent to
  `handle_open_preferences`.
- `src/view_models/workspace/tests.rs`: three new tests, each with the name start `adr_0046_forward_`.
- `tests/architecture_tests.rs`: two new tests, each with the name start `adr_0046_forward_`. Two
  situational ADR 0067 guards, `adr_0067_keyboard_and_menu_share_platform_modifier_resolution` and
  `macos_app_menu_bootstrap_exposes_standard_app_commands`, are updated for the new `AppMenuKeystroke`
  shape. Each guard keeps its own protection: the shared platform adapter, and the standard macOS
  commands and their keys.
- `docs/tasks/adr-0046-task-015-forward-navigation.md`: this packet. The Status line, this section, and
  the Operator Visual Check section change.

No file named in "Do not touch" changes. No ADR changes. No database schema changes.

### Behavior Changed

- The Forward button in the content-list frame header is live. Before this change, the button showed on
  the screen with its icon, but a click on it did nothing.
- A Forward click moves the frame history by one step, to the page Back last left. This covers a track,
  an album, an artist, a playlist, and a search. It also covers an Index feed page, an Index track page,
  a publisher page, and a name-match page.
- A Forward click to a publisher page or a name-match page uses the same stale-result check Back uses.
  A stale network answer does not replace a newer page.
- The content-list frame responds to a key, and not only to a button click, for Back and for Forward.
- On Linux, `alt-left` and `alt-right` move frame history, the same as a click on Back or Forward.
- On macOS, `cmd-[` and `cmd-]` do the same, away from a text box. A text box keeps `cmd-[` and `cmd-]`
  for its own line-indent command.
- No other frame control changes.

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0046_forward_` | Green, 7 tests |
| `cargo test` | Green, 1778 unit tests, 288 guard tests, 10 doc tests marked skip |
| `cargo test --test architecture_tests` | Green, 288 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |
| `grep -rn "dead_code" src` | Green, no line found |

### Required Change 4

Required Change 4 asks for four binds: `cmd-[` and `cmd-]` on macOS, and `alt-left` and `alt-right` on
each other system.

The GPUI parts this app uses bind two of these four keys, in a text box, for a different command:

- On macOS, `cmd-[` moves a list line out, and `cmd-]` moves a list line in. Each bind sits in
  `gpui-base-0.6.1`, file `src/input/base/state.rs`, near line 185 and line 189.
- On macOS, `alt-left` and `alt-right` move the cursor one word back and one word forward. These binds
  sit in the same file, near line 276 and line 278.
- On each other system, the same file binds `ctrl-left` and `ctrl-right` to the same word move, not
  `alt-left` or `alt-right`. The app finds no bind at `alt-left` or `alt-right` on a system that is not
  macOS.

The operator decided on 2026-10-02: the macOS pair binds with the negated `Input` context, so a text box
keeps its own key. The pair for each other system carries no context, because this packet finds no clash
there.

`src/app/menu.rs` holds two new actions, `NavigateBack` and `NavigateForward`. A new type,
`AppMenuKeystroke`, carries one shared keystroke for each first bind. The other shape carries a macOS
keystroke and context, with a different keystroke and context for each other system. A new constant,
`TEXT_BOX_EXCLUDED_CONTEXT`, holds the text `!Input`. Two new entries in `APP_MENU_BINDING_SPECS` carry
the Back and Forward binds, each with this shape.

Two new methods on `TopApp`, `handle_navigate_back` and `handle_navigate_forward`, send each key to the
same method its button calls: `handle_content_list_back_select` and `handle_content_list_forward_select`.
A key does nothing when its control is not available. Those methods do nothing then.

Two situational ADR 0067 guards in `tests/architecture_tests.rs` checked the field name `keystroke` as a
fixed string. `adr_0067_keyboard_and_menu_share_platform_modifier_resolution` and
`macos_app_menu_bootstrap_exposes_standard_app_commands` are updated for the new `AppMenuKeystroke` shape.
Each guard keeps its own protection: the shared platform adapter for each `Shared` entry, and the
standard macOS commands with their own keys.

### Deviations From The Task

1. `handle_content_list_back_select` moves its body into a new shared method,
   `restore_content_list_nav_entry`, so Forward can use the same path. Back's behavior does not change.
   This matches the packet's own hard rule: "Forward must reuse the same restore path."

### Unresolved Concerns

1. This task's own scope names only `src/`, `tests/` and the packet document. So this change does not
   edit `docs/pending-human-checks.md` or `docs/plans/broadcast-chain-delivery-order.md`. A new change
   should record this packet's open visual gate there.
2. Clean-up of the proof directory `/tmp/v4vmm-governance.ie6k8TQf` stays open. This packet does not
   include this clean-up.
3. The macOS steps of the Operator Visual Check need a macOS desktop session. No such session is
   confirmed for the operator today.

## Operator Visual Check

This check reads the app only. It writes no tag and no file, and it needs no fixture. It needs a Linux
desktop session and this checkout. The macOS steps in V2 need a macOS desktop session. Skip them without
one, and write down that you skipped them.

Use the on-screen Back and Forward buttons, then the keys, for each step below.

1. Close v4vmm. Make the binary:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app:

   ```bash
   target/debug/v4vmm
   ```
3. Open Music. Select a track, an album, or an artist, so the content-list frame shows a second page.
   Write down the page you see.

**V1 - the frame header shows a Forward button adjacent to the Back button, and Forward starts unavailable**

4. Look at the content-list frame header.
   - Correct: the Forward button is adjacent to the Back button, with the same style and the same type
     of icon. Forward shows as not available. It does not react to a click.
   - Incorrect: no Forward button shows, or Forward shows as available before a Back click.
5. Click Back.
   - Correct: Forward shows as available.
   - Incorrect: Forward stays not available.

**V2 - a Forward click goes to the page Back left, and the keys match the buttons**

6. Click Forward.
   - Correct: the frame shows the correct page from step 3 again, with the same scroll position and the
     same content.
   - Incorrect: the frame shows a different page, an empty page, or a page that does not complete
     loading.
7. Do steps 3 through 6 again with a publisher page. Open one from a track or an album, if your library
   has one. Do them again with a name-match page from an Index name search, if one is open.
   - Correct: each page type gives the same result through Back and Forward.
   - Incorrect: a publisher page or a name-match page does not go back, or shows stale content.
8. On Linux, press `alt-left`.
   - Correct: this does the same as a click on Back, in step 5.
   - Incorrect: nothing happens, or a text box takes the key.
9. Press `alt-right`.
   - Correct: this does the same as a click on Forward, in step 6.
   - Incorrect: nothing happens, or a text box takes the key.
10. This step needs a macOS desktop session. Skip it on Linux, and write down that you skipped it.
    With a page open where Back is available, move focus away from a text box. Press `cmd-[`.
    - Correct: this does the same as a click on Back.
    - Incorrect: nothing happens.
11. Press `cmd-]` on macOS.
    - Correct: this does the same as a click on Forward.
    - Incorrect: nothing happens.
12. On macOS, click in a text box, for example a Settings text field. Press `cmd-[`, then `cmd-]`.
    - Correct: the text box moves a line out, then in. The content-list frame page does not change.
    - Incorrect: the content-list frame page changes, or the text box does not move the line.

**V3 - usual and narrow widths, Light and Dark, a larger scale**

13. With a page open where Forward is available, resize the window to its usual width, then to its
    narrowest width. Open Settings -> General (`Ctrl+Comma`). Select Light, then Dark. Select a larger
    scale. Go back to the content-list frame after each change.
    - Correct: Back and Forward stay in view at each width, each theme, and each scale, with no clipped
      icon. A color change between Light and Dark is expected. It is not a defect by itself.
    - Incorrect: a button is clipped, overlaps a different control, or disappears at a combination.
14. Close the app.

**Clean-up**

None. This check reads your library. It creates no fixture, and it writes no file.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0046-task-015-forward-navigation.md`
- ADR 0046 Invariant 2 and ADR 0081 Decision 3
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the Forward control, its wiring, its rendering and the two key bindings.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives the label, the availability and the icon. The screen only composes. Use the scaled tokens of ADR 0039 and the existing chrome composite.
- Background work uses the ADR 0040 runtime. Never call `cx.spawn` from a screen.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The frame add, remove, detach and dock model. ADR 0081 packet 001 deletes it.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R15-01 to R15-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V3.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A key binding conflicts with an existing binding or a text input key.
- The Back path does not restore a page that Forward must restore, and a shared restore function is needed.
- A change needs a file in "Do not touch".

## Orchestrator Review - 2026-10-02

The orchestrator reviewed the diff two times and ran each check. Each check is Green: 1,778 unit tests, 288 guards, and no warning.

- Forward and Back share one restore function, `restore_content_list_nav_entry`, so a Forward press restores a page as Back does.
- The first session stopped at Required Change 4 because of a real key conflict on macOS. The operator decided the keys on 2026-10-02. A key test uses real keymap dispatch and proves that `cmd-[` and `cmd-]` stay out of a text input.
- Two ADR 0067 guards matched the old binding field name. The implementer changed them and kept their ADR citations.
- The macOS key steps of the visual check need a macOS desktop session.
