// z.B. in lib.rs oder debug.rs
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    OnceLock,
};

pub mod step {
    pub const TOKENS: u8 = 1;
    pub const AST: u8 = 2;
    pub const HIR: u8 = 3;
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
        use std::sync::atomic::Ordering;
        if $crate::args::DEBUG_ENABLE.load(Ordering::Relaxed) {
            eprintln!("[DEBUG] {}", format_args!($($arg)*));
        }
    };
}

pub static INPUT_FILE: OnceLock<String> = OnceLock::new();

pub static DEBUG_ENABLE: AtomicBool = AtomicBool::new(false);

pub static VERBOSE_ENABLE: AtomicBool = AtomicBool::new(false);

pub static TRACE_ENABLE: AtomicBool = AtomicBool::new(false);

pub static COLOR_ENABLE: AtomicBool = AtomicBool::new(false);

pub static STEP_OUTPUT: AtomicU8 = AtomicU8::new(0);

pub static COMPILE_MODE: AtomicU8 = AtomicU8::new(0);

pub struct ArgumentParser;

impl ArgumentParser {
    pub fn parse() {
        let mut args = std::env::args().skip(1);

        if let Some(file) = args.next() {
            INPUT_FILE.set(file).ok();
        }

        while let Some(arg) = args.next() {
            match arg.as_str() {
                // short flags: -d, -dt, -dvt
                s if s.starts_with('-') && !s.starts_with("--") => {
                    for c in s[1..].chars() {
                        match c {
                            'd' => DEBUG_ENABLE.store(true, Ordering::Relaxed),
                            'v' => VERBOSE_ENABLE.store(true, Ordering::Relaxed),
                            't' => TRACE_ENABLE.store(true, Ordering::Relaxed),
                            other => eprintln!("unknown short flag: -{}", other),
                        }
                    }
                }

                // lange flags: --no-color
                s if s.starts_with("--") => match &s[2..] {
                    "no-color" => COLOR_ENABLE.store(false, Ordering::Relaxed),
                    "color" => COLOR_ENABLE.store(true, Ordering::Relaxed),
                    other => eprintln!("unknown long flag: --{}", other),
                },

                // step=tokens,ast,hir
                s if s.starts_with("step=") => {
                    for value in s["step=".len()..].split(',') {
                        match value {
                            "tokens" => {
                                let _ = STEP_OUTPUT.fetch_or(step::TOKENS, Ordering::Relaxed);
                            }
                            "ast" => {
                                let _ = STEP_OUTPUT.fetch_or(step::AST, Ordering::Relaxed);
                            }
                            "hir" => {
                                let _ = STEP_OUTPUT.fetch_or(step::HIR, Ordering::Relaxed);
                            }
                            other => eprintln!("unknown step: {}", other),
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
