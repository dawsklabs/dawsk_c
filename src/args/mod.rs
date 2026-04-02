use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub mod step {
    pub const TOKEN: u8 = 1;
    pub const AST: u8 = 2;
    pub const HIR: u8 = 3;
}

pub fn step_enabled(step: u8) -> bool {
    STEP_OUTPUT.load(Ordering::Relaxed) & step != 0
}

pub mod mode {
    pub const BUILD: u8 = 1;
    pub const CHECK: u8 = 2;
    pub const RUN: u8 = 3;
}

#[macro_export]
macro_rules! debug {
    // debug!("msg")
    ($($arg:tt)*) => {
        if $crate::args::DEBUG_ENABLE.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!("[DEBUG] {}", format_args!($($arg)*));
        }
    };
}

pub static DEBUG_ENABLE: AtomicBool = AtomicBool::new(false);

pub static VERBOSE_ENABLE: AtomicBool = AtomicBool::new(false);

pub static TRACE_ENABLE: AtomicBool = AtomicBool::new(false);

pub static COLOR_ENABLE: AtomicBool = AtomicBool::new(true);

pub static UNICODE_ENABLE: AtomicBool = AtomicBool::new(false);

pub static STEP_OUTPUT: AtomicU8 = AtomicU8::new(0);

pub static COMPILE_MODE: AtomicU8 = AtomicU8::new(0);

pub struct ArgumentParser;

impl ArgumentParser {
    pub fn parse() {
        let args = std::env::args().skip(1);

        for arg in args {
            match arg.as_str() {
                // short flags: -d, -dt, -dvt
                s if s.starts_with('-') && !s.starts_with("--") => {
                    for c in s[1..].chars() {
                        match c {
                            'd' => DEBUG_ENABLE.store(true, Ordering::Relaxed),
                            'v' => VERBOSE_ENABLE.store(true, Ordering::Relaxed),
                            't' => TRACE_ENABLE.store(true, Ordering::Relaxed),
                            'u' => UNICODE_ENABLE.store(true, Ordering::Relaxed),
                            other => eprintln!("unknown short flag: -{}", other),
                        }
                    }
                }

                s if s.starts_with("--") => {
                    let flag = &s[2..];
                    if let Some(steps) = flag.strip_prefix("step=") {
                        for value in steps.split(',') {
                            match value {
                                "token" => {
                                    STEP_OUTPUT.fetch_or(step::TOKEN, Ordering::Relaxed);
                                }
                                "ast" => {
                                    STEP_OUTPUT.fetch_or(step::AST, Ordering::Relaxed);
                                }
                                "hir" => {
                                    STEP_OUTPUT.fetch_or(step::HIR, Ordering::Relaxed);
                                }
                                other => eprintln!("unknown step: {}", other),
                            }
                        }
                    } else {
                        match flag {
                            "no-color" => COLOR_ENABLE.store(false, Ordering::Relaxed),
                            "unicode" => UNICODE_ENABLE.store(true, Ordering::Relaxed),
                            other => eprintln!("unknown long flag: --{}", other),
                        }
                    }
                }

                // subcommands
                "build" => COMPILE_MODE.store(mode::BUILD, Ordering::Relaxed),
                "check" => COMPILE_MODE.store(mode::CHECK, Ordering::Relaxed),
                "run" => COMPILE_MODE.store(mode::RUN, Ordering::Relaxed),

                other => eprintln!("unknown argument: {}", other),
            }
        }
    }
}
