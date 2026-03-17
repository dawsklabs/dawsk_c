use std::cmp::Ordering;
use std::fmt::{Formatter, Result};
use std::io::Write;

use strip_ansi_escapes::Writer;

use super::{AnsiMode, *};

#[allow(dead_code)]
pub struct Characters {
    pub hbar: char,
    pub vbar: char,
    pub xbar: char,
    pub vbar_gap: char,
    pub line_margin: char,

    pub uarrow: char,
    pub rarrow: char,

    pub ltop: char,
    pub mtop: char,
    pub rtop: char,
    pub lbot: char,
    pub rbot: char,
    pub mbot: char,

    pub lbox: char,
    pub rbox: char,

    pub lcross: char,
    pub rcross: char,

    pub lunderbar: char,
    pub runderbar: char,
    pub munderbar: char,
    pub underline: char,
    pub underbar_single: char,
}

impl Characters {
    pub fn unicode() -> Self {
        Self {
            hbar: '─',
            vbar: '│',
            xbar: '┼',
            vbar_gap: '┆',
            line_margin: '┤',
            uarrow: '▲',
            // TODO: Revert when more commonly supported
            // uarrow: '🭯',
            rarrow: '▶',
            ltop: '╭',
            mtop: '┬',
            rtop: '╮',
            lbot: '╰',
            mbot: '┴',
            rbot: '╯',
            lbox: '┤',
            rbox: '│',
            lcross: '├',
            rcross: '┤',
            lunderbar: '┌',
            runderbar: '┐',
            munderbar: '┬',
            underline: '─',
            underbar_single: '▲',
            // TODO: Revert when more commonly supported
            // underbar_single: '🭯',
        }
    }

    pub fn ascii() -> Self {
        Self {
            hbar: '-',
            vbar: '|',
            xbar: '+',
            vbar_gap: ':',
            line_margin: '|',
            uarrow: '^',
            rarrow: '>',
            ltop: ',',
            mtop: 'v',
            rtop: '.',
            lbot: '`',
            mbot: '-',
            rbot: '\'',
            lbox: '[',
            rbox: ']',
            lcross: '|',
            rcross: '|',
            lunderbar: '-',
            runderbar: '-',
            munderbar: '-',
            underline: '-',
            underbar_single: '^',
        }
    }
}

/// Output stream to check for whether color is enabled.
#[derive(Clone, Copy, Debug)]
pub enum StreamType {
    /// Standard Output
    Stdout,
    /// Standard Error
    Stderr,
}

