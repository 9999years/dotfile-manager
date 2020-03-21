use std::convert::{TryFrom, TryInto};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dialoguer::{theme::ColorfulTheme, Confirmation};
use serde::Deserialize;
use symlink;

use crate::config::{AnyDotfile, CONFIG};

/// A `Dotfile` struct fully resolved to canonical paths.
#[derive(Debug)]
pub struct AbsDotfile {
    /// The dotfile's path in the dotfile repository.
    pub repo: PathBuf,
    /// The dotfile's path in the user environment.
    pub installed: PathBuf,
}

impl AbsDotfile {
    pub fn link(&self) -> io::Result<()> {
        if cfg!(unix) || self.repo.is_file() {
            symlink::symlink_file(&self.repo, &self.installed)
        } else {
            symlink::symlink_dir(&self.repo, &self.installed)
        }
    }

    fn should_overwrite(&self) -> io::Result<bool> {
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

fn home_dir() -> io::Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Home directory not found")
        })
}

impl TryFrom<Dotfile> for AbsDotfile {
    type Error = io::Error;

    fn try_from(d: Dotfile) -> io::Result<Self> {
        Ok(AbsDotfile {
            repo: make_abs(&CONFIG.dotfile_repo, &d.repo),
            installed: make_abs(
                home_dir()?.as_path(),
                d.installed(),
            ),
        })
    }
}

impl TryFrom<AnyDotfile> for AbsDotfile {
    type Error = io::Error;

    fn try_from(df: AnyDotfile) -> io::Result<Self> {
        match df {
            AnyDotfile::Plain(p) => Dotfile::from(p),
            AnyDotfile::Advanced(d) => d,
        }
        .try_into()
    }
}

fn make_abs(base: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.canonicalize().unwrap_or_else(|_| p.into())
    } else {
        let abs = [base, p].iter().collect::<PathBuf>();
        abs.canonicalize().unwrap_or(abs)
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Dotfile {
    /// The dotfile's path, relative to the dotfile repository.
    repo: PathBuf,
    /// The dotfile's path, relative to your home directory. If left unspecified,
    /// this is the same as `path`.
    installed: Option<PathBuf>,
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
    fn installed(&self) -> &Path {
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
            }.installed(),
            &PathBuf::from("bar"),
        );

        assert_eq!(
            Dotfile {
                repo: "baz".into(),
                installed: None,
            }.installed(),
            &PathBuf::from("baz"),
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

    #[test]
    fn abs_dotfile_try_from() {
        // assert_eq!(
        //     AbsDotfile::try_from(AnyDotfile::Plain("xxx".into())),
        // );
    }
}