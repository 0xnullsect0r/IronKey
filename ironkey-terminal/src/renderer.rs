//! ANSI escape sequence parser and terminal state accumulator.
//!
//! Uses the `vte` crate to parse VT100/ANSI escape codes and builds a
//! scrollback buffer of `TerminalLine`s ready for rendering in iced.

use vte::{Parser, Perform};

// ──────────────────────────────────────────────────────────────────────────────
// Color types
// ──────────────────────────────────────────────────────────────────────────────

/// Terminal colour (maps to iced `Color` in the UI layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColor {
    /// xterm 256-colour index
    Indexed(u8),
    /// 24-bit RGB
    Rgb(u8, u8, u8),
    /// Terminal default foreground
    DefaultFg,
    /// Terminal default background
    DefaultBg,
}

impl AnsiColor {
    /// Convert to an `[f32; 4]` RGBA tuple for iced.
    pub fn to_rgba(&self) -> [f32; 4] {
        match self {
            AnsiColor::DefaultFg => [0.91, 0.929, 0.953, 1.0],
            AnsiColor::DefaultBg => [0.039, 0.047, 0.059, 1.0],
            AnsiColor::Rgb(r, g, b) => {
                [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, 1.0]
            }
            AnsiColor::Indexed(i) => indexed_to_rgba(*i),
        }
    }
}

