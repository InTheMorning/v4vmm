# ADR 0046 Task 015: Forward Navigation

Status: Ready - 2026-10-02. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

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

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V3.
It states the needed state, what counts as wrong, and the cleanup. The check only reads pages.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

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
