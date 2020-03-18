use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use dotfile_manager::config;
use dotfile_manager::config::AnyDotfile;
use dotfile_manager::link::AbsDotfile;

fn main() {
    // let dotfiles: Vec<AnyDotfile> = serde_dhall::from_str(
    // &File::open(
    // [&config::CONFIG.dotfile_repo, Path::new("dotfiles.dhall")]
    // .iter()
    // .collect::<PathBuf>(),
    // )
    // .unwrap()
    // .read_to_string(),
    // )
    // .unwrap();
    // // nix-instantiate --strict --xml --eval ~/.dotfiles/dotfiles.nix
    // for df in dotfiles {
    // let abs_df: AbsDotfile = df.try_into().unwrap();
    // println!("{:?}", abs_df);
    // }
}
