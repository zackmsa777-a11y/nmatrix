use crate::engine::{GlyphSet, Mode, Palette};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const HELP: &str = "nmatrix — native Rust terminal animation\n\n\
Usage: nmatrix [OPTIONS]\n\n\
  --mode       rain|waterfall|waves|spiral|glitch|starfield\n\
  --palette    emerald|cyan|violet|amber|rainbow\n\
  --glyphs     matrix|binary|hex\n\
  --speed      0.15–8 (default 1)\n\
  --density    0.2–1.4 (default 0.7)\n\
  --fps        1–120 (default 60)\n\
  --seed       unsigned integer\n\
  --demo       automatically cycle modes and palettes\n\
  --help       show this help\n\
  --version    show version\n\n\
Keys: 1–6 modes | arrows/m cycle | c colors | g glyphs | [ ] density\n\
      +/- speed | space pause | d demo | h HUD | ? help | q/Esc/Ctrl+C quit\n";

#[derive(Clone, Debug)]
pub struct Options {
    pub mode: Mode,
    pub palette: Palette,
    pub glyphs: GlyphSet,
    pub speed: f32,
    pub density: f32,
    pub fps: u32,
    pub seed: u64,
    pub demo: bool,
    pub help: bool,
    pub version: bool,
}

impl Options {
    pub fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut result = Self {
            mode: Mode::Rain,
            palette: Palette::Emerald,
            glyphs: GlyphSet::Matrix,
            speed: 1.0,
            density: 0.7,
            fps: 60,
            seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            demo: false,
            help: false,
            version: false,
        };
        let mut iter = args.into_iter();
        while let Some(flag) = iter.next() {
            match flag.as_str() {
                "--help" | "-h" => result.help = true,
                "--version" | "-V" => result.version = true,
                "--demo" => result.demo = true,
                "--mode" | "--palette" | "--glyphs" | "--speed" | "--density" | "--fps"
                | "--seed" => {
                    let value = iter.next().ok_or_else(|| format!("{flag} needs a value"))?;
                    let invalid = || format!("invalid {flag}: {value}");
                    match flag.as_str() {
                        "--mode" => result.mode = Mode::parse(&value).ok_or_else(invalid)?,
                        "--palette" => {
                            result.palette = Palette::parse(&value).ok_or_else(invalid)?
                        }
                        "--glyphs" => {
                            result.glyphs = GlyphSet::parse(&value).ok_or_else(invalid)?
                        }
                        "--speed" => {
                            result.speed = value.parse().map_err(|_| invalid())?;
                            if !result.speed.is_finite() || !(0.15..=8.0).contains(&result.speed) {
                                return Err("--speed must be between 0.15 and 8".into());
                            }
                        }
                        "--density" => {
                            result.density = value.parse().map_err(|_| invalid())?;
                            if !result.density.is_finite() || !(0.2..=1.4).contains(&result.density)
                            {
                                return Err("--density must be between 0.2 and 1.4".into());
                            }
                        }
                        "--fps" => {
                            result.fps = value.parse().map_err(|_| invalid())?;
                            if !(1..=120).contains(&result.fps) {
                                return Err("--fps must be between 1 and 120".into());
                            }
                        }
                        "--seed" => result.seed = value.parse().map_err(|_| invalid())?,
                        _ => unreachable!(),
                    }
                }
                _ => return Err(format!("unknown option: {flag}; use --help")),
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(u8),
    Left,
    Right,
    Up,
    Down,
    Escape,
}

pub struct Controls {
    pub mode: Mode,
    pub palette: Palette,
    pub glyphs: GlyphSet,
    pub speed: f32,
    pub density: f32,
    pub paused: bool,
    pub hud: bool,
    pub help: bool,
    pub demo: bool,
    pub running: bool,
}
impl Controls {
    pub fn new(options: Options) -> Self {
        Self {
            mode: options.mode,
            palette: options.palette,
            glyphs: options.glyphs,
            speed: options.speed,
            density: options.density,
            paused: false,
            hud: true,
            help: false,
            demo: options.demo,
            running: true,
        }
    }
    pub fn key(&mut self, key: Key) -> bool {
        match key {
            Key::Char(b'q' | b'Q' | 3) | Key::Escape => self.running = false,
            Key::Char(c @ b'1'..=b'6') => self.mode = Mode::ALL[(c - b'1') as usize],
            Key::Right | Key::Char(b'm') => self.mode = self.mode.next(1),
            Key::Left => self.mode = self.mode.next(-1),
            Key::Char(b'c') => self.palette = self.palette.next(1),
            Key::Char(b'g') => self.glyphs = self.glyphs.next(1),
            Key::Char(b'+' | b'=') | Key::Up => self.speed = (self.speed * 1.2).min(8.0),
            Key::Char(b'-' | b'_') | Key::Down => self.speed = (self.speed / 1.2).max(0.15),
            Key::Char(b']') => self.density = (self.density + 0.1).min(1.4),
            Key::Char(b'[') => self.density = (self.density - 0.1).max(0.2),
            Key::Char(b' ') => self.paused = !self.paused,
            Key::Char(b'h') => self.hud = !self.hud,
            Key::Char(b'?') => self.help = !self.help,
            Key::Char(b'd') => self.demo = !self.demo,
            _ => return false,
        }
        true
    }
}

#[derive(Default)]
pub struct InputDecoder {
    pending: VecDeque<u8>,
    escape_since: Option<Duration>,
}
impl InputDecoder {
    pub fn push(&mut self, bytes: &[u8], now: Duration) {
        for &byte in bytes {
            self.pending.push_back(byte);
        }
        if self.pending.len() > 1024 {
            self.pending.clear();
            self.escape_since = None;
        }
        if self.pending.front() == Some(&27) && self.escape_since.is_none() {
            self.escape_since = Some(now);
        }
    }
    pub fn events(&mut self, now: Duration) -> Vec<Key> {
        let mut events = Vec::new();
        while let Some(&byte) = self.pending.front() {
            if byte != 27 {
                self.pending.pop_front();
                self.escape_since = None;
                if byte == 3 || (32..=126).contains(&byte) {
                    events.push(Key::Char(byte));
                }
                continue;
            }
            let start = *self.escape_since.get_or_insert(now);
            let expired = now.saturating_sub(start) >= Duration::from_millis(35);
            let second = self.pending.get(1).copied();
            if matches!(second, Some(b'[' | b'O')) {
                if let Some(end) =
                    (2..self.pending.len()).find(|&i| (0x40..=0x7e).contains(&self.pending[i]))
                {
                    let final_byte = self.pending[end];
                    for _ in 0..=end {
                        self.pending.pop_front();
                    }
                    self.escape_since = None;
                    match final_byte {
                        b'C' => events.push(Key::Right),
                        b'D' => events.push(Key::Left),
                        b'A' => events.push(Key::Up),
                        b'B' => events.push(Key::Down),
                        _ => {}
                    }
                    continue;
                }
                if !expired {
                    break;
                }
                self.pending.clear();
            } else if second.is_none() && !expired {
                break;
            } else {
                self.pending.pop_front();
            }
            self.escape_since = None;
            events.push(Key::Escape);
        }
        events
    }
}
