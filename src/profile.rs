//! A single saved name, loaded without rewriting it on later launches.
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

pub fn validate_name(name: &str) -> Result<String, String> {
    if name.chars().any(char::is_control) {
        return Err("Names cannot contain terminal control characters".into());
    }
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 20 || display_width(name) == 0 {
        return Err("Enter a name with 1–20 characters".into());
    }
    Ok(name.to_owned())
}

pub struct NameConfig {
    path: PathBuf,
}
impl NameConfig {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn from_env() -> io::Result<Self> {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .ok_or_else(|| io::Error::other("no config directory; use --no-name"))?;
        Ok(Self::new(root.join("nmatrix/config")))
    }
    pub fn load(&self) -> io::Result<Option<String>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let name = text.strip_prefix("name=").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid name config; use --name to replace it",
            )
        })?;
        validate_name(name.trim_end_matches(['\r', '\n']))
            .map(Some)
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))
    }
    pub fn save(&self, name: &str) -> io::Result<()> {
        let name = validate_name(name)
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("invalid config path"))?;
        fs::create_dir_all(parent)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = self
            .path
            .with_extension(format!("tmp-{}-{stamp}", std::process::id()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)?;
            writeln!(file, "name={name}")?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn char_width(c: char) -> usize {
    if c.is_control() {
        return 0;
    }
    unsafe extern "C" {
        fn wcwidth(c: libc::wchar_t) -> libc::c_int;
    }
    // Use libc's full Unicode table without changing the process-wide locale.
    // The immutable locale is shared and retained for the process lifetime.
    static UTF8_LOCALE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let locale = *UTF8_LOCALE.get_or_init(|| unsafe {
        for name in [c"C.UTF-8", c"C.utf8", c"en_US.UTF-8"] {
            let locale = libc::newlocale(libc::LC_CTYPE_MASK, name.as_ptr(), std::ptr::null_mut());
            if !locale.is_null() {
                return locale as usize;
            }
        }
        0
    });
    if locale != 0 {
        // Only this thread switches locale; restore it immediately after wcwidth.
        let width = unsafe {
            let previous = libc::uselocale(locale as libc::locale_t);
            let width = wcwidth(c as libc::wchar_t);
            libc::uselocale(previous);
            width
        };
        return width.max(0) as usize;
    }
    // A minimal system without UTF-8 locales gets conservative bounds.
    if matches!(c as u32, 0x0300..=0x036f | 0x064b..=0x065f | 0x200b..=0x200f | 0xfe00..=0xfe0f) {
        0
    } else if c.is_ascii() {
        1
    } else {
        2
    }
}

// Keep trailing combining marks with their base. Emoji presentation/keycaps
// can widen a one-column base even when wcwidth reports the selector as zero.
fn text_units(text: &str) -> impl Iterator<Item = (&str, usize)> {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let (start, c) = chars.next()?;
        let mut width = char_width(c);
        while let Some(&(_, c)) = chars.peek() {
            if char_width(c) != 0 {
                break;
            }
            if matches!(c, '\u{fe0f}' | '\u{20e3}') && width > 0 {
                width = width.max(2);
            }
            chars.next();
        }
        let end = chars.peek().map_or(text.len(), |&(offset, _)| offset);
        Some((&text[start..end], width))
    })
}
pub fn display_width(text: &str) -> usize {
    text_units(text).map(|(_, width)| width).sum()
}
pub fn clip_text(text: &str, width: usize) -> String {
    let mut used = 0;
    text_units(text)
        .take_while(|&(_, unit_width)| {
            used += unit_width;
            used <= width
        })
        .flat_map(|(unit, _)| unit.chars())
        .filter(|c| !c.is_control())
        .collect()
}