/// A trait used to add formatting attributes to displayable items intended to be written to a
/// particular stream (`stdout` or `stderr`).
///
/// Attributes specified through this trait are not composable (i.e: the behaviour of two nested attributes each with a
/// conflicting attribute is left unspecified).
pub trait StreamAwareFmt: Sized {
    /// Returns true if color is enabled for the given stream.
    fn color_enabled_for(_: StreamType) -> bool {
        COLOR_ENABLE.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Give this value the specified foreground colour, when color is enabled for the specified stream.
    fn fg(self, color: Option<&'static str>, stream: StreamType) -> Foreground<Self> {
        if Self::color_enabled_for(stream) {
            Foreground(self, color)
        } else {
            Foreground(self, None)
        }
    }

    /// Give this value the specified background colour, when color is enabled for the specified stream.
    fn bg(self, color: Option<&'static str>, stream: StreamType) -> Background<Self> {
        if Self::color_enabled_for(stream) {
            Background(self, color)
        } else {
            Background(self, None)
        }
    }
}

impl<T: Display> StreamAwareFmt for T {}

/// A trait used to add formatting attributes to displayable items.
///
/// Attributes specified through this trait are not composable (i.e: the behaviour of two nested attributes each with a
/// conflicting attribute is left unspecified).
pub trait Fmt: Sized {
    /// Give this value the specified foreground colour.
    fn fg(self, color: Option<&'static str>) -> Foreground<Self>
    where
        Self: fmt::Display,
    {
        StreamAwareFmt::fg(self, color, StreamType::Stderr)
    }

    /// Give this value the specified background colour.
    fn bg(self, color: Option<&'static str>) -> Background<Self>
    where
        Self: Display,
    {
        StreamAwareFmt::bg(self, color, StreamType::Stdout)
    }
}

impl<T: Display> Fmt for T {}

/// A trait used to add formatting attributes to displayable items intended to be written to `stdout`.
///
/// Attributes specified through this trait are not composable (i.e: the behaviour of two nested attributes each with a
/// conflicting attribute is left unspecified).
pub trait StdoutFmt: StreamAwareFmt {
    /// Give this value the specified foreground colour, when color is enabled for `stdout`.
    fn fg(self, color: Option<&'static str>) -> Foreground<Self> {
        StreamAwareFmt::fg(self, color, StreamType::Stdout)
    }

    /// Give this value the specified background colour, when color is enabled for `stdout`.
    fn bg(self, color: Option<&'static str>) -> Background<Self> {
        StreamAwareFmt::bg(self, color, StreamType::Stdout)
    }
}

impl<T: fmt::Display> StdoutFmt for T {}

#[derive(Clone, Debug, Copy)]
pub struct Foreground<T>(T, Option<&'static str>);

impl<T: Display> Display for Foreground<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        if let Some(col) = self.1 {
            write!(f, "{}{}{}", col, self.0, color::RESET_FG)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Background<T>(T, Option<&'static str>);

impl<T: Display> Display for Background<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        if let Some(col) = self.1 {
            write!(f, "{}{}{}", col, self.0, color::RESET_BG)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum WrappedWriter<W: Write> {
    Strip(Writer<W>),
    Keep(W),
}

impl<W: Write> WrappedWriter<W> {
    pub(crate) fn new(w: W, config: &Config) -> Self {
        match config.ansi_mode {
            AnsiMode::Off => Self::Strip(Writer::new(w)),
            AnsiMode::On => Self::Keep(w),
        }
    }
}

impl<W: Write> Write for WrappedWriter<W> {
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Strip(w) => w.flush(),
            Self::Keep(w) => w.flush(),
        }
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Strip(w) => w.write(buf),
            Self::Keep(w) => w.write(buf),
        }
    }
}

/// A type that can generate distinct 8-bit colors.
pub struct ColorGenerator {
    state: [u16; 3],
    min_brightness: f32,
}

impl Default for ColorGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorGenerator {
    /// Create a new [`ColorGenerator`] with the given pre-chosen state.
    ///
    /// The minimum brightness can be used to control the colour brightness (0.0 - 1.0). The default is 0.5.
    pub const fn from_state(state: [u16; 3], min_brightness: f32) -> Self {
        Self {
            state,
            min_brightness: min_brightness.max(0.0).min(1.0),
        }
    }

    /// Create a new [`ColorGenerator`] with the default state.
    pub const fn new() -> Self {
        Self::from_state([30000, 15000, 35000], 0.5)
    }

    /// Generate the next colour in the sequence.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> String {
        let mut i = 0;
        while i < 3 {
            self.state[i] = (self.state[i] as usize).wrapping_add(40503 * (i * 4 + 1130)) as u16;
            i += 1;
        }
        let r = (self.state[0] >> 8) as u8;
        let g = (self.state[1] >> 8) as u8;
        let b = (self.state[2] >> 8) as u8;
        color::fg_rgb(r, g, b)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn const_colors() {
//         const COLORS: [Color; 3] = {
//             let mut gen = ColorGenerator::new();
//             [gen.next(), gen.next(), gen.next()]
//         };
//         assert_ne!(COLORS[0], COLORS[1]);
//         assert_ne!(COLORS[1], COLORS[2]);
//         assert_ne!(COLORS[2], COLORS[0]);
//     }
// }
