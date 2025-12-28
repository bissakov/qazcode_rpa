use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub struct StopControl {
    flag: Arc<AtomicBool>,
    condvar: Arc<Condvar>,
}

impl StopControl {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            condvar: Arc::new(Condvar::new()),
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    pub fn request_stop(&self) {
        self.flag.store(true, Ordering::Relaxed);
        self.condvar.notify_all();
    }

    pub fn reset(&self) {
        self.flag.store(false, Ordering::Relaxed);
    }

    pub fn sleep_interruptible(&self, ms: u64) -> bool {
        if self.is_stopped() {
            return false;
        }

        let mutex = Mutex::new(());
        let guard = mutex.lock().unwrap();
        let result = self.condvar.wait_timeout(guard, Duration::from_millis(ms));

        !self.is_stopped()
    }
}

impl Clone for StopControl {
    fn clone(&self) -> Self {
        Self {
            flag: Arc::clone(&self.flag),
            condvar: Arc::clone(&self.condvar),
        }
    }
}

impl Default for StopControl {
    fn default() -> Self {
        Self::new()
    }
}
