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
    let n = c as u32;
    if c.is_control()
        || matches!(n, 0x0300..=0x036f | 0x064b..=0x065f | 0x200b..=0x200f | 0xfe00..=0xfe0f)
    {
        0
    } else if matches!(n, 0x1100..=0x115f | 0x2329..=0x232a | 0x2e80..=0xa4cf | 0xac00..=0xd7a3 | 0xf900..=0xfaff | 0xfe10..=0xfe19 | 0xfe30..=0xfe6f | 0xff00..=0xff60 | 0xffe0..=0xffe6 | 0x1f300..=0x1faff | 0x20000..=0x3ffff)
    {
        2
    } else {
        1
    }
}
pub fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}
pub fn clip_text(text: &str, width: usize) -> String {
    let mut used = 0;
    text.chars()
        .take_while(|&c| {
            used += char_width(c);
            used <= width
        })
        .collect()
}
