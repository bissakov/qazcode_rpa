use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub fn interruptible_sleep(ms: u64, stop: &AtomicBool) -> Result<(), ()> {
    if stop.load(Ordering::Relaxed) {
        return Err(());
    }

    thread::park_timeout(Duration::from_millis(ms));

    if stop.load(Ordering::Relaxed) {
        Err(())
    } else {
        Ok(())
    }
}
