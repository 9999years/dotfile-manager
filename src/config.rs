use std::convert::TryFrom;
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

use crate::link::{AbsDotfile, Dotfile};
use crate::nix;
use crate::nix::NixEvalError;
use crate::util::{file_size, home_dir, make_abs};

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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum SerdeDotfile {
    // TODO: better names...
    Path(PathBuf),
    Advanced(Dotfile),
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

impl From<SerdeDotfile> for Dotfile {
    fn from(d: SerdeDotfile) -> Self {
        match d {
            SerdeDotfile::Path(p) => p.into(),
            SerdeDotfile::Advanced(d) => d,
        }
    }
}

/// A wrapper struct for use when deserializing a dotfile list.
#[derive(Deserialize)]
struct SerdeDotfileList {
    /// Allow a `$schema` identifier for formats/programs that support it (mostly
    /// JSON).
    #[allow(dead_code)]
    #[serde(rename = "$schema")]
    schema: Option<String>,
    dotfiles: Vec<SerdeDotfile>,
}

impl SerdeDotfileList {
    fn dotfiles(&self) -> Vec<Dotfile> {
        self.dotfiles.iter().cloned().map(Into::into).collect()
    }
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
enum DotfileListFiletype {
    Nix,
    JSON,
    TOML,
    YAML,
}

impl DotfileListFiletype {
    fn extensions(self) -> Vec<PathBuf> {
        match self {
            DotfileListFiletype::Nix => vec!["nix".into()],
            DotfileListFiletype::JSON => vec!["json".into()],
            DotfileListFiletype::TOML => vec!["toml".into()],
            DotfileListFiletype::YAML => vec!["yaml".into(), "yml".into()],
        }
    }
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

    fn dotfiles_paths(&self) -> Vec<(PathBuf, DotfileListFiletype)> {
        vec![
            DotfileListFiletype::Nix,
            DotfileListFiletype::JSON,
            DotfileListFiletype::TOML,
            DotfileListFiletype::YAML,
        ]
        .iter()
        .map(|filetype| {
            filetype
                .extensions()
                .iter()
                .map(|ext| self.dotfiles_filename(ext))
                .map(|filename| (filename, *filetype))
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect()
    }

    fn dotfiles_path(&self) -> Result<(PathBuf, File, DotfileListFiletype), DotfilesReadError> {
        self.dotfiles_paths()
            .iter()
            .find(|(path, _)| path.exists())
            .map(Result::Ok)
            .unwrap_or(Err(DotfilesReadError::NoneFound))
            .and_then(|(path, filetype)| {
                Ok(File::open(path).map(|file| (path.clone(), file, *filetype))?)
            })
    }

    pub fn dotfiles(&self) -> Result<Vec<Dotfile>, DotfilesReadError> {
        let (path, mut file, filetype) = self.dotfiles_path()?;
        match filetype {
            DotfileListFiletype::JSON => Ok(serde_json::from_reader::<_, SerdeDotfileList>(
                BufReader::new(file),
            )?
            .dotfiles()),
            DotfileListFiletype::YAML => Ok(serde_yaml::from_reader::<_, SerdeDotfileList>(
                BufReader::new(file),
            )?
            .dotfiles()),
            DotfileListFiletype::TOML => {
                let mut s = String::with_capacity(file_size(&file, 2048usize));
                file.read_to_string(&mut s)?;
                Ok(toml::from_str::<SerdeDotfileList>(&s)?.dotfiles())
            }
            DotfileListFiletype::Nix => {
                nix::eval_file(&path).map_err(|err| match err {
                    // Don't use multiple json serde error types
                    NixEvalError::SerdeJSON(err) => DotfilesReadError::SerdeJSON(err),
                    err => DotfilesReadError::NixEval(err),
                })
            }
        }
    }

    pub fn canonicalize_dotfile(d: Dotfile) -> io::Result<AbsDotfile> {
        Ok(AbsDotfile {
            repo: make_abs(&CONFIG.dotfile_repo, d.repo()),
            installed: make_abs(home_dir()?.as_path(), d.installed()),
        })
    }
}
