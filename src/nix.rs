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

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn nix_eval_file() {
        let res = dbg!(eval_file::<Vec<String>>(&PathBuf::from("test-data/string-list.nix")));
        // Allow tests to pass on systems without Nix installed.
        if res.is_err() {
            assert!(matches!(res, Err(NixEvalError::NoNix(_))));
        } else {
            assert_eq!(res.unwrap(), vec!["foo", "bar", "baz"]);
        }
    }

    #[test]
    fn nix_eval_missing_file() {
        let res = dbg!(eval_file::<Vec<String>>(&PathBuf::from("test-data/doesnt-exist.sldgkjaslj")));
        assert!(matches!(res, Err(NixEvalError::EvalFailed(_))));
        if let NixEvalError::EvalFailed(err) = res.unwrap_err() {
            assert!(err.starts_with("error: getting status of"));
            assert!(err.ends_with(": No such file or directory\n"));
        }
    }
}
