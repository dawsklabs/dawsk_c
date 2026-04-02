use std::sync::atomic::Ordering;
use const_format::concatcp;

use crate::args::COLOR_ENABLE;

pub const MAUVE_COLOR:    &str = concatcp!("\x1b[38;2;", 203u8, ";", 166u8, ";", 247u8, "m");
pub const LAVENDAR_COLOR: &str = concatcp!("\x1b[38;2;", 170u8, ";", 155u8, ";", 235u8, "m");
pub const BLUE_COLOR:     &str = concatcp!("\x1b[38;2;", 127u8, ";", 176u8, ";", 255u8, "m");
pub const GREEN_COLOR:    &str = concatcp!("\x1b[38;2;", 166u8, ";", 227u8, ";", 161u8, "m");
pub const CYAN_COLOR:     &str = concatcp!("\x1b[38;2;", 134u8, ";", 230u8, ";", 201u8, "m");
pub const PEACH_COLOR:    &str = concatcp!("\x1b[38;2;", 250u8, ";", 179u8, ";", 135u8, "m");
pub const YELLOW_COLOR:   &str = concatcp!("\x1b[38;2;", 255u8, ";", 219u8, ";", 162u8, "m");
pub const FLAMINGO_COLOR: &str = concatcp!("\x1b[38;2;", 235u8, ";", 193u8, ";", 193u8, "m");
pub const MAROON_COLOR:   &str = concatcp!("\x1b[38;2;", 235u8, ";", 160u8, ";", 172u8, "m");
pub const RED_COLOR:      &str = concatcp!("\x1b[38;2;", 255u8, ";", 144u8, ";", 145u8, "m");
pub const SUBTEXT_COLOR:  &str = concatcp!("\x1b[38;2;", 166u8, ";", 173u8, ";", 200u8, "m");

pub const BOLD:           &str = "\x1b[1m";
pub const ITALIC:         &str = "\x1b[3m";
pub const UNDERLINED:     &str = "\x1b[4m";
pub const STRIKETHROUGH:  &str = "\x1b[9m";

pub const RESET:                &str = "\x1b[0m";
pub const RESET_FG:             &str = "\x1b[39m";
pub const RESET_BG:             &str = "\x1b[49m";

pub const RESET_BOLD:           &str = "\x1b[22m";
pub const RESET_ITALIC:         &str = "\x1b[23m";
pub const RESET_UNDERLINE:      &str = "\x1b[24m";
pub const RESET_STRIKETHROUGH:  &str = "\x1b[29m";

pub fn fg_rgb(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

pub fn bg_rgb(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[48;2;{r};{g};{b}m")
}

pub fn c(color: &'static str) -> &'static str {
    if COLOR_ENABLE.load(Ordering::Relaxed) {
        color
    } else {
        ""
    }
}
