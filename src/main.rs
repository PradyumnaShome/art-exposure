use glob::glob;
use reqwest;
use reqwest::blocking::Response;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

extern crate url;

extern crate cocoa;
extern crate image;

use image::{ImageBuffer, Rgba};

mod image_processing;
mod api_client;

#[derive(Deserialize)]
pub struct SearchResult {
    #[serde(rename = "objectIDs")]
    object_ids: Vec<u32>,
}

#[derive(Deserialize, Clone)]
pub struct ImageInfo {
    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "artistDisplayName")]
    artist: String,
    #[serde(rename = "primaryImage")]
    url: String,
}

fn make_filename_safe(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' => {
                result.push(c);
            }
            _ => {
                result.push('_');
            }
        }
    }
    result
}

fn initialize_app_data(app_data_dir: &Path) {
    if !app_data_dir.exists() {
        if let Err(err) = fs::create_dir(&app_data_dir) {
            panic!("Error creating directory: {:?}", err);
        }
    }

    // Define the file extensions to remove
    let file_extensions = ["jpg", "jpeg", "png"];

    // Iterate through the specified extensions and remove matching files
    for extension in &file_extensions {
        let pattern = format!("{}/*.{}", app_data_dir.to_str().unwrap(), extension);

        // Use the glob crate to match files
        if let Ok(entries) = glob(&pattern) {
            for entry in entries {
                if let Ok(path) = entry {
                    if path.is_file() {
                        if let Err(err) = fs::remove_file(&path) {
                            eprintln!("Error removing file: {:?}", err);
                        }
                    }
                }
            }
        } else {
            eprintln!("Error matching files with extension: {}", extension);
        }
    }
}

fn process_image(unprocessed_image: &mut Vec<u8>, image_info: &ImageInfo, font_path: &String) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let resized_image = image_processing::resize_image(unprocessed_image);

    let image_with_transparent_border = image_processing::add_transparent_border(resized_image, 100);

    image_processing::add_text_to_image(image_with_transparent_border, &image_info.title, &image_info.artist, font_path)
}

fn set_wallpaper_macos(home_dir: &str, app_data_dir: &Path, file_name: &String) {
    let wallpaper_path = app_data_dir.join(file_name);
    let wallpaper_path_str = wallpaper_path.to_str().unwrap();

    let script = format!(
        "tell application \"System Events\" to tell every desktop to set picture to \"{}\"",
        wallpaper_path_str
    );

    match Command::new("osascript").arg("-e").arg(&script).status() {
        Ok(_) => println!("Set wallpaper successfully"),
        Err(err) => panic!("Error setting wallpaper: {}", err),
    }

    let db_path = Path::new(&home_dir).join("Library/Application Support/Dock/desktoppicture.db");

    let _ = Command::new("sqlite3")
        .arg(db_path)
        .arg("INSERT INTO data (value) VALUES (1);")
        .spawn()
        .expect("Failed to set 'Fit to Screen' option using sqlite3");
}

fn set_wallpaper(home_dir: &str, app_data_dir: &Path, file_name: &String) {
    match std::env::consts::OS {
        "macos" => set_wallpaper_macos(home_dir, app_data_dir, file_name),
        _ => println!("{} is an unsupported OS right now.", std::env::consts::OS),
    }
}
fn main() -> Result<(), reqwest::Error> {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    // Get the home directory
    let home_dir = match env::var("HOME") {
        Ok(path) => path,
        Err(_) => {
            panic!("Error: HOME environment variable is not set.");
        }
    };

    let app_data_dir = Path::new(&home_dir).join(".art-exposure");

    initialize_app_data(&app_data_dir);

    let mut font_path = "fonts/Lato-Regular.ttf".to_string();
    let mut query = "Impressionism".to_string();

    match args.len() {
        3 => {
            query = args[1].clone();
            font_path = args[2].clone();
        },
        _ => {
            println!("Usage: ./art-exposure [search term] [path to .ttf file]");
            println!("Using default search term: {} and font {}", query, font_path);
        }
    };

    let uri_encoded_query =
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();

    let search_url = format!(
        "https://collectionapi.metmuseum.org/public/collection/v1/search?q={}",
        uri_encoded_query
    );

    // Make a GET request to the search API
    let response: SearchResult = reqwest::blocking::get(&search_url)?.json().unwrap();

    let image_info = match api_client::fetch_random_image(&response) {
        Some(data) => data,
        None => {
            panic!("Could not find an image with a URL");
        }
    };
    // Display the image info
    println!("Artist: {}", &image_info.artist);
    println!("Title: {}", &image_info.title);
    println!("URL: {}", &image_info.url);

    // Download the image
    let image_response: Option<Response> = match reqwest::blocking::get(&image_info.url.clone()) {
        Ok(response) => Some(response),
        Err(err) => {
            panic!(
                "Error downloading image: {}. Error: {}",
                &image_info.url, err
            );
        }
    };

    let mut image_data = Vec::new();
    image_response.unwrap().copy_to(&mut image_data)?;

    let processed_image = process_image(&mut image_data, &image_info, &font_path);

    let file_name = image_info.artist.clone()
        + &String::from(" - ")
        + &image_info.title.clone()
        + &String::from(".png");

    let safe_file_name = make_filename_safe(&file_name);

    // Save the image to a file
    processed_image
        .save(&app_data_dir.join(safe_file_name.clone()))
        .unwrap();

    println!("Saved image to {}", &app_data_dir.join(safe_file_name.clone()).to_str().unwrap());

    // Set the wallpaper
    set_wallpaper(&home_dir, &app_data_dir, &safe_file_name);

    Ok(())
}
