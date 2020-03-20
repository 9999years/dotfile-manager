use std::io;
use std::path::Path;
use std::process::Command;

use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NixEvalError {
    #[error("nix-instantiate binary not found: {0}")]
    NoNix(io::Error),
    #[error("executing nix-instantiate failed: {0}")]
    CommandFailed(#[from] io::Error),
    #[error("Nix evaluation failed: {0:?}")]
    EvalFailed(String),
    #[error("{0}")]
    SerdeJSON(#[from] serde_json::Error),
}

pub fn eval_file<T: DeserializeOwned>(path: &Path) -> Result<T, NixEvalError> {
    let output_res = Command::new("nix-instantiate")
        .args(&["--strict", "--json", "--eval"])
        .arg(path)
        .output();
    match output_res {
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => Err(NixEvalError::NoNix(err)),
            _ => Err(err.into()),
        },
        Ok(output) => {
            if !output.stderr.is_empty() {
                Err(NixEvalError::EvalFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            } else {
                Ok(serde_json::from_reader(&output.stdout[..])?)
            }
        }
    }
}
