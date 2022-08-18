#![doc(html_root_url = "https://docs.rs/tokio-process-pty/0.2.0")]
//! A version of `tokio::process` with pseudo-terminal (pty) support.
//!
//! Based on `tokio::process` from tokio 0.2.9.
//!
//! ## Description.
//!
//! This crate is a fork of [`tokio::process`](https://docs.rs/tokio/0.2.9/tokio/process),
//! that adds some new functionality to tokio's
//! [`tokio::process:Command`](https://docs.rs/tokio/0.2.9/tokio/process/struct.Command.html) struct:
//!
//! - [`pty`](struct.Command.html#method.pty):
//!   creates a pseudo-terminal device (pty) for the new process.
//!   On `spawn()`, a pty master/slave device set is created, and the
//!   process' stdin/stdout/stderr handles are connected to the slave side
//!   of the pty. The stdin/stdout/stderr methods of the returned
//!   `Child` struct all refer to the master side of the pty.
//!
//! - [`pty_size`](struct.Command.html#method.pty_size):
//!   sets the initial size (rows/columns) of the pty slave device.
//!   This is the size that is reported by the `TIOCGWINSZ` ioctl on the pty slave device.
//!
//! - [`new_session`](struct.Command.html#method.new_session):
//!   puts the new process in its own process group and session as session leader.
//!   If a pty is used, it will become the controlling tty of the new session.
//!   Signals sent to the parents' group or session do not reach the new process.
//! 

#[path = "mod.rs"]
mod process;

#[doc(inline)]
pub use process::*;

