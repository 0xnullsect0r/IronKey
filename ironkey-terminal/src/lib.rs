//! ironkey-terminal — PTY spawning, ANSI parsing, and terminal state management.
//!
//! Provides a `PtySession` that wraps a pseudo-terminal running a shell,
//! and a `TerminalState` that accumulates rendered lines for display in iced.

pub mod pty;
pub mod renderer;

pub use pty::PtySession;
pub use renderer::{AnsiColor, TerminalLine, TerminalSpan, TerminalState};
