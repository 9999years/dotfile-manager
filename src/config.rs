use std::convert::{TryFrom, TryInto};
use std::fs::DirBuilder;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use dirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::link::Dotfile;

lazy_static! {
    static ref CONFIG_DIR_NAME: &'static Path = Path::new("dotfile-manager");
    static ref DEFAULT_DOTFILE_REPO_NAME: &'static Path = Path::new(".dotfiles");
    static ref CONFIG_DIR: io::Result<PathBuf> = {
        // TODO don't unwrap
        [&dirs::config_dir().unwrap(), *CONFIG_DIR_NAME]
            .iter()
            .collect::<PathBuf>()
            .canonicalize()
    };
    pub static ref CONFIG: Config = {
        Config::try_default().unwrap_or_default()
    };
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Default)]
pub struct Config {
    pub dotfile_repo: PathBuf,
}

impl Config {
    fn try_default() -> Option<Self> {
        Some(Config {
            dotfile_repo: [&dirs::home_dir()?, *DEFAULT_DOTFILE_REPO_NAME]
                .iter()
                .collect::<PathBuf>(),
        })
    }
}
