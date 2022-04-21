#![allow(unused)]
mod errors;
mod ui;
mod utils;

use errors::*;
use std::env;
use std::fs::{self, OpenOptions};
use tempfile::Builder;
use utils::{download, get_patch_from_zip, search_smwcentral};

extern crate flips;

// #[tokio::main]
fn main() -> Result<()> {
    // let args: Vec<String> = env::args().collect();
    // let patch_id = args.get(1).expect("Patch ID is provided");
    // // patch_rom(patch_id).await?;
    ui::run();
    // let term = String::from("akogare");
    // let results = search_smwcentral(&term).await?.unwrap();
    // println!("{:?}", results);

    Ok(())
}
