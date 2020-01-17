use std::io;
use termion::raw::IntoRawMode;
use tokio::prelude::*;
use tokio_process_pty::Command;

// handy helper.
type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // get local terminal size.
    let (rows, cols) = termion::terminal_size()?;

    // spawn a shell.
    let mut child = Command::new("/bin/sh")
        .pty()
        .pty_size(cols, rows)
        .new_session()
        .spawn()
        .expect("failed to spawn");
    println!("spawned /bin/sh, pid: {}", child.id());

    // set the local tty into raw mode.
    let _guard = io::stdout().into_raw_mode()?;

    // handles to process stdin/stdout.
    let pty_stdin = child.stdin.take().unwrap();
    let pty_stdout = child.stdout.take().unwrap();

    // copy pty stdout -> tty stdout.
    let from_pty = tokio::spawn(async move {
        copy_pty_tty(pty_stdout, io::stdout()).await
    });

    // copy tty_stdin -> pty_stdin.
    let to_pty = tokio::spawn(async move {
        copy_tty_pty(io::stdin(), pty_stdin).await
    });

    // await for one of the two to finish.
    let _ = futures_util::future::select(from_pty, to_pty).await;

    // restore original tty mode.
    //tty_restore(io::stdin(), &termios)?;

    // Collect exit status.
    let status = child.await?;

    Ok(())
}

// copy AsyncRead -> Write.
async fn copy_pty_tty<R, W>(mut from: R, mut to: W) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: io::Write,
{
    let mut buffer = [0u8; 1000];
    loop {
        let n = from.read(&mut buffer[..])
            .await
            .map_err(|e| { println!("read from pty: {}\r", e); e })?;
        if n == 0 {
            println!("pty: EOF\r");
            break;
        }
        tokio::task::block_in_place(|| {
            to.write_all(&buffer[0..n])?;
            to.flush()
        }).map_err(|e| { println!("write to tty: {}\r", e); e })?;
    }
    Ok(())
}

// copy Read -> AsyncWrite.
async fn copy_tty_pty<R, W>(mut from: R, mut to: W) -> io::Result<()>
where
    R: io::Read,
    W: AsyncWrite + Unpin,
{
    let mut buffer = [0u8; 1000];
    loop {
        let n = tokio::task::block_in_place(|| {
            from.read(&mut buffer[..])
        }).map_err(|e| { println!("read from tty: {}\r", e); e })?;
        if n == 0 {
            println!("tty: EOF\r");
            break;
        }
        to.write_all(&buffer[0..n])
            .await
            .map_err(|e| { println!("write to pty: {}\r", e); e })?;
    }
    Ok(())
}

