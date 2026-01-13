use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

pub static ABORT: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[inline]
pub fn abort() {
    ABORT.store(true, Ordering::SeqCst);
}

#[inline]
pub fn is_aborted() -> bool {
    ABORT.load(Ordering::SeqCst)
}
