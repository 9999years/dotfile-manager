use std::path::Path;
use std::io;
use std::process::Command;

use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NixEvalError {
    #[error("executing nix-instantiate failed: {0}")]
    CommandFailed(#[from] io::Error),
    #[error("Nix evaluation failed: {0:?}")]
    EvalFailed(String),
    #[error("{0}")]
    SerdeJSON(#[from] serde_json::Error),
}

pub fn eval_file<T: DeserializeOwned>(path: &Path) -> Result<T, NixEvalError> {
    let output = Command::new("nix-instantiate")
        .args(&["--strict", "--xml", "--eval"])
        .arg(path)
        .output()?;
    if !output.stderr.is_empty() {
        Err(NixEvalError::EvalFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    } else {
        serde_json::from_reader(&output.stdout[..]).map_err(Into::into)
    }
}
