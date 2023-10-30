use reqwest;
use reqwest::blocking::Response;
use serde::{Deserialize};
use rand::Rng;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::env;

extern crate url;

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

fn main() -> Result<(), reqwest::Error> {

    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    let query = match args.len() {
        1 => "French Impressionism".to_string(),
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
                println!("Title: {}", object_info.as_ref().unwrap().title);
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

    // Step 5: Save the image to a file
    let mut file = File::create("met_image.jpg").unwrap();
    file.write_all(&image_data).unwrap();

    // Step 6: Set the macOS background
    match Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "tell application \"System Events\" to set picture of every desktop to \"{}\"",
            std::env::current_dir()
                .unwrap()
                .join("met_image.jpg")
                .display()
        ))
        .status() {
            Ok(_) => println!("Set background successfully"),
            Err(err) => println!("Error setting background: {}", err),
        }

    Ok(())
}
