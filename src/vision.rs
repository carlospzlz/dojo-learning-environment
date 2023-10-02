use image::imageops;
use image::{DynamicImage, GrayImage, Rgb, RgbImage};
use imageproc::distance_transform::Norm;
use imageproc::filter::median_filter;
use imageproc::morphology::dilate;

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
    let mut sum_squared_diff: i64 = 0;
    for x in 0..img1.width() {
        for y in 0..img1.height() {
            let pixel1 = img1.get_pixel(x, y);
            let pixel2 = img2.get_pixel(x, y);
            let r1 = pixel1[0] as i64;
            let g1 = pixel1[1] as i64;
            let b1 = pixel1[2] as i64;
            let r2 = pixel2[0] as i64;
            let g2 = pixel2[1] as i64;
            let b2 = pixel2[2] as i64;
            sum_squared_diff += (r1 - r2).pow(2) + (g1 - g2).pow(2) + (b1 - b2).pow(2);
        }
    }
    let total_pixels = img1.width() * img1.height();
    let max_sum_squared_diff = total_pixels * 255_u32.pow(2) * 3;
    let mse = sum_squared_diff as f32 / max_sum_squared_diff as f32;
    mse
}

pub fn get_frame_abstraction(
    frame: &RgbImage,
    hist_threshold: u32,
    blur: f32,
    radius: u32,
    min_red: u8,
    min_green: u8,
    min_blue: u8,
    low_red: u8,
    low_green: u8,
    low_blue: u8,
) -> Option<RgbImage> {
    // Remove life bars
    let frame = DynamicImage::ImageRgb8(frame.clone()).crop(0, 100, 368, 480);
    let mut frame = frame.to_rgb8();
    let mask = apply_thresholds(&frame, min_red, min_green, min_blue, low_red, low_green, low_blue);
    let mask = DynamicImage::ImageRgb8(mask).to_luma8();
    //let mask = dilate(&mask, Norm::L1, 16);
    let mask = dilate(&mask, Norm::L1, 6);
    // Discard bad abstractions
    if get_detected_amount(&mask) < 0.1 {
        println!("Discarded");
        return None
    }
    apply_mask(&mut frame, &mask);
    // Drop pixels that are likely to be background
    //let histogram = get_histogram(&frame);
    //remove_more_frequent_than(&mut frame, &histogram, hist_threshold);
    // Extra processing: Blur and Median
    //let frame = imageops::blur(&frame, blur);
    //let frame = median_filter(&frame, radius, radius);
    //Down-size, so compute time doesn't explode
    let frame = DynamicImage::ImageRgb8(frame);
    let frame = frame.resize_exact(100, 100, image::imageops::FilterType::Lanczos3);
    //DynamicImage::ImageLuma8(frame).to_rgb8()
    Some(frame.to_rgb8())
    //frame
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

pub fn enclose_with_q(img: &mut RgbImage, q: f32) {
    if q == 0.0 {
        return;
    }
    let color = if q > 0.0 {
        Rgb([0, (q * 255.0) as u8, 0])
    } else {
        Rgb([(-q * 255.0) as u8, 0, 0])
    };
    for x in 0..img.width() {
        img.put_pixel(x, 0, color);
        img.put_pixel(x, img.height() - 1, color);
    }
    for y in 0..img.height() {
        img.put_pixel(0, y, color);
        img.put_pixel(img.width() - 1, y, color);
    }
}

pub fn apply_thresholds(img: &RgbImage, min_red: u8, min_green: u8, min_blue: u8, low_red: u8, low_green: u8, low_blue: u8) -> RgbImage {
    // Remove life bars
    //let img = DynamicImage::ImageRgb8(img.clone()).crop(0, 100, 368, 480);
    //let img = img.to_rgb8();
    let mut img_out = RgbImage::new(img.width(), img.height());
    // Avoid some annoying white dots at the right of the frame
    for x in 0..img.width() - 1 {
        for y in 0..img.height() {
            let pixel = img.get_pixel(x, y);
            let r = if pixel[0] > min_red || pixel[0] < low_red { pixel[0] } else { 0 };
            let g = if pixel[1] > min_green || pixel[1] < low_green { pixel[1] } else { 0 };
            let b = if pixel[2] > min_blue || pixel[2] < low_blue { pixel[2] } else { 0 };
            img_out.put_pixel(x, y, Rgb([r, g, b]));
        }
    }
    img_out
}

fn apply_mask(img: &mut RgbImage, mask: &GrayImage) {
    if (img.width() != img.width()) || (mask.height() != mask.height()) {
        panic!(
            "Image dimensions differ: {}x{}, {}x{}",
            img.width(),
            img.height(),
            mask.width(),
            mask.height()
        );
    }
    for x in 0..img.width() {
        for y in 0..img.height() {
            let pixel = img.get_pixel(x, y);
            let multiplier = mask.get_pixel(x, y)[0] as f32 / 255.0;
            let r = (pixel[0] as f32 * multiplier) as u8;
            let g = (pixel[1] as f32 * multiplier) as u8;
            let b = (pixel[2] as f32 * multiplier) as u8;
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }
}

fn get_detected_amount(img: &GrayImage) -> f32 {
    let mut count = 0;
    for x in 0..img.width() {
        for y in 0..img.height() {
            count += (img.get_pixel(x, y)[0] > 0) as u32;
        }
    }
    count as f32 / (img.width() * img.height()) as f32
}
