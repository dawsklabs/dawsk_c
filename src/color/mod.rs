use std::fmt::{Display, Formatter, Result};

pub enum Color {
    Reset,
    Bold,
    Italic,
    Underlined,
    FgHex(String),
    BgHex(String),
    FgRGB(u8, u8, u8),
    BgRGB(u8, u8, u8),
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Color::Reset => write!(f, "\x1b[0m"),
            Color::Bold => write!(f, "\x1b[1m"),
            Color::Italic => write!(f, "\x1b[3m"),
            Color::Underlined => write!(f, "\x1b[4m"),
            Color::FgHex(hex) => {
                if hex.len() == 6 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        return write!(f, "\x1b[38;2;{};{};{}m", r, g, b);
                    }
                }
                // Fallback auf Reset, wenn ungültig
                write!(f, "\x1b[0m")
            },
            Color::BgHex(hex) => {
                if hex.len() == 6 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        return write!(f, "\x1b[48;2;{};{};{}m", r, g, b);
                    }
                }
                // Fallback auf Reset, wenn ungültig
                write!(f, "\x1b[0m")
            },
            Color::FgRGB(r, g, b) => write!(f, "\x1b[38;2;{};{};{}m", r, g, b),
            Color::BgRGB(r, g, b) => write!(f, "\x1b[48;2;{};{};{}m", r, g, b),
        }
    }
}