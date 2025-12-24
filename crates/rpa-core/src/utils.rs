use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub fn interruptible_sleep(ms: u64, stop: &AtomicBool) -> bool {
    if stop.load(Ordering::Relaxed) {
        return false;
    }

    thread::park_timeout(Duration::from_millis(ms));

    !stop.load(Ordering::Relaxed)
}
