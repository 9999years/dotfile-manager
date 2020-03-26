use std::convert::TryFrom;
use std::io;
use std::path::PathBuf;

use thiserror::Error;

use dotfile_manager::config;
use dotfile_manager::config::{Config, ConfigReadError, DotfilesReadError};
use dotfile_manager::dotfile::AbsDotfile;

#[derive(Debug, Error)]
enum MainError {
    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("{0}")]
    ConfigRead(#[from] ConfigReadError),

    #[error("{0}")]
    DotfilesRead(#[from] DotfilesReadError),
}

fn main() {
    let main_ret = main_inner();
    if let Err(err) = main_ret {
        println!("Error: {}", err);
        println!("{:?}", err)
    }
}

fn main_inner() -> Result<(), MainError> {
    let cfg =
        Config::try_from(dbg!(config::config_file())?.as_path()).or_else(|err| match err {
            ConfigReadError::NotFound(_) => Config::try_default(),
            err => Err(err),
        })?;
    println!("Configuration: {:?}", cfg);
    let abs_dotfiles = cfg
        .dotfiles()?
        .iter()
        .map(|d| AbsDotfile::new(d, &cfg))
        .collect::<Result<Vec<_>, _>>()?;
    println!("Dotfiles: {:?}", abs_dotfiles);
    Ok(())
}
