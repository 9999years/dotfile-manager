use std::convert::{TryFrom, TryInto};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dialoguer::{theme::ColorfulTheme, Confirmation};
use serde::Deserialize;
use symlink;

use crate::config::{SerdeDotfile, CONFIG};
use crate::util::{home_dir, make_abs};

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

impl TryFrom<Dotfile> for AbsDotfile {
    type Error = io::Error;

    fn try_from(d: Dotfile) -> io::Result<Self> {
        Ok(AbsDotfile {
            repo: make_abs(&CONFIG.dotfile_repo, &d.repo),
            installed: make_abs(home_dir()?.as_path(), d.installed()),
        })
    }
}

impl TryFrom<SerdeDotfile> for AbsDotfile {
    type Error = io::Error;

    fn try_from(df: SerdeDotfile) -> io::Result<Self> {
        match df {
            SerdeDotfile::Path(p) => Dotfile::from(p),
            SerdeDotfile::Advanced(d) => d,
        }
        .try_into()
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Dotfile {
    /// The dotfile's path, relative to the dotfile repository.
    repo: PathBuf,
    /// The dotfile's path, relative to your home directory. If left unspecified,
    /// this is the same as `repo`.
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

impl From<SerdeDotfile> for Dotfile {
    fn from(d: SerdeDotfile) -> Self {
        match d {
            SerdeDotfile::Path(p) => p.into(),
            SerdeDotfile::Advanced(d) => d,
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
            &PathBuf::from("bar"),
        );

        assert_eq!(
            Dotfile {
                repo: "baz".into(),
                installed: None,
            }
            .installed(),
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
        //     AbsDotfile::try_from(SerdeDotfile::Path("xxx".into())),
        // );
    }
}
