//! GPUI completion marshalling for the independent worker (ADR 0066).

use gpui::{App, Context, Window};
use tokio::sync::oneshot;

/// Queue app shutdown after the platform's window-close callback returns.
pub(crate) fn quit_after_window_close(cx: &App) {
    // GPUI 0.2.2 holds its X11 client borrow during on_should_close. App::quit
    // borrows that client again. App::defer is also too early: its effects flush
    // inside the callback's App update. A foreground task runs after dispatch.
    cx.spawn(async move |cx| {
        let _ = cx.update(|cx| cx.quit());
    })
    .detach();
}

pub(crate) fn present_startup<T: 'static, R: Send + 'static>(
    receiver: oneshot::Receiver<R>,
    window: &Window,
    cx: &mut Context<T>,
    apply: impl FnOnce(&mut T, Result<R, oneshot::error::RecvError>, &mut Window, &mut Context<T>)
        + 'static,
) {
    let window = window.window_handle();
    cx.spawn(async move |entity, cx| {
        let result = receiver.await;
        // A closed window/entity discards the result. The worker remains owned
        // by bootstrap until its current operation finishes and is joined.
        let _ = window.update(cx, |_, window, cx| {
            let _ = entity.update(cx, |this, cx| {
                apply(this, result, window, cx);
                cx.notify();
            });
        });
    })
    .detach();
}

/// Apply a prepared result once, only while its originating window generation lives.
pub(crate) fn mount_current<T>(
    vm: &mut crate::view_models::startup::StartupReportVm,
    generation: u64,
    mount: impl FnOnce() -> T,
) -> Option<T> {
    vm.mount(generation).then(mount)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WindowDisposition {
    Continue,
    ExitUnsuccessful,
}

/// An activation error on a surviving window is nonfatal. Creation failure or
/// a gone window has no second-window fallback.
pub(crate) fn window_disposition(created: bool, exists: bool) -> WindowDisposition {
    if created && exists {
        WindowDisposition::Continue
    } else {
        WindowDisposition::ExitUnsuccessful
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::startup::{StartupAction, StartupReportVm};

    #[test]
    fn adr_0066_only_the_current_preparation_mounts_a_normal_factory_once() {
        let mut vm = StartupReportVm::new(true);
        let generation = vm.begin(StartupAction::CheckAgain).unwrap();
        let mut factories = 0;
        assert!(mount_current(&mut vm, generation + 1, || factories += 1).is_none());
        assert!(mount_current(&mut vm, generation, || factories += 1).is_some());
        assert!(mount_current(&mut vm, generation, || factories += 1).is_none());
        assert_eq!(factories, 1);
        let mut vm = StartupReportVm::new(true);
        let generation = vm.begin(StartupAction::CheckAgain).unwrap();
        vm.close();
        assert!(mount_current(&mut vm, generation, || factories += 1).is_none());
        assert_eq!(factories, 1);
    }

    #[test]
    fn adr_0066_window_failure_and_closed_window_exit_but_activation_is_nonfatal() {
        assert_eq!(
            window_disposition(false, false),
            WindowDisposition::ExitUnsuccessful
        );
        assert_eq!(
            window_disposition(true, false),
            WindowDisposition::ExitUnsuccessful
        );
        assert_eq!(window_disposition(true, true), WindowDisposition::Continue);
    }
}
