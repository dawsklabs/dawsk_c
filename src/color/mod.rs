use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone)]
pub enum Color {
    Reset,
    Bold,
    Italic,
    Underlined,
    FgHex(&'static str),
    BgHex(&'static str),
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
            Color::FgHex(hex) | Color::BgHex(hex) => {
                // Prüfen: Länge = 7, erstes Zeichen #
                if hex.len() == 7 && hex.starts_with('#') {
                    // parse R, G, B
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[1..3], 16),
                        u8::from_str_radix(&hex[3..5], 16),
                        u8::from_str_radix(&hex[5..7], 16),
                    ) {
                        return match self {
                            Color::FgHex(_) => write!(f, "\x1b[38;2;{};{};{}m", r, g, b),
                            Color::BgHex(_) => write!(f, "\x1b[48;2;{};{};{}m", r, g, b),
                            _ => unreachable!(),
                        };
                    }
                }
                // Fallback auf Reset bei ungültigem Code
                write!(f, "\x1b[0m")
            }
            Color::FgRGB(r, g, b) => write!(f, "\x1b[38;2;{};{};{}m", r, g, b),
            Color::BgRGB(r, g, b) => write!(f, "\x1b[48;2;{};{};{}m", r, g, b),
        }
    }
}