use crate::errors::*;
use scraper::{ElementRef, Html, Selector};
use std::fs::{self, File, OpenOptions};
use std::io::{copy, Cursor, Read};
use tempfile::Builder;

pub async fn download(url: &str) -> Result<File> {
    let tmp_dir = Builder::new().prefix("smw").tempdir()?;
    let response = reqwest::get(url).await?;

    let mut dest = {
        let fname = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or("patch.zip");

        let fname = tmp_dir.path().join(fname);
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

pub fn get_patch_from_zip(zip_file: &File, patch_file: &mut File) -> Result<Option<String>> {
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();
    println!("{} items in downloaded archive.", archive.len());
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        println!("|-> {:?}", outpath);
        match outpath.extension() {
            Some(extension) if extension == "bps" => {
                copy(&mut file, patch_file)?;
                let filename = outpath.file_stem().unwrap();
                return Ok(Some(String::from(filename.to_str().unwrap())));
            }
            _ => continue,
        }
    }
    Ok(None)
}

fn remove_whitespace(string: &String) -> String {
    let chars: Vec<&str> = string.split_whitespace().collect();
    chars.join("")
}

#[derive(Debug)]
pub struct RomHackDetails {
    name: String,
    added_timestamp: String,
    demo: String,
    featured: String,
    length: String,
    game_type: String,
    authors: String,
    rating: String,
    size: String,
    download_url: String,
    downloads: String,
}

impl RomHackDetails {
    pub fn ordered_fields(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.added_timestamp.clone(),
            self.demo.clone(),
            self.featured.clone(),
            self.length.clone(),
            self.game_type.clone(),
            self.authors.clone(),
            self.rating.clone(),
            self.size.clone(),
            self.downloads.clone(),
        ]
    }
}

pub fn search_smwcentral(search_term: &String) -> Result<Option<Vec<RomHackDetails>>> {
    let url = format!(
        "https://www.smwcentral.net/?p=section&s=smwhacks&f%5Bname%5D={}",
        search_term
    );
    let response = reqwest::blocking::get(url)?;
    let mut html_content = response.text()?;

    let document = Html::parse_document(html_content.as_str());
    let selector = Selector::parse("#list_content tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let time_selector = Selector::parse("time").unwrap();
    let span_selector = Selector::parse("span").unwrap();

    let mut results = vec![];
    for element in document.select(&selector).skip(1) {
        let columns: Vec<ElementRef> = element.select(&td_selector).collect();
        let name = columns[0].select(&a_selector).next().unwrap().inner_html();
        let added_timestamp = columns[0]
            .select(&time_selector)
            .next()
            .unwrap()
            .inner_html()
            .split_once(" ")
            .unwrap()
            .0
            .to_string();

        let demo = columns[1].inner_html().trim().to_string();
        let featured = columns[2].inner_html().trim().to_string();
        let length = columns[3].inner_html().trim().to_string();
        let game_type = columns[4].inner_html().trim().to_string();

        let authors = columns[5].select(&a_selector).next().unwrap().inner_html();
        let rating = columns[6].inner_html().trim().to_string();
        let size = columns[7].inner_html().trim().replace("&nbsp;", " ");
        let download_url = columns[8]
            .select(&a_selector)
            .next()
            .unwrap()
            .value()
            .attr("href")
            .unwrap()
            .replace("//", "https://");
        let downloads = columns[8]
            .select(&span_selector)
            .next()
            .unwrap()
            .inner_html();

        results.push(RomHackDetails {
            name,
            added_timestamp,
            demo,
            featured,
            length,
            game_type,
            authors,
            rating,
            size,
            download_url,
            downloads,
        })
    }
    if results.len() == 0 {
        return Ok(None);
    }
    Ok(Some(results))
}

async fn patch_rom(download_url: &String) -> Result<()> {
    let tmp_dir = Builder::new().prefix("smw").tempdir()?;
    let fname = tmp_dir.path().join("patch.bps");
    let mut patch_file = OpenOptions::new()
        .write(true)
        .create(true)
        .read(true)
        .open(&fname)?;

    let zip_file = download(download_url.as_str())
        .await
        .expect("Can download target");
    let output_filename = get_patch_from_zip(&zip_file, &mut patch_file)?.unwrap();

    let base_file = fs::read("base.smc")?;
    // TODO: Figure out the proper header stripping logic
    let (_, base_file) = base_file.split_at(512);

    let patch = fs::read(&fname)?;
    // println!("romlen&0x7FFF==512? {}", base_file.len() & 0x7FFF);

    let output = flips::BpsPatch::new(patch)
        .apply(base_file)
        .expect("Failed to apply patch");
    fs::write(format!("{}.smc", output_filename), output)?;
    Ok(())
}
