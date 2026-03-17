use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

pub static ABORT: OnceLock<AtomicBool> = OnceLock::new();

pub fn abort() {
    // Initialisieren, falls noch nicht passiert
    let flag = ABORT.get_or_init(|| AtomicBool::new(false));
    flag.store(true, Ordering::SeqCst);
}

pub fn is_aborted() -> bool {
    let flag = ABORT.get_or_init(|| AtomicBool::new(false));
    flag.load(Ordering::SeqCst)
}
