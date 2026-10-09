#![cfg(target_os = "linux")]
use nmatrix::terminal::Terminal;
static LOCAL_TERMINAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn attributes(fd: i32) -> libc::termios {
    let mut result = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::tcgetattr(fd, &mut result) }, 0);
    result
}
fn same_attributes(a: libc::termios, b: libc::termios) {
    assert_eq!(a.c_iflag, b.c_iflag);
    assert_eq!(a.c_oflag, b.c_oflag);
    assert_eq!(a.c_cflag, b.c_cflag);
    assert_eq!(a.c_lflag, b.c_lflag);
    assert_eq!(a.c_cc, b.c_cc);
    assert_eq!(a.c_ispeed, b.c_ispeed);
    assert_eq!(a.c_ospeed, b.c_ospeed);
}
fn open_pair() -> (File, File) {
    let mut master = -1;
    let mut slave = -1;
    let size = libc::winsize {
        ws_row: 30,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null(),
                &size,
            )
        },
        0
    );
    for fd in [master, slave] {
        assert_eq!(
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
    }
    assert_eq!(
        unsafe { libc::fcntl(master, libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) }
}

struct Session {
    master: File,
    slave: File,
    child: Child,
    original: libc::termios,
    output: Vec<u8>,
}
impl Session {
    fn new(args: &[&str], basic: bool) -> Self {
        let (master, slave) = open_pair();
        let original = attributes(slave.as_raw_fd());
        let mut command = Command::new(env!("CARGO_BIN_EXE_nmatrix"));
        command
            .args(args)
            .env("TERM", if basic { "linux" } else { "xterm-256color" })
            .env("COLORTERM", if basic { "" } else { "truecolor" })
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()));
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 || libc::ioctl(0, libc::TIOCSCTTY, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().unwrap();
        Self {
            master,
            slave,
            child,
            original,
            output: Vec::new(),
        }
    }
    fn pump(&mut self, duration: Duration) {
        let end = Instant::now() + duration;
        let mut buffer = [0u8; 65536];
        loop {
            match self.master.read(&mut buffer) {
                Ok(n) if n > 0 => self.output.extend_from_slice(&buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Ok(_) => {}
                Err(e) => panic!("PTY read: {e}"),
            }
            if Instant::now() >= end {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    fn wait_for(&mut self, needle: &str) {
        let end = Instant::now() + Duration::from_secs(3);
        while !String::from_utf8_lossy(&self.output).contains(needle) {
            self.pump(Duration::from_millis(10));
            if self.child.try_wait().unwrap().is_some() || Instant::now() >= end {
                panic!(
                    "missing {needle:?}: {}",
                    String::from_utf8_lossy(&self.output)
                );
            }
        }
    }
    fn send(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
    }
    fn resize(&self, width: u16, height: u16) {
        let size = libc::winsize {
            ws_row: height,
            ws_col: width,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            unsafe { libc::ioctl(self.slave.as_raw_fd(), libc::TIOCSWINSZ, &size) },
            0
        );
    }
    fn finish(&mut self, expected: i32) {
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            self.pump(Duration::from_millis(5));
            if let Some(status) = self.child.try_wait().unwrap() {
                self.pump(Duration::from_millis(5));
                assert_eq!(
                    status.code(),
                    Some(expected),
                    "{}",
                    String::from_utf8_lossy(&self.output)
                );
                break;
            }
            assert!(Instant::now() < end, "app did not exit");
        }
        same_attributes(self.original, attributes(self.slave.as_raw_fd()));
        assert!(
            self.output
                .windows(b"\x1b[?7h".len())
                .any(|w| w == b"\x1b[?7h")
        );
        assert!(
            self.output
                .windows(b"\x1b[?25h".len())
                .any(|w| w == b"\x1b[?25h")
        );
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn all_modes_start_resize_and_quit_cleanly() {
    for mode in [
        "rain",
        "waterfall",
        "waves",
        "spiral",
        "glitch",
        "starfield",
    ] {
        let mut s = Session::new(&["--mode", mode, "--seed", "7"], false);
        s.wait_for("NMATRIX");
        s.send(b"cg[]+-dh?");
        for (w, h) in [(1, 1), (120, 40), (4, 2), (100, 30)] {
            s.resize(w, h);
            s.pump(Duration::from_millis(40));
        }
        s.send(b"q");
        s.finish(0);
    }
}

#[test]
fn fragmented_arrows_do_not_quit_the_application() {
    let mut s = Session::new(&[], false);
    s.wait_for("RAIN");
    s.send(b"\x1b");
    s.pump(Duration::from_millis(5));
    s.send(b"[");
    s.pump(Duration::from_millis(5));
    s.send(b"C");
    s.wait_for("WATERFALL");
    s.send(b"q");
    s.finish(0);
}

#[test]
fn low_fps_controls_respond_without_waiting_for_the_next_frame() {
    let mut s = Session::new(&["--fps", "1"], false);
    s.wait_for("NMATRIX");
    let start = Instant::now();
    s.send(b"c");
    s.wait_for("CYAN");
    s.send(b"q");
    s.finish(0);
    assert!(start.elapsed() < Duration::from_millis(400));
}

#[test]
fn pause_tiny_window_escape_and_ctrl_c_restore_terminal() {
    for quit in [b"\x1b".as_slice(), b"\x03".as_slice()] {
        let mut s = Session::new(&[], false);
        s.wait_for("NMATRIX");
        s.send(b" ");
        s.wait_for("PAUSED");
        s.resize(1, 1);
        s.pump(Duration::from_millis(50));
        s.send(quit);
        s.finish(0);
    }
}

#[test]
fn signals_restore_terminal_and_preserve_exit_status() {
    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
        let mut s = Session::new(&[], false);
        s.wait_for("NMATRIX");
        assert_eq!(unsafe { libc::kill(s.child.id() as i32, signal) }, 0);
        s.finish(128 + signal);
    }
}

#[test]
fn basic_terminal_never_receives_truecolor_sequences() {
    let mut s = Session::new(&[], true);
    s.wait_for("NMATRIX");
    s.send(b"q");
    s.finish(0);
    let text = String::from_utf8_lossy(&s.output);
    assert!(!text.contains("38;2;") && !text.contains("48;2;"));
}

#[test]
fn invalid_arguments_and_piped_output_do_not_enter_raw_mode() {
    for (args, status) in [
        (vec!["--speed", "nan"], 2),
        (vec!["--fps", "0"], 2),
        (vec![], 1),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_nmatrix"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(status));
        assert!(!output.stdout.contains(&27));
        assert!(!output.stderr.is_empty());
    }
    let help = Command::new(env!("CARGO_BIN_EXE_nmatrix"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success() && String::from_utf8_lossy(&help.stdout).contains("--demo"));
}

#[test]
fn panic_unwinds_the_terminal_guard_and_restores_attributes() {
    let _lock = LOCAL_TERMINAL.lock().unwrap();
    let (_master, slave) = open_pair();
    let original = attributes(slave.as_raw_fd());
    let original_flags = unsafe { libc::fcntl(slave.as_raw_fd(), libc::F_GETFL) };
    let outcome = std::panic::catch_unwind(|| {
        let _guard = Terminal::open(slave.as_raw_fd(), slave.as_raw_fd()).unwrap();
        panic!("deliberate cleanup probe");
    });
    assert!(outcome.is_err());
    same_attributes(original, attributes(slave.as_raw_fd()));
    assert_eq!(
        unsafe { libc::fcntl(slave.as_raw_fd(), libc::F_GETFL) },
        original_flags
    );
}

#[test]
fn signal_exits_even_when_terminal_output_is_not_drained() {
    let mut s = Session::new(&["--fps", "120", "--mode", "waterfall"], false);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(unsafe { libc::kill(s.child.id() as i32, libc::SIGTERM) }, 0);
    let deadline = Instant::now() + Duration::from_millis(400);
    loop {
        if let Some(status) = s.child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(143));
            break;
        }
        assert!(
            Instant::now() < deadline,
            "blocked output prevented signal exit"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    same_attributes(s.original, attributes(s.slave.as_raw_fd()));
}

#[test]
fn positive_submillisecond_reads_sleep_instead_of_busy_polling() {
    let _lock = LOCAL_TERMINAL.lock().unwrap();
    let (_master, slave) = open_pair();
    let guard = Terminal::open(slave.as_raw_fd(), slave.as_raw_fd()).unwrap();
    let start = Instant::now();
    for _ in 0..20 {
        assert!(guard.read(Duration::from_micros(500)).unwrap().is_empty());
    }
    assert!(start.elapsed() >= Duration::from_millis(10));
}
