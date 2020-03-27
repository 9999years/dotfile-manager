use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dialoguer::{theme::ColorfulTheme, Confirmation};
use serde::Deserialize;
use symlink;

use crate::config::Config;
use crate::util::{home_dir, make_abs};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum SerdeDotfile {
    Path(PathBuf),
    Advanced(Dotfile),
}

impl From<SerdeDotfile> for Dotfile {
    fn from(d: SerdeDotfile) -> Self {
        match d {
            SerdeDotfile::Path(p) => p.into(),
            SerdeDotfile::Advanced(d) => d,
        }
    }
}

/// A `Dotfile` struct fully resolved to canonical paths.
#[derive(Debug)]
pub struct AbsDotfile {
    /// The dotfile's path in the dotfile repository.
    pub repo: PathBuf,
    /// The dotfile's path in the user environment.
    pub installed: PathBuf,
}

impl AbsDotfile {
    pub fn new(d: &Dotfile, cfg: &Config) -> io::Result<Self> {
        Ok(Self {
            repo: make_abs(&cfg.dotfile_repo, d.repo()),
            installed: make_abs(home_dir()?.as_path(), d.installed()),
        })
    }

    pub fn link(&self) -> io::Result<()> {
        if cfg!(unix) || self.repo.is_file() {
            symlink::symlink_file(&self.repo, &self.installed)
        } else {
            symlink::symlink_dir(&self.repo, &self.installed)
        }
    }

    fn should_overwrite(&self) -> io::Result<bool> {
        // TODO: More choices, not y/n
        // - verbose help
        // - diff the two files
        // - check if the files are the same (before this...?)
        Confirmation::with_theme(&ColorfulTheme::default())
            .with_text(&format!(
                "Overwrite {} with a link to {}?",
                self.installed.display(),
                self.repo.display()
            ))
            .interact()
    }

    pub fn link_interactive(&self) -> io::Result<()> {
        if self.installed.exists() {
            if self.should_overwrite()? {
                if self.installed.is_dir() {
                    fs::remove_dir(&self.installed)?;
                } else {
                    fs::remove_file(&self.installed)?;
                }
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Link source already exists",
                ));
            }
        }
        self.link()
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Dotfile {
    /// The dotfile's path, relative to the dotfile repository.
    pub repo: PathBuf,
    /// The dotfile's path, relative to your home directory. If left unspecified,
    /// this is the same as `repo`.
    pub installed: Option<PathBuf>,
}

impl From<PathBuf> for Dotfile {
    fn from(p: PathBuf) -> Self {
        Self {
            repo: p,
            installed: None,
        }
    }
}

impl Dotfile {
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    pub fn installed(&self) -> &Path {
        &self.installed.as_ref().unwrap_or(&self.repo)
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn dotfile_installed() {
        assert_eq!(
            Dotfile {
                repo: "foo".into(),
                installed: Some("bar".into()),
            }
            .installed(),
            Path::new("bar"),
        );

        assert_eq!(
            Dotfile {
                repo: "baz".into(),
                installed: None,
            }
            .installed(),
            Path::new("baz"),
        );

        assert_eq!(
            Dotfile {
                repo: "baz".into(),
                installed: None,
            }
            .repo(),
            Path::new("baz"),
        );
    }

    #[test]
    fn dotfile_from_path() {
        assert_eq!(
            Dotfile::from(PathBuf::from("xxx")),
            Dotfile {
                repo: "xxx".into(),
                installed: None,
            }
        );
    }
}
