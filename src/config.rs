use std::convert::{TryFrom, TryInto};
use std::fs::DirBuilder;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix;
#[cfg(windows)]
use std::os::windows;

use dirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref CONFIG_DIR_NAME: &'static Path = Path::new("dotfile-manager");
    static ref CONFIG_DIR: io::Result<PathBuf> = {
        // TODO don't unwrap
        [&dirs::config_dir().unwrap(), *CONFIG_DIR_NAME]
            .iter()
            .collect::<PathBuf>()
            .canonicalize()
    };
    static ref CONFIG: Config = {
        Config {
            dotfile_repo: PathBuf::new() // TODO fill this in
        }
    };
}

/// A `Dotfile` struct fully resolved to canonical paths.
pub struct AbsDotfile {
    /// The dotfile's path in the dotfile repository.
    path: PathBuf,
    /// The dotfile's path in the user environment.
    dest: PathBuf,
}

impl TryFrom<Dotfile> for AbsDotfile {
    type Error = io::Error;

    fn try_from(d: Dotfile) -> io::Result<Self> {
        Ok(AbsDotfile {
            path: canonicalize(&CONFIG.dotfile_repo, &d.path)?,
            dest: canonicalize(
                &dirs::home_dir().ok_or_else(|| io::Error::new(
                    io::ErrorKind::NotFound,
                    "Home directory not found!",
                ))?,
                d.dest(),
            )?,
        })
    }
}

fn canonicalize(rel: &Path, p: &Path) -> io::Result<PathBuf> {
    if p.is_absolute() {
        p.canonicalize()
    } else {
        [rel, p].iter().collect::<PathBuf>().canonicalize()
    }
}

#[derive(Deserialize)]
pub struct Dotfile {
    /// The dotfile's path, relative to the dotfile repository.
    path: PathBuf,
    /// The dotfile's path, relative to your home directory. If left unspecified,
    /// this is the same as `path`.
    dest: Option<PathBuf>,
}

impl Dotfile {
    fn dest(&self) -> &Path {
        &self.dest.as_ref().unwrap_or(&self.path)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum AnyDotfile {
    // TODO: better names...
    Plain(PathBuf),
    Advanced(Dotfile),
}

#[derive(Deserialize)]
pub struct DotfilesWrapper {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub dotfiles: Vec<AnyDotfile>,
}

#[derive(Deserialize)]
pub struct Config {
    pub dotfile_repo: PathBuf,
}
