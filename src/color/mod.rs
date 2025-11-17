use std::fmt::{Display, Formatter, Result};

pub enum Color {
    Reset,
    FgHex(String),
    BgHex(String),
    FgRGB(u8, u8, u8),
    BgRGB(u8, u8, u8),
    FgHSL(u8, u8, u8),
    BgHSL(u8, u8, u8),
}

impl Color {
    fn hsl_to_rgb(h: u8, s: u8, l: u8) -> (u8, u8, u8) {
        let h = h.min(100) as f32 / 360.0;
        let s = s.min(100) as f32 / 100.0;
        let l = l.min(100) as f32 / 100.0;

        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;

        fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
            let mut t = t;
            if t < 0.0 { t += 1.0; }
            if t > 1.0 { t -= 1.0; }
            if t < 1.0/6.0 { return p + (q - p) * 6.0 * t; }
            if t < 1.0/2.0 { return q; }
            if t < 2.0/3.0 { return p + (q - p) * (2.0/3.0 - t) * 6.0; }
            p
        }

        let r = hue_to_rgb(p, q, h + 1.0/3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0/3.0);

        ((r*255.0).round() as u8, (g*255.0).round() as u8, (b*255.0).round() as u8)
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Color::Reset => write!(f, "\x1b[0m"),
            Color::FgHex(hex) => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
                write!(f, "\x1b[38;2;{};{};{}m", r, g, b)
            }
            Color::BgHex(hex) => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
                write!(f, "\x1b[48;2;{};{};{}m", r, g, b)
            }
            Color::FgRGB(r, g, b) => write!(f, "\x1b[38;2;{};{};{}m", r, g, b),
            Color::BgRGB(r, g, b) => write!(f, "\x1b[48;2;{};{};{}m", r, g, b),
            Color::FgHSL(h, s, l) => {
                let (r, g, b) = Self::hsl_to_rgb(*h, *s, *l);
                write!(f, "\x1b[38;2;{};{};{}m", r, g, b)
            }
            Color::BgHSL(h, s, l) => {
                let (r, g, b) = Self::hsl_to_rgb(*h, *s, *l);
                write!(f, "\x1b[48;2;{};{};{}m", r, g, b)
            }
        }
    }
}