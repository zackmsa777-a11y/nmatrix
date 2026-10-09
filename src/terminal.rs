//! Linux terminal ownership. Drop always restores the saved input state.
use std::io;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

static INTERRUPTED: AtomicI32 = AtomicI32::new(0);
extern "C" fn signal_handler(signal: i32) {
    INTERRUPTED.store(signal, Ordering::Relaxed);
}

pub struct Terminal {
    input: i32,
    output: i32,
    original: libc::termios,
    original_output_flags: i32,
    signals: Vec<(i32, libc::sigaction)>,
}

impl Terminal {
    pub fn open(input: i32, output: i32) -> io::Result<Self> {
        // These borrowed descriptors are never closed by this guard.
        if unsafe { libc::isatty(input) } != 1 || unsafe { libc::isatty(output) } != 1 {
            return Err(io::Error::other("nmatrix needs an interactive terminal"));
        }
        let mut original: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(input, &mut original) } == -1 {
            return Err(io::Error::last_os_error());
        }
        let original_output_flags = unsafe { libc::fcntl(output, libc::F_GETFL) };
        if original_output_flags == -1 {
            return Err(io::Error::last_os_error());
        }
        let mut raw = original;
        unsafe {
            libc::cfmakeraw(&mut raw);
        }
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(input, libc::TCSANOW, &raw) } == -1 {
            return Err(io::Error::last_os_error());
        }
        let mut guard = Self {
            input,
            output,
            original,
            original_output_flags,
            signals: Vec::new(),
        };
        if unsafe {
            libc::fcntl(
                output,
                libc::F_SETFL,
                original_output_flags | libc::O_NONBLOCK,
            )
        } == -1
        {
            return Err(io::Error::last_os_error());
        }
        INTERRUPTED.store(0, Ordering::Relaxed);
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
            let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
            let mut old: libc::sigaction = unsafe { std::mem::zeroed() };
            action.sa_sigaction = signal_handler as *const () as usize;
            unsafe {
                libc::sigemptyset(&mut action.sa_mask);
            }
            if unsafe { libc::sigaction(signal, &action, &mut old) } == -1 {
                return Err(io::Error::last_os_error());
            }
            guard.signals.push((signal, old));
        }
        guard.write("\x1b[?1049h\x1b[0m\x1b[?25l\x1b[?7l\x1b[2J")?;
        Ok(guard)
    }

    pub fn signal() -> i32 {
        INTERRUPTED.load(Ordering::Relaxed)
    }

    pub fn size(&self) -> io::Result<(usize, usize)> {
        let mut size: libc::winsize = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(self.output, libc::TIOCGWINSZ, &mut size) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok((
            if size.ws_col == 0 {
                80
            } else {
                size.ws_col as usize
            },
            if size.ws_row == 0 {
                24
            } else {
                size.ws_row as usize
            },
        ))
    }

    pub fn read(&self, timeout: Duration) -> io::Result<Vec<u8>> {
        let mut poll = libc::pollfd {
            fd: self.input,
            events: libc::POLLIN,
            revents: 0,
        };
        let ms = timeout.as_nanos().div_ceil(1_000_000).min(16) as i32;
        let ready = unsafe { libc::poll(&mut poll, 1, ms) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            return if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                Ok(Vec::new())
            } else {
                Err(error)
            };
        }
        if ready == 0 || Self::signal() != 0 {
            return Ok(Vec::new());
        }
        if poll.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "terminal input closed",
            ));
        }
        let mut bytes = [0u8; 256];
        let count = unsafe { libc::read(self.input, bytes.as_mut_ptr().cast(), bytes.len()) };
        if count < 0 {
            let error = io::Error::last_os_error();
            return if matches!(
                error.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                Ok(Vec::new())
            } else {
                Err(error)
            };
        }
        Ok(bytes[..count as usize].to_vec())
    }

    pub fn write(&self, text: &str) -> io::Result<()> {
        write_fd(self.output, text.as_bytes(), Duration::from_secs(1), true)
    }
}

fn write_fd(fd: i32, bytes: &[u8], timeout: Duration, interruptible: bool) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    let mut written = 0;
    while written < bytes.len() {
        if interruptible && Terminal::signal() != 0 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "terminal output stalled",
            ));
        }
        let count =
            unsafe { libc::write(fd, bytes[written..].as_ptr().cast(), bytes.len() - written) };
        if count < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if error.kind() == io::ErrorKind::WouldBlock {
                let mut poll = libc::pollfd {
                    fd,
                    events: libc::POLLOUT,
                    revents: 0,
                };
                let ms = deadline
                    .saturating_duration_since(Instant::now())
                    .as_nanos()
                    .div_ceil(1_000_000)
                    .min(16) as i32;
                if unsafe { libc::poll(&mut poll, 1, ms) } < 0 {
                    let error = io::Error::last_os_error();
                    if error.kind() != io::ErrorKind::Interrupted {
                        return Err(error);
                    }
                }
                continue;
            }
            return Err(error);
        }
        if count == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        written += count as usize;
    }
    Ok(())
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Cleanup is best effort, but input restoration must run even if output fails.
        unsafe {
            libc::tcsetattr(self.input, libc::TCSANOW, &self.original);
            if Self::signal() != 0 {
                libc::tcflush(self.output, libc::TCOFLUSH);
            }
        }
        let _ = write_fd(
            self.output,
            b"\x18\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l",
            Duration::from_millis(50),
            false,
        );
        unsafe {
            libc::fcntl(self.output, libc::F_SETFL, self.original_output_flags);
        }
        for (signal, old) in self.signals.iter().rev() {
            unsafe {
                libc::sigaction(*signal, old, std::ptr::null_mut());
            }
        }
    }
}
