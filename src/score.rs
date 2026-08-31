//! High score, kept between sessions.
//!
//! One number in one file, in the platform's data directory -- not enough to
//! justify a dependency.

use std::fs;
use std::path::PathBuf;

const FILE: &str = "highscore";

fn dir() -> Option<PathBuf> {
    let mut path = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?)
    } else if cfg!(target_os = "macos") {
        let mut home = PathBuf::from(std::env::var_os("HOME")?);
        home.push("Library/Application Support");
        home
    } else if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(data)
    } else {
        let mut home = PathBuf::from(std::env::var_os("HOME")?);
        home.push(".local/share");
        home
    };
    path.push(env!("CARGO_PKG_NAME"));
    Some(path)
}

pub fn load() -> u32 {
    dir()
        .and_then(|dir| fs::read_to_string(dir.join(FILE)).ok())
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// Best effort: a game is not worth failing over an unwritable home
/// directory.
pub fn save(score: u32) {
    if let Some(dir) = dir() {
        if fs::create_dir_all(&dir).is_ok() {
            let _ = fs::write(dir.join(FILE), score.to_string());
        }
    }
}
