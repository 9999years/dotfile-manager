use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

pub fn make_abs(base: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.canonicalize().unwrap_or_else(|_| p.into())
    } else {
        let abs = [base, p].iter().collect::<PathBuf>();
        abs.canonicalize().unwrap_or(abs)
    }
}

/// Atttempt to determine the size of a given `File`, or return a default; useful
/// for initializing strings.
pub fn file_size(file: &File, default: usize) -> usize {
    file.metadata()
        .map(|m| m.len())
        .map_err(|_| ())
        .and_then(|len| len.try_into().map_err(|_| ()))
        .unwrap_or(default)
}

pub fn home_dir() -> io::Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))
}
