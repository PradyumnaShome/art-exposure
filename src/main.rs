use reqwest;
use reqwest::blocking::Response;
use serde::{Deserialize};
use rand::Rng;
use std::process::Command;
use std::env;

extern crate image;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

extern crate url;

extern crate cocoa;

use cocoa::appkit::{NSScreen};
use cocoa::foundation::NSRect;
use cocoa::base::nil;

use dirs::home_dir;

#[derive(Deserialize)]
struct SearchResult {
    total: u32,
    objectIDs: Vec<u32>,
}

#[derive(Deserialize)]
struct ObjectInfo {
    title: String,
    artistDisplayName: String,
    primaryImage: String,
}

fn add_transparent_border(image: &DynamicImage, border_width: u32) -> RgbaImage {
    let (original_width, original_height) = image.dimensions();
    let new_width = original_width + 2 * border_width;
    let new_height = original_height + 2 * border_width;

    let mut new_image = ImageBuffer::new(new_width, new_height);

    for x in 0..new_width {
        for y in 0..new_height {
            if x < border_width || x >= new_width - border_width || y < border_width || y >= new_height - border_width {
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


fn resize_image(image_data: & Vec<u8>)-> DynamicImage {
    let new_height:u32 =  unsafe {
        let screen = NSScreen::mainScreen(nil);
        let frame: NSRect = NSScreen::frame(screen);
        frame.size.height as u32
    };

    println!("Scaling to height: {}", new_height);
    
    let image = image::load_from_memory(image_data).unwrap();

    let (width, height) = image.dimensions();

    image.resize(
        ((width as f32  * new_height as f32) / height as f32) as u32,
        new_height,
        FilterType::Lanczos3)
}

fn main() -> Result<(), reqwest::Error> {

    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    let query = match args.len() {
        1 => "Impressionism".to_string(),
        2 => args[1].clone(),
        _ => panic!("Usage: ./art-exposure [search term]"),
    };
    
    let uri_encoded_query= url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
    
    let search_url = format!(
        "https://collectionapi.metmuseum.org/public/collection/v1/search?q={}",
        uri_encoded_query
    );
    
    // Step 1: Make a GET request to the search API
    let response: SearchResult = reqwest::blocking::get(&search_url)?.json().unwrap();


    // Step 2: Pick a random objectID
    let mut tried_image_count = 0;
    let max_tries = 10;
    let mut rng = rand::thread_rng();
    let mut image_url: Option<String> = None;
    let mut image_title: Option<String> = None;
    let mut image_artist: Option<String> = None;

    while tried_image_count < max_tries {
        if image_url.is_some() {
            break;
        }
        let random_index = rng.gen_range(0, response.objectIDs.len());
        let random_object_id = response.objectIDs[random_index];
        
        // Step 3: Make a GET request to the object API
        let object_url = format!(
            "https://collectionapi.metmuseum.org/public/collection/v1/objects/{}",
            random_object_id
        );
    
        let object_info: Option<ObjectInfo> = match reqwest::blocking::get(&object_url)?.json() {
            Ok(info) => Some(info),
            Err(err) => {
                println!("Error getting object info: {}", err);
                continue;
            }
        };
        image_url = match object_info.as_ref().unwrap().primaryImage.is_empty() {
            true => None,
            false  => {
                image_title = Some(object_info.as_ref().unwrap().title.clone());
                println!("Title: {}", object_info.as_ref().unwrap().title);

                image_artist = Some(object_info.as_ref().unwrap().artistDisplayName.clone());
                println!("Artist: {}", object_info.as_ref().unwrap().artistDisplayName);

                Some(object_info.as_ref().unwrap().primaryImage.clone())   
            },
        };

        tried_image_count += 1;
    }

    if image_url.is_none() {
        panic!("Could not find an image with a URL");
    }

    let image_url = image_url.unwrap();
    // Step 4: Download the image
    println!("Downloading image: {}", image_url);
    let image_response: Option<Response> = match reqwest::blocking::get(image_url.clone()) {
        Ok(response) => Some(response),
        Err(err) => {
            panic!("Error downloading image: {}. Error: {}", image_url, err);
        }
    };

    let mut image_data = Vec::new();
    image_response.unwrap().copy_to(&mut image_data)?;

    let resized_image = resize_image(&mut image_data);

    let resized_image_with_transparent_border = add_transparent_border(&resized_image, 50);

    let file_name = image_artist.clone().unwrap() + &String::from(" - ") + &image_title.clone().unwrap() + &String::from(".png");

    // Step 5: Save the image to a file
    resized_image_with_transparent_border.save(file_name.clone()).unwrap();

    // Step 6: Set the macOS background
    match Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "tell application \"System Events\" to set picture of every desktop to \"{}\"",
            std::env::current_dir()
                .unwrap()
                .join(file_name.clone())
                .display()
        ))
        .status() {
            Ok(_) => println!("Set background successfully"),
            Err(err) => println!("Error setting background: {}", err),
        }
    
    let db_path =  if let Some(home) = home_dir() {
        home.join("Library/Application Support/Dock/desktoppicture.db")
    } else {
        panic!("Could not find home directory");
    };

    let _ = Command::new("sqlite3")
        .arg(db_path)
        .arg("INSERT INTO data (value) VALUES (1);")
        .spawn()
        .expect("Failed to set 'Fit to Screen' option using sqlite3");

    Ok(())
}
