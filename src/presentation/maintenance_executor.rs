//! Bounded independent maintenance worker; no normal runtime or database (ADR 0066).

use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread::{self, JoinHandle};

use tokio::sync::oneshot;

type Job = Box<dyn FnOnce() + Send>;
enum Message {
    Run(Job),
    Stop,
    Retire,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    Busy,
    Closed,
}

#[derive(Clone)]
pub struct MaintenanceClient {
    sender: mpsc::SyncSender<Message>,
    busy: Arc<AtomicBool>,
    retired: Arc<Mutex<Vec<Job>>>,
}

pub struct MaintenanceWorker {
    pub client: MaintenanceClient,
    handle: JoinHandle<()>,
}

impl MaintenanceWorker {
    pub fn start() -> io::Result<Self> {
        Self::start_with(|work| {
            thread::Builder::new()
                .name("v4vmm-maintenance".into())
                .spawn(work)
        })
    }

    fn start_with(spawn: impl FnOnce(Job) -> io::Result<JoinHandle<()>>) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let busy = Arc::new(AtomicBool::new(false));
        let retired: Arc<Mutex<Vec<Job>>> = Arc::default();
        let releases = retired.clone();
        let handle = spawn(Box::new(move || {
            while let Ok(message) = receiver.recv() {
                let stopping = matches!(message, Message::Stop);
                if let Message::Run(job) = message {
                    job();
                }
                let pending =
                    std::mem::take(&mut *releases.lock().expect("retired session resources"));
                for release in pending {
                    release();
                }
                if stopping {
                    break;
                }
            }
        }))?;
        Ok(Self {
            client: MaintenanceClient {
                sender,
                busy,
                retired,
            },
            handle,
        })
    }

    /// Called after the desktop loop exits: wait for outstanding I/O, then join.
    pub fn finish(self) {
        let _ = self.client.sender.send(Message::Stop);
        if let Err(panic) = self.handle.join() {
            std::panic::resume_unwind(panic);
        }
    }
}

impl MaintenanceClient {
    /// Retire detached session/preparation resources on the existing worker, including window close.
    pub(crate) fn retire<T: Send + 'static>(&self, resources: T) {
        self.retired
            .lock()
            .expect("retired session resources")
            .push(Box::new(move || drop(resources)));
        // A queued/current job already wakes the worker and drains this queue.
        let _ = self.sender.try_send(Message::Retire);
    }

    pub fn submit<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<oneshot::Receiver<T>, AdmissionError> {
        if self.busy.swap(true, Ordering::AcqRel) {
            return Err(AdmissionError::Busy);
        }
        let busy = self.busy.clone();
        let (sender, receiver) = oneshot::channel();
        let job = Box::new(move || {
            let result = work();
            busy.store(false, Ordering::Release);
            let _ = sender.send(result);
        });
        if let Err(error) = self.sender.try_send(Message::Run(job)) {
            self.busy.store(false, Ordering::Release);
            return Err(match error {
                mpsc::TrySendError::Full(_) => AdmissionError::Busy,
                mpsc::TrySendError::Disconnected(_) => AdmissionError::Closed,
            });
        }
        Ok(receiver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_detached_resources_are_released_on_the_independent_worker() {
        struct Resource {
            origin: std::thread::ThreadId,
            released: Arc<AtomicBool>,
        }
        impl Drop for Resource {
            fn drop(&mut self) {
                assert_ne!(self.origin, std::thread::current().id());
                self.released.store(true, Ordering::Release);
            }
        }
        let worker = MaintenanceWorker::start().unwrap();
        let released = Arc::new(AtomicBool::new(false));
        worker.client.retire(Resource {
            origin: std::thread::current().id(),
            released: released.clone(),
        });
        worker.finish();
        assert!(released.load(Ordering::Acquire));
    }

    #[test]
    fn adr_0066_pending_resource_release_keeps_worker_retryable() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let client = MaintenanceClient {
            sender,
            busy: Arc::new(AtomicBool::new(false)),
            retired: Arc::default(),
        };
        client.retire(());
        assert_eq!(client.submit(|| ()).err(), Some(AdmissionError::Busy));
        assert!(matches!(receiver.recv().unwrap(), Message::Retire));
        let result = client.submit(|| 7).unwrap();
        let Message::Run(job) = receiver.recv().unwrap() else {
            panic!("Expected the retried operation");
        };
        job();
        assert_eq!(result.blocking_recv().unwrap(), 7);
    }

    #[test]
    fn adr_0066_worker_start_failure_is_fallible_without_a_runtime() {
        assert!(
            MaintenanceWorker::start_with(|_| Err(io::Error::other("injected spawn failure")))
                .is_err()
        );
    }

    #[test]
    fn adr_0066_worker_rejects_duplicates_and_finishes_after_receiver_closes() {
        let worker = MaintenanceWorker::start().unwrap();
        let (release, wait) = mpsc::channel();
        let (started, running) = mpsc::channel();
        let completed = Arc::new(AtomicBool::new(false));
        let done = completed.clone();
        let receiver = worker
            .client
            .submit(move || {
                started.send(()).unwrap();
                wait.recv().unwrap();
                done.store(true, Ordering::Release);
            })
            .unwrap();
        running.recv().unwrap();
        assert_eq!(
            worker.client.submit(|| ()).err(),
            Some(AdmissionError::Busy)
        );
        drop(receiver);
        assert!(!completed.load(Ordering::Acquire));
        release.send(()).unwrap();
        let client = worker.client.clone();
        worker.finish();
        assert!(completed.load(Ordering::Acquire));
        assert_eq!(client.submit(|| ()).err(), Some(AdmissionError::Closed));
    }
}
