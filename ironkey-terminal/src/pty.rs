//! PTY (pseudo-terminal) spawning and I/O management.
//!
//! On Linux, uses the `portable-pty` crate to spawn a shell (zsh or bash)
//! in a PTY and exposes read/write channels.
//!
//! On non-Linux targets, provides a stub that returns an error on spawn.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// A handle to a running PTY session.
pub struct PtySession {
    /// Shell command that was spawned
    pub shell: String,
    /// Current working directory
    pub cwd: PathBuf,

    #[cfg(target_os = "linux")]
    inner: LinuxPty,
}

#[cfg(target_os = "linux")]
struct LinuxPty {
    master: Box<dyn portable_pty::MasterPty + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn std::io::Write + Send>,
    reader: Box<dyn std::io::Read + Send>,
}

/// Options for spawning a PTY session.
#[derive(Debug, Clone)]
pub struct PtyOptions {
    /// Shell binary to run (e.g. "/usr/bin/zsh")
    pub shell: String,
    /// Initial working directory
    pub cwd: PathBuf,
    /// Terminal size
    pub cols: u16,
    pub rows: u16,
}

impl Default for PtyOptions {
    fn default() -> Self {
        Self {
            shell: find_shell(),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            cols: 120,
            rows: 40,
        }
    }
}

/// Return the best available shell on this system.
fn find_shell() -> String {
    for shell in &["/usr/bin/zsh", "/bin/zsh", "/usr/bin/bash", "/bin/bash", "/bin/sh"] {
        if std::path::Path::new(shell).exists() {
            return shell.to_string();
        }
    }
    "/bin/sh".to_string()
}

impl PtySession {
    /// Spawn a new shell in a PTY.
    pub fn spawn(opts: PtyOptions) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Self::spawn_linux(opts)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = opts;
            anyhow::bail!("PTY spawning is only supported on Linux")
        }
    }

    #[cfg(target_os = "linux")]
    fn spawn_linux(opts: PtyOptions) -> Result<Self> {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize};

        let pty_system = native_pty_system();
        let size = PtySize {
            rows: opts.rows,
            cols: opts.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .context("Failed to open PTY pair")?;

        let mut cmd = CommandBuilder::new(&opts.shell);
        cmd.cwd(&opts.cwd);

        // Set IronKey-specific environment
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env(
            "IRONKEY_VERSION",
            env!("CARGO_PKG_VERSION"),
        );

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell")?;

        let writer = pair
            .master
            .take_writer()
            .context("Failed to get PTY writer")?;
        let reader = pair
            .master
            .try_clone_reader()
            .context("Failed to get PTY reader")?;

        Ok(Self {
            shell: opts.shell,
            cwd: opts.cwd,
            inner: LinuxPty {
                master: pair.master,
                _child: child,
                writer,
                reader,
            },
        })
    }

    /// Write input bytes to the shell's stdin.
    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::io::Write;
            self.inner.writer.write_all(data).context("Writing to PTY")
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = data;
            anyhow::bail!("PTY not supported on this platform")
        }
    }

    /// Read available output bytes from the shell (non-blocking attempt).
    ///
    /// Returns the bytes read; may be 0 if no data is available yet.
    pub fn read_output(&mut self, buf: &mut [u8]) -> Result<usize> {
        #[cfg(target_os = "linux")]
        {
            use std::io::Read;
            match self.inner.reader.read(buf) {
                Ok(n) => Ok(n),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
                Err(e) => Err(e.into()),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = buf;
            Ok(0)
        }
    }

    /// Resize the terminal to `cols` × `rows`.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use portable_pty::PtySize;
            self.inner
                .master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .context("Resizing PTY")
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (cols, rows);
            Ok(())
        }
    }
}
