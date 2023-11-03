extern crate imageproc;
extern crate rusttype;
use std::fs::File;

extern crate image;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};

use cocoa::appkit::NSScreen;
use cocoa::base::nil;
use cocoa::foundation::NSRect;

use std::io::Read;

fn load_custom_font(font_path: &str) -> Result<rusttype::Font<'static>, std::io::Error> {
    let mut font_file = File::open(font_path)?;
    let mut font_data = Vec::new();
    font_file.read_to_end(&mut font_data)?;
    let font = Font::try_from_vec(font_data).unwrap();
    Ok(font)
}

pub fn add_text_to_image(image: RgbaImage, title: &str, artist: &str, font_path: &String) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    // Convert the DynamicImage to an RgbaImage
    let mut rgba_image: RgbaImage = image.clone();

    // Load a font and specify the font size
    let font = load_custom_font(&font_path).unwrap();

    let title_scale = Scale::uniform(1.0 * image.width() as f32 / title.len() as f32);
    let artist_scale = Scale::uniform(0.5 * image.width() as f32 / artist.len() as f32);

    // Define text positions
    let title_position = (
        (0.3 * image.width() as f32) as i32,
        image.height() as i32 - 275
    ); // Adjust these values as needed
    
    let artist_position = (
        (0.4 * image.width() as f32) as i32,
        image.height() as i32 - 250 + title_scale.y as i32
    ); // Adjust these values as needed

    // Draw the "Title" text in bold
    let title_color = Rgba([0, 0, 0, 255]); // Adjust the color as needed
    draw_text_mut(&mut rgba_image, title_color, title_position.0, title_position.1, title_scale, &font, title);

    // Draw the "Artist" text
    let artist_color = Rgba([0, 0, 0, 255]); // Adjust the color as needed
    draw_text_mut(&mut rgba_image, artist_color, artist_position.0, artist_position.1,  artist_scale, &font, artist);

    rgba_image
}

pub fn add_transparent_border(image: DynamicImage, border_width: u32) -> RgbaImage {
    let (original_width, original_height) = image.dimensions();
    let bottom_border_height = 3 * border_width;
    let new_width = original_width + 2 * border_width;
    let new_height = original_height +  border_width + bottom_border_height;

    let mut new_image = ImageBuffer::new(new_width, new_height);

    for x in 0..new_width {
        for y in 0..new_height {
            if x < border_width
                || x >= new_width - border_width
                || y < border_width
                || y >= new_height - bottom_border_height
            {
                new_image.put_pixel(x, y, Rgba([0, 0, 0, 0])); // Transparent border
            } else {
                let original_x = x - border_width;
                let original_y = y - border_width;
                let pixel = image.get_pixel(original_x, original_y);
                new_image.put_pixel(x, y, pixel.clone());
            }
        }
    }

    new_image
}

pub fn resize_image(image_data: &Vec<u8>) -> DynamicImage {
    let new_height: u32 = unsafe {
        let screen = NSScreen::mainScreen(nil);
        let frame: NSRect = NSScreen::frame(screen);
        frame.size.height as u32
    };

    let image = image::load_from_memory(image_data).unwrap();

    let (width, height) = image.dimensions();

    image.resize(
        ((width as f32 * new_height as f32) / height as f32) as u32,
        new_height,
        FilterType::Lanczos3,
    )
}