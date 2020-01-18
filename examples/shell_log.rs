//! shell_log - run a shell and log all output to a file.
//!
//! This utility has the same basic functionality as the standard
//! unix script(1) utility.
//!
//! It spawns a shell, using a pseudo-terminal, relays all the i/o
//! between the pty and the local tty, and also log all output
//! of the shell and any commands it runs to a file.
//!
use std::io;
use std::process::exit;

use termion::raw::IntoRawMode;

use tokio::prelude::*;
use tokio::task;
use tokio_process_pty::Command;

// handy helper.
type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
    if let Err(e) = run_shell().await {
        println!("run_shell: {}", e);
    }
    exit(0);
}

async fn run_shell() -> Result<()> {
    // open output file.
    let logfile = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("shell_log.txt")?;

    // get terminal size.
    let (rows, cols) = termion::terminal_size()?;

    // spawn a shell.
    let mut child = Command::new("/bin/sh")
        .pty()
        .pty_size(cols, rows)
        .new_session()
        .spawn()?;
    println!("spawned /bin/sh, pid: {}. logging to shell_log.txt.", child.id());

    // set the local tty into raw mode.
    let raw_guard = io::stdout().into_raw_mode()?;

    // handles to process stdin/stdout.
    let pty_stdin = child.stdin.take().unwrap();
    let pty_stdout = child.stdout.take().unwrap();

    // copy pty stdout -> tty stdout, and log.
    let from_pty = task::spawn(async move {
        copy_pty_tty(pty_stdout, io::stdout(), logfile).await
    });

    // copy tty_stdin -> pty_stdin.
    let to_pty = task::spawn(async move {
        copy_tty_pty(io::stdin(), pty_stdin).await
    });

    // wait for the first one to finish.
    let _ = futures_util::future::select(from_pty, to_pty).await;
    drop(raw_guard);

    // Collect exit status.
    let status = child.await?;
    println!("process exited with status {:?}", status);

    Ok(())
}

// copy AsyncRead -> Write.
async fn copy_pty_tty<R, W, L>(mut from: R, mut to: W, mut log: L) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: io::Write + Send + 'static,
    L: io::Write + Send + 'static,
{
    let mut buffer = [0u8; 1000];
    loop {
        let n = from.read(&mut buffer[..]).await?;
        if n == 0 {
            break;
        }
        // tokio doesn't have async-write to stdout, so use block-in-place.
        task::block_in_place(|| {
            to.write_all(&buffer[0..n])?;
            to.flush()?;
            log.write_all(&buffer[0..n])?;
            log.flush()
        })?;
    }
    Ok(())
}

// copy Read -> AsyncWrite.
async fn copy_tty_pty<R, W>(mut from: R, mut to: W) -> io::Result<()>
where
    R: io::Read + Send + 'static,
    W: AsyncWrite + Unpin,
{
    loop {
        let mut buffer = [0u8; 1000];
        // tokio doesn't have async-read from stdin, so use block-in-place.
        let n = task::block_in_place(|| {
            from.read(&mut buffer[..])
        })?;
        if n == 0 {
            break;
        }
        to.write_all(&buffer[..n]).await?;
    }
    Ok(())
}

