use crate::ImageInfo;
use crate::SearchResult;
use rand::Rng;

pub fn fetch_random_image(response: &SearchResult) -> Option<ImageInfo> {
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