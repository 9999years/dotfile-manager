use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde_yaml;

use dotfile_manager::config;
use dotfile_manager::config::DotfilesWrapper;
use dotfile_manager::link::AbsDotfile;

fn main() {
    let dotfiles: DotfilesWrapper = serde_yaml::from_reader(BufReader::new(
        File::open(
            [&config::CONFIG.dotfile_repo, Path::new("dotfiles.yml")]
                .iter()
                .collect::<PathBuf>(),
        )
        .unwrap(),
    ))
    .unwrap();
    for df in dotfiles.dotfiles {
        let abs_df: AbsDotfile = df.try_into().unwrap();
        println!("{:?}", abs_df);
    }
}