/// Map xterm 256-colour index to RGBA.
fn indexed_to_rgba(i: u8) -> [f32; 4] {
    // Standard 16 colours
    let (r, g, b): (u8, u8, u8) = match i {
        0 => (0, 0, 0),
        1 => (170, 0, 0),
        2 => (0, 170, 0),
        3 => (170, 85, 0),
        4 => (0, 0, 170),
        5 => (170, 0, 170),
        6 => (0, 170, 170),
        7 => (170, 170, 170),
        8 => (85, 85, 85),
        9 => (255, 85, 85),
        10 => (85, 255, 85),
        11 => (255, 255, 85),
        12 => (85, 85, 255),
        13 => (255, 85, 255),
        14 => (85, 255, 255),
        15 => (255, 255, 255),
        16..=231 => {
            // 6x6x6 colour cube
            let idx = i - 16;
            let b_val = idx % 6;
            let g_val = (idx / 6) % 6;
            let r_val = idx / 36;
            let scale = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (scale(r_val), scale(g_val), scale(b_val))
        }
        232..=255 => {
            // Greyscale ramp
            let level = 8 + (i - 232) * 10;
            (level, level, level)
        }
    };
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

// ──────────────────────────────────────────────────────────────────────────────
// Terminal state
// ──────────────────────────────────────────────────────────────────────────────

/// A styled text span within a terminal line.
#[derive(Debug, Clone)]
pub struct TerminalSpan {
    pub text: String,
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for TerminalSpan {
    fn default() -> Self {
        Self {
            text: String::new(),
            fg: AnsiColor::DefaultFg,
            bg: AnsiColor::DefaultBg,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

/// A full line of terminal output.
#[derive(Debug, Clone, Default)]
pub struct TerminalLine {
    pub spans: Vec<TerminalSpan>,
}

impl TerminalLine {
    /// Returns all text content concatenated.
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

/// Accumulated terminal state: a scrollback buffer of lines.
pub struct TerminalState {
    pub lines: Vec<TerminalLine>,
    pub max_lines: usize,
    parser: Parser,
    current_line: TerminalLine,
    current_span: TerminalSpan,
}

impl TerminalState {
    /// Create a new terminal state with a scrollback buffer of `max_lines`.
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::new(),
            max_lines,
            parser: Parser::new(),
            current_line: TerminalLine::default(),
            current_span: TerminalSpan::default(),
        }
    }

    /// Feed raw bytes from the PTY into the parser.
    pub fn feed(&mut self, data: &[u8]) {
        let mut performer = Performer {
            lines: &mut self.lines,
            current_line: &mut self.current_line,
            current_span: &mut self.current_span,
            max_lines: self.max_lines,
        };
        self.parser.advance(&mut performer, data);
    }

    /// Return the last `n` lines of the scrollback buffer.
    pub fn tail(&self, n: usize) -> &[TerminalLine] {
        let start = self.lines.len().saturating_sub(n);
        &self.lines[start..]
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// vte Perform implementation
// ──────────────────────────────────────────────────────────────────────────────

struct Performer<'a> {
    lines: &'a mut Vec<TerminalLine>,
    current_line: &'a mut TerminalLine,
    current_span: &'a mut TerminalSpan,
    max_lines: usize,
}

impl<'a> Performer<'a> {
    fn flush_span(&mut self) {
        if !self.current_span.text.is_empty() {
            let span = std::mem::take(self.current_span);
            self.current_line.spans.push(span);
        }
    }

    fn commit_line(&mut self) {
        self.flush_span();
        let line = std::mem::take(self.current_line);
        self.lines.push(line);
        // Trim scrollback
        if self.lines.len() > self.max_lines {
            self.lines.drain(0..self.lines.len() - self.max_lines);
        }
    }
}

impl<'a> Perform for Performer<'a> {
    fn print(&mut self, c: char) {
        self.current_span.text.push(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | b'\r' => {
                if byte == b'\r' {
                    // Carriage return alone: don't commit a new line
                    return;
                }
                self.commit_line();
            }
            b'\x08' => {
                // Backspace
                if let Some(span) = self.current_line.spans.last_mut() {
                    span.text.pop();
                } else {
                    self.current_span.text.pop();
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        if action == 'm' {
            // SGR — Select Graphic Rendition
            self.flush_span();
            let mut iter = params.iter();
            loop {
                let param = match iter.next() {
                    Some(p) => p,
                    None => break,
                };
                let code = param.first().copied().unwrap_or(0);
                match code {
                    0 => {
                        // Reset
                        self.current_span.fg = AnsiColor::DefaultFg;
                        self.current_span.bg = AnsiColor::DefaultBg;
                        self.current_span.bold = false;
                        self.current_span.italic = false;
                        self.current_span.underline = false;
                    }
                    1 => self.current_span.bold = true,
                    3 => self.current_span.italic = true,
                    4 => self.current_span.underline = true,
                    22 => self.current_span.bold = false,
                    23 => self.current_span.italic = false,
                    24 => self.current_span.underline = false,
                    // Standard foreground colours (30–37)
                    30..=37 => {
                        self.current_span.fg = AnsiColor::Indexed(code as u8 - 30);
                    }
                    38 => {
                        // Extended fg: 38;5;n or 38;2;r;g;b
                        self.handle_extended_color(true, &mut iter);
                    }
                    39 => self.current_span.fg = AnsiColor::DefaultFg,
                    // Standard background colours (40–47)
                    40..=47 => {
                        self.current_span.bg = AnsiColor::Indexed(code as u8 - 40);
                    }
                    48 => {
                        self.handle_extended_color(false, &mut iter);
                    }
                    49 => self.current_span.bg = AnsiColor::DefaultBg,
                    // Bright foreground (90–97)
                    90..=97 => {
                        self.current_span.fg = AnsiColor::Indexed(code as u8 - 90 + 8);
                    }
                    // Bright background (100–107)
                    100..=107 => {
                        self.current_span.bg = AnsiColor::Indexed(code as u8 - 100 + 8);
                    }
                    _ => {}
                }
            }
        }
    }

    fn hook(
        &mut self,
        _params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

impl<'a> Performer<'a> {
    fn handle_extended_color(
        &mut self,
        is_fg: bool,
        iter: &mut vte::ParamsIter<'_>,
    ) {
        let sub = match iter.next() {
            Some(p) => p.first().copied().unwrap_or(0),
            None => return,
        };
        match sub {
            5 => {
                // 256-colour index
                if let Some(p) = iter.next() {
                    let idx = p.first().copied().unwrap_or(0) as u8;
                    if is_fg {
                        self.current_span.fg = AnsiColor::Indexed(idx);
                    } else {
                        self.current_span.bg = AnsiColor::Indexed(idx);
                    }
                }
            }
            2 => {
                // True colour
                let r = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                let g = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                let b = iter.next().and_then(|p| p.first().copied()).unwrap_or(0) as u8;
                if is_fg {
                    self.current_span.fg = AnsiColor::Rgb(r, g, b);
                } else {
                    self.current_span.bg = AnsiColor::Rgb(r, g, b);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_accumulation() {
        let mut state = TerminalState::new(100);
        state.feed(b"hello\nworld\n");
        assert_eq!(state.lines.len(), 2);
        assert_eq!(state.lines[0].plain_text(), "hello");
        assert_eq!(state.lines[1].plain_text(), "world");
    }

    #[test]
    fn scrollback_limit() {
        let mut state = TerminalState::new(5);
        for i in 0..10 {
            state.feed(format!("line{}\n", i).as_bytes());
        }
        assert!(state.lines.len() <= 5);
    }

    #[test]
    fn indexed_colour_mapping() {
        let rgba = indexed_to_rgba(0);
        assert_eq!(rgba, [0.0, 0.0, 0.0, 1.0]);
        let rgba15 = indexed_to_rgba(15);
        assert_eq!(rgba15, [1.0, 1.0, 1.0, 1.0]);
    }
}
