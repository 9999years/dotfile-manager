use std::convert::TryInto;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Read;
use std::path::{Path, PathBuf};

use dirs;
use lazy_static::lazy_static;
use serde::Deserialize;
use thiserror::Error;

use crate::link::Dotfile;
use crate::nix;
use crate::nix::NixEvalError;

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

/// Atttempt to determine the size of a given `File`, or return a default; useful
/// for initializing strings.
fn file_size(file: &File, default: usize) -> usize {
    file.metadata()
        .map(|m| m.len())
        .map_err(|_| ())
        .and_then(|len| len.try_into().map_err(|_| ()))
        .unwrap_or(default)
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

/// The configuration data for the dotfile-manager program.
#[derive(Deserialize, Default)]
pub struct Config {
    /// The directory where dotfiles are stored; if not absolute, interpreted as
    /// relative to the user's home directory.
    pub dotfile_repo: PathBuf,
    /// Basename of the dotfiles list file; default `dotfiles`. Relative to
    /// `dotfile_repo`.
    pub dotfiles_basename: PathBuf,
}

/// An error when reading/deserializing a dotfiles list file.
#[derive(Error, Debug)]
pub enum DotfilesReadError {
    /// Could not find any dotfiles list files.
    #[error("no dotfiles lists found")]
    NoneFound,

    /// Error while opening a dotfiles list file.
    #[error("couldn't open dotfiles")]
    File(#[from] io::Error),

    /// Deserialization error (JSON); includes deserialization from evaluated Nix
    /// expression language output.
    #[error("failed to parse as JSON / incorrect schema")]
    SerdeJSON(#[from] serde_json::Error),

    /// Deserialization error (YAML).
    #[error("failed to parse as YAML / incorrect schema")]
    SerdeYAML(#[from] serde_yaml::Error),

    /// Deserialization error (TOML).
    #[error("failed to parse as TOML / incorrect schema")]
    SerdeTOML(#[from] toml::de::Error),

    /// Evaluation error (Nix expression language).
    #[error("{0}")]
    NixEval(#[from] NixEvalError),
}

/// The file format of a dotfiles list file.
#[derive(Copy, Clone, Debug)]
enum DotfilesFiletype {
    Nix,
    JSON,
    TOML,
    YAML,
}

impl Config {
    pub fn try_default() -> Option<Self> {
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

    fn toml_dotfiles_path(&self) -> PathBuf {
        self.dotfiles_filename("toml")
    }

    fn yaml_dotfiles_paths(&self) -> Vec<PathBuf> {
        vec![
            self.dotfiles_filename("yml"),
            self.dotfiles_filename("yaml"),
        ]
    }

    fn dotfiles_paths(&self) -> Vec<(PathBuf, DotfilesFiletype)> {
        let mut res = vec![
            (self.nix_dotfiles_path(), DotfilesFiletype::Nix),
            (self.json_dotfiles_path(), DotfilesFiletype::JSON),
            (self.toml_dotfiles_path(), DotfilesFiletype::TOML),
        ];
        self.yaml_dotfiles_paths()
            .iter()
            .map(|path| (path.to_path_buf(), DotfilesFiletype::YAML))
            .for_each(|s| res.push(s));
        res
    }

    fn dotfiles_path(&self) -> Result<(PathBuf, File, DotfilesFiletype), DotfilesReadError> {
        self.dotfiles_paths()
            .iter()
            .find(|(path, _)| path.exists())
            .map(Result::Ok)
            .unwrap_or(Err(DotfilesReadError::NoneFound))
            .and_then(|(path, filetype)| {
                Ok(File::open(path).map(|file| (path.clone(), file, *filetype))?)
            })
    }

    pub fn dotfiles(&self) -> Result<Dotfiles, DotfilesReadError> {
        let (path, mut file, filetype) = self.dotfiles_path()?;
        match filetype {
            DotfilesFiletype::JSON => {
                Ok(serde_json::from_reader::<_, DotfilesWrapper>(BufReader::new(file))?.dotfiles)
            }
            DotfilesFiletype::YAML => {
                Ok(serde_yaml::from_reader::<_, DotfilesWrapper>(BufReader::new(file))?.dotfiles)
            }
            DotfilesFiletype::TOML => {
                let mut s = String::with_capacity(file_size(&file, 2048usize));
                file.read_to_string(&mut s)?;
                Ok(toml::from_str::<DotfilesWrapper>(&s)?.dotfiles)
            }
            DotfilesFiletype::Nix => {
                nix::eval_file(&path).map_err(|err| match err {
                    // Don't use multiple json serde error types
                    NixEvalError::SerdeJSON(err) => DotfilesReadError::SerdeJSON(err),
                    err => DotfilesReadError::NixEval(err),
                })
            }
        }
    }
}
