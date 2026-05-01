# ADR 0024 Review Checklist

## Reviewed Artifact

Use this checklist for each ADR 0024 implementation diff and for the Phase 2
checkpoint.

## Pass / Fail

- Status: Phase 2 checkpoint passed with Task 004 adjustment.
- Reviewer: Codex
- Date: 2026-04-30
- Review: `docs/reviews/adr-0024-phase-2-checkpoint.md`

## Architectural Invariants

- [ ] `src/application/**` has no `gpui` or `gpui_component` imports.
- [ ] `ApplicationServices` is explicit root wiring, not a service locator or
      string registry.
- [ ] `CommandBus::execute(command, context)` remains synchronous and GPUI-free.
- [ ] Long-running GPUI-triggered commands run through `GpuiCommandRunner`.
- [ ] `CommandContext` carries operation id, cancellation, and trace data needed
      by the slice.
- [ ] `CommandError` is the shared error channel with family variants.
- [ ] Command failures are returned through `Result`; failure is not represented
      by application events.
- [ ] `ApplicationEventBus` broadcasts to app-level subscribers and is not a
      per-screen local dispatcher.
- [ ] `PresentationEventBridge` / `GpuiEventBridge` drains app events on the UI
      thread before notifying.
- [ ] Subscribers do not infer command success from event absence.
- [ ] `ApplicationQueryService` reads local app state only.
- [ ] Remote-only discovery/search is not hidden in local queries or overloaded
      onto commands.
- [ ] Ports are introduced only where the phase needs replaceability.
- [ ] `DownloadManager` is used for migrated download/subscription commands.
- [ ] Command handlers and ports crossing background execution are `Send + Sync`
      where required.

## Slice-Specific Checks

- [ ] Playlist slice preserves ordering, deduplication, and CLI compatibility
      decisions.
- [ ] Subscription/download slice preserves audio download, tag-write, and
      library-membership behavior.
- [ ] Metadata/feed update slice preserves source facts and does not add hidden
      inference.
- [ ] Playback slice uses `PlayTrack`, `PausePlayback`, `ResumePlayback`,
      `StopPlayback`, and `SeekPlayback`; it does not use `StartPlayback` for
      track playback. `SetPlaybackVolume` remains deferred until the playback
      driver boundary has an approved volume operation.
- [ ] Presentation cleanup happens only after workflow dispatch moved out of the
      affected screen code.

## Tests And Verification

- [ ] Focused command tests cover success and failure.
- [ ] Query tests pin local snapshot behavior.
- [ ] Event tests verify emitted event families and broadcast behavior.
- [ ] `tests/architecture_tests.rs` prevents boundary regressions for migrated
      paths.
- [ ] `cargo fmt -- --check` passed.
- [ ] `cargo check` passed.
- [ ] Relevant focused `cargo test ...` commands passed.
- [ ] `cargo clippy --lib --tests -- -D warnings` passed before phase merge.

## Required Fixes

- None recorded.

## Optional Improvements

- None recorded.

## Merge Recommendation

- Not ready until reviewed.

## Phase 2 Checkpoint Questions

- Did the playlist slice validate `CommandContext`?
- Did app-scoped `ApplicationEventBus` update every affected view?
- Did `ApplicationServices` stay explicit and boring?
- Did `ApplicationQueryService` avoid remote network behavior?
- Were architecture tests useful without excessive false positives?
- Should ADR 0024 or remaining task packets be revised before Phase 3?
