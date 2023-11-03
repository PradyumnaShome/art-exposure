use glob::glob;
use rand::Rng;
use reqwest;
use reqwest::blocking::Response;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

extern crate image;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

extern crate url;

extern crate cocoa;

use cocoa::appkit::NSScreen;
use cocoa::base::nil;
use cocoa::foundation::NSRect;

#[derive(Deserialize)]
struct SearchResult {
    #[serde(rename = "objectIDs")]
    object_ids: Vec<u32>,
}

#[derive(Deserialize, Clone)]
struct ImageInfo {
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

fn add_transparent_border(image: &DynamicImage, border_width: u32) -> RgbaImage {
    let (original_width, original_height) = image.dimensions();
    let new_width = original_width + 2 * border_width;
    let new_height = original_height + 2 * border_width;

    let mut new_image = ImageBuffer::new(new_width, new_height);

    for x in 0..new_width {
        for y in 0..new_height {
            if x < border_width
                || x >= new_width - border_width
                || y < border_width
                || y >= new_height - border_width
            {
                new_image.put_pixel(x, y, Rgba([0, 0, 0, 0])); // Transparent border
            } else {
                let original_x = x - border_width;
                let original_y = y - border_width;
                let pixel = image.get_pixel(original_x, original_y);
                new_image.put_pixel(x, y, pixel);
            }
        }
    }

    new_image
}

fn resize_image(image_data: &Vec<u8>) -> DynamicImage {
    let new_height: u32 = unsafe {
        let screen = NSScreen::mainScreen(nil);
        let frame: NSRect = NSScreen::frame(screen);
        frame.size.height as u32
    };

    println!("Scaling to height: {}", new_height);

    let image = image::load_from_memory(image_data).unwrap();

    let (width, height) = image.dimensions();

    image.resize(
        ((width as f32 * new_height as f32) / height as f32) as u32,
        new_height,
        FilterType::Lanczos3,
    )
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

fn fetch_random_image(response: &SearchResult) -> Option<ImageInfo> {
    let mut tried_image_count = 0;
    let max_tries = 20;
    let mut rng = rand::thread_rng();
    let mut image_info: Option<ImageInfo>;

    while tried_image_count < max_tries {
        // Pick a random object ID from the response
        let random_index = rng.gen_range(0, response.object_ids.len());
        let random_object_id = response.object_ids[random_index];

        // Make a GET request to the object API
        let object_url = format!(
            "https://collectionapi.metmuseum.org/public/collection/v1/objects/{}",
            random_object_id
        );

        image_info = match reqwest::blocking::get(&object_url).unwrap().json() {
            Ok(info) => Some(info),
            Err(err) => {
                println!("Error getting object info: {}", err);
                continue;
            }
        };
        match image_info.clone().unwrap().url.is_empty() {
            false => {
                return image_info;
            }
            _ => (),
        };

        tried_image_count += 1;
    }
    return None;
}

fn process_image(unprocessed_image: &mut Vec<u8>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let resized_image = resize_image(unprocessed_image);

    add_transparent_border(&resized_image, 100)
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

    let query = match args.len() {
        1 => "Impressionism".to_string(),
        2 => args[1].clone(),
        _ => panic!("Usage: ./art-exposure [search term]"),
    };

    let uri_encoded_query =
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();

    let search_url = format!(
        "https://collectionapi.metmuseum.org/public/collection/v1/search?q={}",
        uri_encoded_query
    );

    // Make a GET request to the search API
    let response: SearchResult = reqwest::blocking::get(&search_url)?.json().unwrap();

    let image_info = match fetch_random_image(&response) {
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

    let processed_image = process_image(&mut image_data);

    let file_name = image_info.artist.clone()
        + &String::from(" - ")
        + &image_info.title.clone()
        + &String::from(".png");

    let safe_file_name = make_filename_safe(&file_name);

    // Save the image to a file
    processed_image
        .save(&app_data_dir.join(safe_file_name.clone()))
        .unwrap();

    // Set the wallpaper
    set_wallpaper(&home_dir, &app_data_dir, &safe_file_name);

    Ok(())
}
