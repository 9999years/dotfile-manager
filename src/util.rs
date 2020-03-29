use std::convert::TryInto;
use std::fs;
use std::fs::{File, Metadata};
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn make_abs(base: &Path, p: &Path) -> PathBuf {
    let abs = base.join(p);
    abs.canonicalize().unwrap_or(abs)
}

pub trait SupportsMetadata {
    /// Get the metadata for this object, if possible.
    fn metadata(&self) -> io::Result<Metadata>;
}

impl SupportsMetadata for &File {
    fn metadata(&self) -> io::Result<Metadata> {
        File::metadata(self)
    }
}

impl SupportsMetadata for &Path {
    fn metadata(&self) -> io::Result<Metadata> {
        fs::metadata(self)
    }
}

/// Attempt to determine the size of a given `File`, or return a default; useful
/// for initializing strings.
pub fn file_size(file: impl SupportsMetadata, default: usize) -> usize {
    file.metadata()
        .ok()
        .and_then(|m| m.len().try_into().ok())
        .unwrap_or(default)
}

pub fn home_dir() -> io::Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))
}

pub fn file_to_string(file: &mut File) -> io::Result<String> {
    let mut s = String::with_capacity(file_size(&*file, 2048usize));
    file.read_to_string(&mut s)?;
    Ok(s)
}

#[cfg(test)]
mod test {
    use std::env;

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_make_abs() {
        let base = Path::new("/usr/lib");
        assert!(base.is_absolute());
        assert_eq!(make_abs(base, Path::new("xxx")), Path::new("/usr/lib/xxx"));
        assert_eq!(
            make_abs(base, Path::new("./xxx")),
            Path::new("/usr/lib/xxx")
        );
        assert_eq!(
            make_abs(base, Path::new("./qt/whatever")),
            Path::new("/usr/lib/qt/whatever")
        );

        // we can't normalize appropriately if the path doesn't exist, because
        // there may be symlink nonsense.
        assert_eq!(
            make_abs(Path::new("/foo/bar"), Path::new("../baz")),
            Path::new("/foo/bar/../baz")
        );

        let cwd = &env::current_dir().unwrap();
        assert_eq!(
            make_abs(cwd, Path::new("test-data/../src/facts.rs")),
            cwd.join("src/facts.rs")
        );

        // absolute paths override the base
        assert_eq!(make_abs(cwd, Path::new("/tmp")), Path::new("/tmp"));
    }

    #[test]
    fn test_file_size() {
        // Empty file that exists
        assert_eq!(
            file_size(&File::open("test-data/empty-file.txt").unwrap(), 100),
            0
        );

        // Path that doesn't exist
        assert_eq!(
            file_size(Path::new("test-data/nonexistent-file.txt"), 413),
            413
        );

        // File that exists
        assert_eq!(
            file_size(Path::new("test-data/fixed-size-file.txt"), 413),
            112
        );
    }

    #[test]
    fn test_home_dir() {
        let dir = home_dir().unwrap();
        assert!(dir.is_absolute());
    }

    #[test]
    fn test_file_to_string() {
        assert_eq!(
            file_to_string(&mut File::open("test-data/fixed-size-file.txt").unwrap()).unwrap(),
            indoc!(
                r#"This file has a fixed size; if you modify its size, tests will break. In
                particular, tests for util::file_size.
                "#
            )
        );
    }
}
