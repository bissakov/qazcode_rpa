use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

pub fn interruptible_sleep(ms: u64, stop: &AtomicBool) -> bool {
    if stop.load(Ordering::Relaxed) {
        return false;
    }

    let start = std::time::Instant::now();
    let duration = Duration::from_millis(ms);
    let check_interval = Duration::from_millis(50);

    while start.elapsed() < duration {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        thread::sleep(check_interval);
    }

    !stop.load(Ordering::Relaxed)
}
