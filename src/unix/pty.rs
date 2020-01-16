use std::io;
use std::os::unix::io::{RawFd, AsRawFd};
use std::process::Stdio;

use nix::fcntl::{open, fcntl, FcntlArg, OFlag};
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::sys::termios::{tcgetattr, tcsetattr, cfmakeraw};
use nix::sys::termios::SpecialCharacterIndices::*;
use nix::sys::termios::{ControlFlags, InputFlags, OutputFlags, LocalFlags, SetArg};
use nix::sys::termios::Termios;
use nix::sys::stat::Mode;
use nix::unistd::{dup2, setsid, close};

#[derive(Debug)]
pub(crate) struct Pty {
    pub master: RawFd,
    pub slave:  RawFd,
    pub stdin:  bool,
    pub stdout: bool,
    pub stderr: bool,
}

impl Pty {
    // Open a pseudo tty master/slave pair. set slave to defaults, and master to non-blocking.
    pub(crate) fn open(rows: u16, cols: u16) -> io::Result<Pty> {

        // open a pty master/slave set
        let winsize = if rows > 0 && cols > 0 {
            Some(Winsize{ ws_row: rows, ws_col: cols, ws_xpixel: 0, ws_ypixel: 0 })
        } else {
            None
        };
        let OpenptyResult{ master, slave } = openpty(winsize.as_ref(), None).map_err(to_io_error)?;

        // set master to non-blocking.
        let mut oflag = OFlag::empty();
        oflag.insert(OFlag::O_CLOEXEC);
        oflag.insert(OFlag::O_NONBLOCK);
        fcntl(master, FcntlArg::F_SETFL(oflag)).map_err(to_io_error)?;

        // set master into raw mode.
        let mut termios = tcgetattr(master).map_err(to_io_error)?;
        cfmakeraw(&mut termios);
        tcsetattr(master, SetArg::TCSANOW, &termios).map_err(to_io_error)?;

        // get current settings of the slave terminal, change them to
        // cooked mode, and set them again.
        let mut termios = tcgetattr(slave).map_err(to_io_error)?;
        set_cooked(&mut termios);
        tcsetattr(slave, SetArg::TCSANOW, &termios).map_err(to_io_error)?;

        Ok(Pty {
            master,
            slave,
            stdin: true,
            stdout: true,
            stderr: true,
        })
    }
}

// Nix error to std::io::Error.
fn to_io_error(n: nix::Error) -> io::Error {
    match n {
        nix::Error::Sys(errno) =>io::Error::from_raw_os_error(errno as i32),
        nix::Error::InvalidPath =>io::Error::new(io::ErrorKind::InvalidInput, "invalid path"),
        nix::Error::InvalidUtf8 => io::Error::new(io::ErrorKind::InvalidData, "invalid utf8"),
        nix::Error::UnsupportedOperation => io::Error::new(io::ErrorKind::Other, "unsupported operation"),
    }
}

// Change termios to cooked mode.
fn set_cooked(termios: &mut Termios) {

    // default control chars, mostly.
    termios.control_chars[VINTR as usize] = 0o003;
    termios.control_chars[VQUIT as usize] = 0o034;
    termios.control_chars[VERASE as usize] = 0o177;
    termios.control_chars[VEOF as usize] = 0o004;
    termios.control_chars[VSTART as usize] = 0o021;
    termios.control_chars[VSTOP as usize] = 0o023;
    termios.control_chars[VSUSP as usize] = 0o032;
    termios.control_chars[VWERASE as usize] = 0o027;
    termios.control_chars[VLNEXT as usize] = 0o026;

    // default cooked control flags.
    let mut cflags = ControlFlags::empty();
    cflags.insert(ControlFlags::CS8);
    cflags.insert(ControlFlags::CREAD);
    termios.control_flags = cflags;

    // default cooked input flags.
    let mut iflags = InputFlags::empty();
    iflags.insert(InputFlags::ICRNL);
    iflags.insert(InputFlags::IXON);
    iflags.insert(InputFlags::IXANY);
    iflags.insert(InputFlags::IMAXBEL);
    iflags.insert(InputFlags::IUTF8);
    termios.input_flags = iflags;

    // default cooked output flags.
    let mut oflags = OutputFlags::empty();
    oflags.insert(OutputFlags::OPOST);
    oflags.insert(OutputFlags::ONLCR);
    oflags.insert(OutputFlags::NL0);
    oflags.insert(OutputFlags::CR0);
    oflags.insert(OutputFlags::TAB0);
    oflags.insert(OutputFlags::BS0);
    oflags.insert(OutputFlags::VT0);
    oflags.insert(OutputFlags::FF0);
    termios.output_flags = oflags;

    // default local flags.
    let mut lflags = LocalFlags::empty();
    lflags.insert(LocalFlags::ISIG);
    lflags.insert(LocalFlags::ICANON);
    lflags.insert(LocalFlags::IEXTEN);
    lflags.insert(LocalFlags::ECHO);
    lflags.insert(LocalFlags::ECHOE);
    lflags.insert(LocalFlags::ECHOK);
    lflags.insert(LocalFlags::ECHOCTL);
    lflags.insert(LocalFlags::ECHOKE);
    termios.local_flags = lflags;
}
