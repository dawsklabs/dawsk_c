use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

pub static ABORT: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[inline]
pub fn abort() {
    ABORT.store(true, Ordering::SeqCst);
}

#[inline]
pub fn is_aborted() -> bool {
    ABORT.load(Ordering::SeqCst)
}