use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use dirs;
use lazy_static::lazy_static;
use serde::Deserialize;
use thiserror::Error;

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

pub type Dotfiles = Vec<AnyDotfile>;

#[derive(Deserialize)]
pub struct DotfilesWrapper {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub dotfiles: Dotfiles,
}

#[derive(Deserialize, Default)]
pub struct Config {
    pub dotfile_repo: PathBuf,
    pub dotfiles_basename: PathBuf,
}

#[derive(Error, Debug)]
pub enum DotfilesReadError {
    #[error("couldn't open dotfiles")]
    File(#[from] io::Error),
    #[error("failed to parse as JSON / incorrect schema")]
    SerdeJson(#[from] serde_json::Error),
    #[error("failed to parse as YAML / incorrect schema")]
    SerdeYaml(#[from] serde_yaml::Error),
}

impl Config {
    fn try_default() -> Option<Self> {
        Some(Config {
            dotfile_repo: [&dirs::home_dir()?, *DEFAULT_DOTFILE_REPO_NAME]
                .iter()
                .collect::<PathBuf>(),
            dotfiles_basename: PathBuf::from("dotfiles"),
        })
    }

    fn dotfiles_basename_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let mut dotfiles_filename = self.dotfiles_basename.clone();
        dotfiles_filename.set_extension(extension);
        dotfiles_filename
    }

    fn dotfiles_filename<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        [
            &self.dotfile_repo,
            &self.dotfiles_basename_extension(extension),
        ]
        .iter()
        .collect::<PathBuf>()
    }

    fn nix_dotfiles_path(&self) -> PathBuf {
        self.dotfiles_filename("nix")
    }

    fn json_dotfiles_path(&self) -> PathBuf {
        self.dotfiles_filename("json")
    }

    fn json_dotfiles(&self) -> Result<Dotfiles, DotfilesReadError> {
        Ok(serde_json::from_reader(BufReader::new(&File::open(
            self.json_dotfiles_path(),
        )?))?)
    }

    fn toml_dotfiles_path(&self) -> PathBuf {
        self.dotfiles_filename("toml")
    }

    fn yaml_dotfiles_paths(&self) -> Vec<PathBuf> {
        vec![
            self.dotfiles_filename("yml"),
            self.dotfiles_filename("yaml"),
        ]
    }

    fn yaml_dotfiles(&self) -> Result<Dotfiles, DotfilesReadError> {
        let paths = self.yaml_dotfiles_paths();
        let mut last_err: Result<_, DotfilesReadError> = Ok(());
        for path in paths {
            match File::open(path) {
                Ok(file) => match serde_yaml::from_reader(BufReader::new(file)) {
                    Ok(dotfiles) => return Ok(dotfiles),
                    Err(err) => last_err = Err(err.into()),
                },
                Err(err) => last_err = Err(err.into()),
            }
        }
        // Note: unwrapping is OK because we know that
        // self.yaml_dotfiles_paths() never returns an empty vector; therefore,
        // either last_err will have an Err value or we'll have returned an Ok
        // already.
        Err(last_err.unwrap_err())
    }
}
