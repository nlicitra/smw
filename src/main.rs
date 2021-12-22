#![allow(unused)]
use error_chain::error_chain;
use std::fs::{File, OpenOptions};
use std::io::{copy, Cursor};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use tempfile::{Builder, NamedTempFile};

error_chain! {
     foreign_links {
         Io(std::io::Error);
         HttpRequest(reqwest::Error);
     }
}

#[tokio::main]
async fn main() -> Result<()> {
    let tmp_dir = Builder::new().prefix("smw").tempdir()?;
    let fname = tmp_dir.path().join("patch.bps");
    // let fname = PathBuf::from("/tmp/patch.bps");
    let patch_path = String::from(fname.as_os_str().to_str().unwrap());
    let mut patch_file = OpenOptions::new()
        .write(true)
        .create(true)
        .read(true)
        .open(fname)?;

    let target = "https://dl.smwcentral.net/27094/";
    let zip_file = download(target).await?;
    get_patch_from_zip(&zip_file, &mut patch_file)?.unwrap();

    println!("Patch path: {}", patch_path);
    let output = Command::new("./flips")
        .arg("--apply")
        .arg(patch_path)
        .arg("base.smc")
        .arg("output.smc")
        .output()
        .expect("fuk");
    println!("{:?}", str::from_utf8(&output.stdout).unwrap());
    Ok(())
}

async fn download(url: &str) -> Result<File> {
    let tmp_dir = Builder::new().prefix("smw").tempdir()?;
    let response = reqwest::get(url).await?;

    let mut dest = {
        let fname = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or("patch.zip");

        println!("file to download: '{}'", fname);
        let fname = tmp_dir.path().join(fname);
        println!("will be located under: '{:?}'", fname);
        OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .open(fname)?
    };
    let mut content = Cursor::new(response.bytes().await?);
    copy(&mut content, &mut dest)?;
    Ok(dest)
}

fn get_patch_from_zip(zip_file: &File, patch_file: &mut File) -> Result<Option<()>> {
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();
    println!("{} items in archive.", archive.len());
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        println!("|-> {:?}", outpath);
        match outpath.as_path().extension() {
            Some(extension) => {
                if extension == "bps" {
                    copy(&mut file, patch_file)?;
                    return Ok(Some(()));
                }
            }
            None => continue,
        }
    }
    Ok(None)
}
