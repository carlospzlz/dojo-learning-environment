use image::imageops;
use image::{DynamicImage, GrayImage, Rgb, RgbImage};
use imageproc::filter::median_filter;

const LIFE_BAR_Y: u32 = 54;
// Life bar seems to be 152 pixels wide
const PLAYER_1_LIFE_BAR_X: [u32; 2] = [12, 164];
const PLAYER_2_LIFE_BAR_X: [u32; 2] = [204, 356];
const VISUALIZATION_BAR_HEIGHT: u32 = 7;

pub struct LifeInfo {
    pub life: f32,
    pub damage: f32,
}

impl Default for LifeInfo {
    fn default() -> Self {
        LifeInfo {
            life: 1.0,
            damage: 0.0,
        }
    }
}

pub fn visualize_life_bars(img: RgbImage) -> RgbImage {
    let grayscale_img = DynamicImage::ImageRgb8(img).to_luma8();
    let mut color_img = DynamicImage::ImageLuma8(grayscale_img.clone()).to_rgb8();
    draw_visualized_life_bar(&grayscale_img, &mut color_img, PLAYER_1_LIFE_BAR_X);
    draw_visualized_life_bar(&grayscale_img, &mut color_img, PLAYER_2_LIFE_BAR_X);
    color_img
}

fn draw_visualized_life_bar(
    grayscale_img: &GrayImage,
    color_img: &mut RgbImage,
    x_limits: [u32; 2],
) {
    for x in x_limits[0]..x_limits[1] {
        let color;
        match grayscale_img.get_pixel(x, LIFE_BAR_Y)[0] {
            0..=100 => color = Rgb([0, 0, 255]),   // Life taken
            101..=200 => color = Rgb([0, 255, 0]), // Life remaining
            201..=255 => color = Rgb([255, 0, 0]), // Hit damage
        }
        let half_height = VISUALIZATION_BAR_HEIGHT / 2;
        for y in LIFE_BAR_Y - half_height..LIFE_BAR_Y + half_height {
            color_img.put_pixel(x, y, color);
        }
    }
}

pub fn get_life_info(img: RgbImage) -> (LifeInfo, LifeInfo) {
    let img = DynamicImage::ImageRgb8(img).to_luma8();
    let player_1_life_info = get_life_info_for_player(&img, PLAYER_1_LIFE_BAR_X);
    let player_2_life_info = get_life_info_for_player(&img, PLAYER_2_LIFE_BAR_X);
    (player_1_life_info, player_2_life_info)
}

fn get_life_info_for_player(img: &GrayImage, x_limits: [u32; 2]) -> LifeInfo {
    let mut life_count = 0;
    let mut damage_count = 0;
    for x in x_limits[0]..x_limits[1] {
        match img.get_pixel(x, LIFE_BAR_Y)[0] {
            0..=100 => (),                  // Life taken
            101..=200 => life_count += 1,   // Life remaining
            201..=255 => damage_count += 1, // Hit damage
        }
    }
    let total = (x_limits[1] - x_limits[0]) as f32;
    LifeInfo {
        life: life_count as f32 / total,
        damage: damage_count as f32 / total,
    }
}

pub fn get_mse(img1: &RgbImage, img2: &RgbImage) -> f32 {
    if (img1.width() != img2.width()) || (img1.height() != img2.height()) {
        panic!(
            "Image dimensions differ: {}x{}, {}x{}",
            img1.width(),
            img1.height(),
            img2.width(),
            img2.height()
        );
    }
    let mut sum_squared_diff: i32 = 0;
    for x in 0..img1.width() {
        for y in 0..img1.height() {
            let pixel1 = img1.get_pixel(x, y);
            let pixel2 = img2.get_pixel(x, y);
            let r1 = pixel1[0] as i32;
            let g1 = pixel1[1] as i32;
            let b1 = pixel1[2] as i32;
            let r2 = pixel2[0] as i32;
            let g2 = pixel2[1] as i32;
            let b2 = pixel2[2] as i32;
            sum_squared_diff += (r1 - r2).pow(2) + (g1 - g2).pow(2) + (b1 - b2).pow(2);
        }
    }
    let total_pixels = (img1.width() * img1.height()) as f32;
    let mse = sum_squared_diff as f32 / total_pixels;
    // Normalize (max MSE is 255^2)
    let mse = mse / (1 << 16) as f32;
    mse
}

pub fn get_frame_abstraction(
    frame: &RgbImage,
    hist_threshold: u32,
    blur: f32,
    radius: u32,
) -> RgbImage {
    // Remove life bars
    let frame = DynamicImage::ImageRgb8(frame.clone()).crop(0, 100, 368, 480);
    let mut frame = frame.to_rgb8();
    // Drop pixels that are likely to be background
    let histogram = get_histogram(&frame);
    remove_more_frequent_than(&mut frame, &histogram, hist_threshold);
    // Extra processing: Blur and Median
    let frame = imageops::blur(&frame, blur);
    let frame = median_filter(&frame, radius, radius);
    //Down-size, so compute time doesn't explode
    let frame = DynamicImage::ImageRgb8(frame);
    let frame = frame.resize_exact(50, 50, image::imageops::FilterType::Nearest);
    frame.to_rgb8()
}

pub fn get_histogram(img: &RgbImage) -> Vec<Vec<Vec<u32>>> {
    let mut histogram = vec![vec![vec![0; 256]; 256]; 256];
    for x in 0..img.width() {
        for y in 0..img.height() {
            let pixel = img.get_pixel(x, y);
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            histogram[r as usize][g as usize][b as usize] += 1;
        }
    }
    histogram
}

pub fn remove_more_frequent_than(img: &mut RgbImage, histogram: &Vec<Vec<Vec<u32>>>, max: u32) {
    for x in 0..img.width() {
        for y in 0..img.height() {
            let pixel = img.get_pixel(x, y);
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            if histogram[r as usize][g as usize][b as usize] > max {
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
    }
}
