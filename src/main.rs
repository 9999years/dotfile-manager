use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use dotfile_manager::config;
use dotfile_manager::config::AnyDotfile;
use dotfile_manager::config::CONFIG;
use dotfile_manager::link::AbsDotfile;

fn main() {
    println!("{:?}", CONFIG.dotfiles());
    // for df in dotfiles {
    // let abs_df: AbsDotfile = df.try_into().unwrap();
    // println!("{:?}", abs_df);
    // }
}
