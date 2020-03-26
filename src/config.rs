use std::convert::TryFrom;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use dirs;
use lazy_static::lazy_static;
use serde::Deserialize;
use thiserror::Error;

use crate::dotfile::{Dotfile, SerdeDotfile};
use crate::nix;
use crate::nix::NixEvalError;
use crate::util::file_to_string;

lazy_static! {
    static ref CONFIG_DIR_NAME: &'static Path = Path::new("dotfile-manager");
    static ref DEFAULT_DOTFILE_REPO_NAME: &'static Path = Path::new(".dotfiles");
    static ref CONFIG_FILE_NAME: &'static Path = Path::new("dotfile-manager.toml");
    pub static ref CONFIG: Config = { Config::try_default().unwrap_or_default() };
}

/// Configuration directory, e.g. ~/.config/dotfile-manager on Linux.
fn config_dir() -> io::Result<PathBuf> {
    Ok([
        &dirs::config_dir()
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Config directory not found."))?,
        *CONFIG_DIR_NAME,
    ]
    .iter()
    .collect::<PathBuf>())
}

/// Configuration file path, e.g. ~/.config/dotfile-manager/dotfile-manager.toml
/// on Linux.
pub fn config_file() -> io::Result<PathBuf> {
    Ok([&config_dir()?, *CONFIG_FILE_NAME]
        .iter()
        .collect::<PathBuf>())
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

impl From<Vec<SerdeDotfile>> for SerdeDotfileList {
    fn from(v: Vec<SerdeDotfile>) -> Self {
        Self {
            schema: None,
            dotfiles: v,
        }
    }
}

impl SerdeDotfileList {
    fn dotfiles(&self) -> Vec<Dotfile> {
        self.dotfiles.iter().cloned().map(Into::into).collect()
    }
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

#[derive(Error, Debug)]
pub enum ConfigReadError {
    #[error("dirs crate failed to find home directory")]
    NoHome,

    #[error("config file {0} doesn't exist")]
    NotFound(PathBuf),

    #[error("failed to open/read config file")]
    File(#[from] io::Error),

    #[error("failed to parse config file as TOML / incorrect schema")]
    SerdeTOML(#[from] toml::de::Error),
}

/// The configuration data for the dotfile-manager program.
#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Config {
    /// The directory where dotfiles are stored; if not absolute, interpreted as
    /// relative to the user's home directory.
    pub dotfile_repo: PathBuf,
    /// Basename of the dotfiles list file; default `dotfiles`. Relative to
    /// `dotfile_repo`.
    pub dotfiles_basename: PathBuf,
}

impl TryFrom<&Path> for Config {
    type Error = ConfigReadError;

    fn try_from(p: &Path) -> Result<Self, <Self as TryFrom<&Path>>::Error> {
        if !p.exists() {
            return Err(ConfigReadError::NotFound(p.to_path_buf()));
        }
        Ok(toml::from_str(&file_to_string(&mut File::open(p)?)?)?)
    }
}

impl Config {
    pub fn try_default() -> Result<Self, ConfigReadError> {
        Ok(Config {
            dotfile_repo: [
                &dirs::home_dir().ok_or(ConfigReadError::NoHome)?,
                *DEFAULT_DOTFILE_REPO_NAME,
            ]
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
                Ok(toml::from_str::<SerdeDotfileList>(&file_to_string(&mut file)?)?.dotfiles())
            }
            DotfileListFiletype::Nix => {
                let list: SerdeDotfileList = nix::eval_file::<Vec<SerdeDotfile>>(&path)
                    .map_err(|err| match err {
                        // Don't use multiple json serde error types
                        NixEvalError::SerdeJSON(err) => DotfilesReadError::SerdeJSON(err),
                        err => DotfilesReadError::NixEval(err),
                    })?
                    .into();
                Ok(list.dotfiles())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_config_file() {
        let cfg = config_file().unwrap();
        assert!(cfg.ends_with("dotfile-manager/dotfile-manager.toml"));
    }

    #[test]
    fn serde_dotfile_list() {
        let dotfiles: SerdeDotfileList = serde_json::from_str(
            r#"
                {
                    "$schema": "...",
                    "dotfiles": [
                        "ok",
                        {
                            "repo": "repo-path",
                            "installed": "installed-path"
                        },
                        "great"
                    ]
                }
                "#,
        )
        .unwrap();

        assert_eq!(
            dotfiles.dotfiles,
            vec![
                SerdeDotfile::Path("ok".into()),
                SerdeDotfile::Advanced(Dotfile {
                    repo: "repo-path".into(),
                    installed: Some("installed-path".into()),
                }),
                SerdeDotfile::Path("great".into()),
            ]
        );

        assert_eq!(
            dotfiles.dotfiles(),
            vec![
                Dotfile {
                    repo: "ok".into(),
                    installed: None
                },
                Dotfile {
                    repo: "repo-path".into(),
                    installed: Some("installed-path".into()),
                },
                Dotfile {
                    repo: "great".into(),
                    installed: None
                },
            ]
        );
    }
}
