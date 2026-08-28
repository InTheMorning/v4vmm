# ADR 0040: Async View-Model Runtime

## Status

Implemented - 2026-05-18. Runtime foundation and legacy scheduling
retirement complete, including the screen-local `cx.spawn` retirement.

## Context

ADR 0024 introduced a typed `ApplicationCommand` / `CommandOutcome` /
`ApplicationEvent` boundary plus a synchronous `CommandBus` and a
`GpuiCommandRunner` that does
`background_executor().spawn(blocking) → entity.update(...) → cx.notify`
on every dispatch. The interface is correct; the *scheduler* underneath
is wrong for what GPUI actually offers.

Verified against the GPUI source tree at
`crates/gpui/src/{app.rs,executor.rs,app/context.rs}`:

- `Context::spawn` (`app/context.rs:237`) delegates to `App::spawn`
  (`app.rs:1667`) which always runs on `foreground_executor`. There is
  no path through `Context::spawn` that lands on background.
- `ForegroundExecutor::spawn` is documented (`executor.rs:424`) as
  *"Enqueues the given Task to run on the main thread."* Its
  `spawn_with_priority` comment (`executor.rs:445`) says *"Priority is
  ignored for foreground tasks - they run in order on the main thread."*
- `cx.notify`, observers, subscribers, and even `refresh_windows` push
  into `App::pending_effects: VecDeque<Effect>` (`app.rs:599, 883`).
  This queue is drained by `flush_effects` (`app.rs:1394`) which runs
  inline at the end of every `App::update` (`app.rs:834, 1413`). The
  loop *also* processes effects produced by effects, in the same drain.

In other words, in GPUI the foreground executor *is* the render thread
*is* the effect-drain thread, and everything is a single serial queue.
This is the same shape as a game engine's main loop. Any architecture
that fans out work via `cx.spawn` or via inline observers will, by
construction, compete with rendering for the same serial budget.

The `GpuiCommandRunner` also offers no:

- backpressure for many concurrent commands (one tokio task per call,
  no shared workers, no semaphore);
- real cancellation (the `CancellationToken` field on
  `CommandContext` is not polled inside the synchronous bus);
- coalescing (every result becomes its own `entity.update`);
- screen-decoupled completion (the closure form binds the dispatcher to
  whoever started the work, so navigation away leaves work orphaned).

The presentation boundary needs a runtime, not just a typed surface.

## Decision

Introduce an **async view-model runtime** owned by
`ApplicationServices`, with three rules:

1. **A single tokio multi-thread runtime owned by the process.**
   Every long-lived concern (downloads, tagging, MusicBrainz lookup,
   now-playing, paged list caches) runs as a tokio task with an
   `mpsc::Sender<Cmd>` inbox. Tasks are called *actors*.
2. **Snapshots flow through `tokio::sync::watch`.** Each actor owns
   its view-model state and publishes immutable snapshots via a
   `watch::Sender<Snapshot>`. Screens hold the matching `Receiver`
   and read `borrow()` once per frame. Old snapshots drop
   automatically — natural per-frame coalescing.
3. **Exactly one bridge crosses the GPUI boundary.**
   `presentation::GpuiVmBridge` runs as a single foreground task that
   wakes on a heartbeat (target ~60 Hz), drains pending `VmEvent`s
   from a `mpsc::Receiver<VmEvent>`, groups them by target entity,
   and applies **one** `entity.update` per affected screen per frame.
   Screens never spawn work; they dispatch into actor inboxes.

Vocabulary:

| Term | Meaning |
|---|---|
| `Actor` | A `tokio::task` owning some VM state, with `mpsc<Cmd>` inbox |
| `ActorHandle<Cmd>` | `Clone`able sender into the actor's inbox |
| `Snapshot<T>` | `watch::Sender<T>` wrapper that publishes immutable VM state |
| `VmBus` | `mpsc<VmEvent>` carrying screen-coalescable notifications |
| `VmEvent` | `{ target: ScreenTag, payload: Box<dyn Any + Send> }` |
| `GpuiVmBridge` | The one foreground task that drains the bus per frame |
| `AsyncCommandRunner` | `dispatch(cmd, ctx)` fire-and-forget |

`ApplicationCommand` / `CommandOutcome` / `ApplicationEvent` remain
exactly as defined in ADR 0024. The runtime *wraps* them, it does not
replace them. Commands run on the runtime's threadpool (not the
foreground executor); their `CommandOutcome.events` are published to
the `ApplicationEventBus`; subscribers translate those events into
`VmEvent`s on the bus.

`CommandContext::cancellation_token` is honoured by adopting
`tokio_util::sync::CancellationToken`. Long-running commands poll it
inside `tokio::select!` and abort cleanly. Operator-cancel actually
cancels.

### Layer rules added by this ADR

```
GPUI thread (frame loop)            ← render only, never holds VM state
        ▲
        │ per-frame drain (1 entity.update per affected screen)
GpuiVmBridge                        ← coalescer, presentation/ only
        ▲
        │ tokio mpsc / watch::Receiver
VM runtime (tokio multi-thread)     ← actors own VM state
        ▲
        │ blocking calls
domain (db / *_service / api)       ← unchanged
```

- `src/runtime/` MAY import `tokio` / `tokio-util`. It MUST NOT import
  `gpui`, `gpui_component`, screen modules, or `application/` types
  beyond `ApplicationCommand` and `ApplicationEvent`.
- `src/presentation/` is the only module that may import both `tokio`
  and `gpui`.
- Screens MUST NOT call `cx.spawn` to do domain work. They dispatch
  into an actor and read snapshots through `watch::Receiver`.
- The synchronous `CommandBus` continues to exist for CLI and tests
  where the runtime is not desired.

## Consequences

Positive:

- Foreground executor is reserved for rendering and one bridge task.
  Frame budget protected by construction.
- N events per frame become 1 `entity.update` per screen per frame.
  No more O(N) effect drains during a render.
- Worker pools (e.g. `Semaphore::new(4)` for downloads) provide
  real backpressure.
- Cancellation works.
- Screens that navigate away do not orphan or block in-flight work.
- Sets up the foundation for ADR 0041 (windowed paged VMs), which
  *requires* per-actor state ownership.

Negative:

- Adds unconditional `tokio` (`rt-multi-thread`, `sync`, `time`,
  `macros`) and `tokio-util` (`sync`) deps for the desktop build. The
  previous feature-gated compatibility path is intentionally retired.
- Two indirection layers added: `Actor → Snapshot → Bridge → Screen`.
  Acceptable cost for the determinism it buys.
- Every screen with paged or long-lived state grows an actor module.
  Mitigated by the `PagedListVm<Id, Row>` generic in ADR 0041.

## Implementation notes

- Heartbeat source: `cx.spawn` a single foreground task that loops
  on `cx.background_executor().timer(Duration::from_millis(16)).await`
  then calls into the bridge to drain. Verify this does not pin the
  foreground task; fall back to a `refresh_windows` observer if so.
- The bridge holds `WeakEntity<T>` per screen tag and skips delivery
  if the entity is dropped.
- `AsyncCommandRunner::dispatch(cmd, ctx)` returns immediately. There
  is no per-call `on_success` / `on_error` — completions surface as
  `ApplicationEvent`s on the bus, which actors translate into `VmEvent`s
  for affected screens.
- `runtime::actor::spawn(name, behaviour)` is the canonical actor
  constructor; it logs panics, supports graceful shutdown, and exposes
  an `ActorHandle<Cmd>`.
- Tests use `tokio::runtime::Builder::new_current_thread()` and a
  pacing fake-clock so coalescing windows are deterministic.
