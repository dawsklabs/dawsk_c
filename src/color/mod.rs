use std::fmt::{Display, Formatter, Result};

pub const MAUVE_COLOR: Color = Color::FgHex("#cba6f7");
pub const LAVENDAR_COLOR: Color = Color::FgHex("#aa9beb");
pub const BLUE_COLOR: Color = Color::FgHex("#7FB0FF");
pub const GREEN_COLOR: Color = Color::FgHex("#a6e3a1");
pub const CYAN_COLOR: Color = Color::FgHex("#86e6c9");
pub const PEACH_COLOR: Color = Color::FgHex("#fab387");
pub const YELLOW_COLOR: Color = Color::FgHex("#ffdba2");
pub const FLAMINGO_COLOR: Color = Color::FgHex("#ebc1c1");
pub const MAROON_COLOR: Color = Color::FgHex("#eba0ac");
pub const RED_COLOR: Color = Color::FgHex("#ff9091");
pub const SUBTEXT_COLOR: Color = Color::FgHex("#a6adc8");

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Color {
    ResetFg,
    ResetBg,
    ResetBold,
    ResetItalic,
    ResetUnderline,
    ResetStrikethrough,
    ResetAll,
    Bold,
    Italic,
    Underlined,
    Strikethrough,
    FgHex(&'static str),
    BgHex(&'static str),
    FgRGB(u8, u8, u8),
    BgRGB(u8, u8, u8),
}

impl Display for Color {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Color::ResetFg => write!(f, "\x1b[39m"),
            Color::ResetBg => write!(f, "\x1b[49m"),
            Color::ResetBold => write!(f, "\x1b[22m"),
            Color::ResetItalic => write!(f, "\x1b[23m"),
            Color::ResetUnderline => write!(f, "\x1b[24m"),
            Color::ResetStrikethrough => write!(f, "\x1b[29m"),
            Color::ResetAll => write!(f, "\x1b[0m"),
            Color::Bold => write!(f, "\x1b[1m"),
            Color::Italic => write!(f, "\x1b[3m"),
            Color::Underlined => write!(f, "\x1b[4m"),
            Color::Strikethrough => write!(f, "\x1b[9m"),
            Color::FgHex(hex) | Color::BgHex(hex) => {
                // check sequence
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
                // Fallback
                write!(f, "\x1b[0m")
            }
            Color::FgRGB(r, g, b) => write!(f, "\x1b[38;2;{};{};{}m", r, g, b),
            Color::BgRGB(r, g, b) => write!(f, "\x1b[48;2;{};{};{}m", r, g, b),
        }
    }
}
