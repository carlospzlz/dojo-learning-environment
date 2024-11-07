use image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage};
use imageproc::distance_transform::Norm;
use imageproc::filter::median_filter;
use imageproc::morphology::{dilate, erode};
use std::cmp;
use std::collections::{HashMap, VecDeque};

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

pub struct VisionStages {
    pub cropped_frame: RgbImage,
    pub contrast_frame: RgbImage,
    pub mask: RgbImage,
    pub masked_frame: RgbImage,
    pub centroids_hud: RgbImage,
    pub chars_hud: RgbImage,
    pub segmented_frame: RgbImage,
}

impl VisionStages {
    fn new(
        cropped_frame: RgbImage,
        contrast_frame: RgbImage,
        mask: RgbImage,
        masked_frame: RgbImage,
        centroids_hud: RgbImage,
        chars_hud: RgbImage,
        segmented_frame: RgbImage,
    ) -> Self {
        Self {
            cropped_frame,
            contrast_frame,
            mask,
            masked_frame,
            centroids_hud,
            chars_hud,
            segmented_frame,
        }
    }
}

impl Default for VisionStages {
    fn default() -> Self {
        Self {
            cropped_frame: RgbImage::default(),
            contrast_frame: RgbImage::default(),
            mask: RgbImage::default(),
            masked_frame: RgbImage::default(),
            centroids_hud: RgbImage::default(),
            chars_hud: RgbImage::default(),
            segmented_frame: RgbImage::default(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Character {
    mask: GrayImage,
    corner1: (u32, u32),
    corner2: (u32, u32),
}

impl Character {
    fn new(mask: GrayImage, corner1: (u32, u32), corner2: (u32, u32)) -> Self {
        Self {
            mask,
            corner1,
            corner2,
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
    let mut count = 0;
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
            if (r1, g1, b1) != (0, 0, 0) || (r2, g2, b2) != (0, 0, 0) {
                sum_squared_diff += (r1 - r2).pow(2) + (g1 - g2).pow(2) + (b1 - b2).pow(2);
                count += 1;
            }
        }
    }
    //let total_pixels = img1.width() * img1.height();
    let max_sum_squared_diff = count * 255_u32.pow(2) * 3;
    let mse = sum_squared_diff as f32 / max_sum_squared_diff as f32;
    mse
}

pub fn get_mse_in_x_limits(
    img1: &RgbImage,
    img2: &RgbImage,
    x_limits1: (u32, u32),
    x_limits2: (u32, u32),
) -> f32 {
    let width1 = x_limits1.1 - x_limits1.0;
    let width2 = x_limits2.1 - x_limits2.0;
    if width1 != width2 {
        panic!(
            "X limits differ: {}-{}, {}-{}",
            x_limits1.0, x_limits1.1, x_limits2.0, x_limits2.1
        )
    }
    let mut sum_squared_diff: i64 = 0;
    let mut count = 0;
    for x in 0..width1 {
        for y in 0..img1.height() {
            let pixel1 = img1.get_pixel(x_limits1.0 + x, y);
            let pixel2 = img2.get_pixel(x_limits2.0 + x, y);
            let r1 = pixel1[0] as i64;
            let g1 = pixel1[1] as i64;
            let b1 = pixel1[2] as i64;
            let r2 = pixel2[0] as i64;
            let g2 = pixel2[1] as i64;
            let b2 = pixel2[2] as i64;
            if (r1, g1, b1) != (0, 0, 0) || (r2, g2, b2) != (0, 0, 0) {
                sum_squared_diff += (r1 - r2).pow(2) + (g1 - g2).pow(2) + (b1 - b2).pow(2);
                count += 1;
            }
        }
    }
    let max_sum_squared_diff = count * 255_u32.pow(2) * 3;
    let mse = sum_squared_diff as f32 / max_sum_squared_diff as f32;
    mse
}

pub fn get_frame_abstraction(
    frame: &RgbImage,
    red_thresholds: [u8; 2],
    green_thresholds: [u8; 2],
    blue_thresholds: [u8; 2],
    dilate_k: u8,
    erode_k: u8,
    filter_radius: u32,
    histogram1: &mut HashMap<Rgb<u8>, f64>,
    histogram2: &mut HashMap<Rgb<u8>, f64>,
    char1_pixel_probability: &mut HashMap<Rgb<u8>, (u64, u64)>,
    char2_pixel_probability: &mut HashMap<Rgb<u8>, (u64, u64)>,
    char1_probability_threshold: f64,
    char2_probability_threshold: f64,
) -> (Option<RgbImage>, VisionStages) {
    // Remove life bars
    let cropped_frame = DynamicImage::ImageRgb8(frame.clone()).crop(0, 100, 368, 480);
    let cropped_frame = cropped_frame.clone().to_rgb8();

    // Apply contrast thresholds
    let contrast_frame = apply_thresholds(
        &cropped_frame,
        red_thresholds,
        green_thresholds,
        blue_thresholds,
    );

    // Make it a mask
    let mask = DynamicImage::ImageRgb8(contrast_frame.clone()).to_luma8();
    let mask = dilate(&mask, Norm::L1, dilate_k);

    // Apply mask
    let mut masked_frame = cropped_frame.clone();
    apply_mask(&mut masked_frame, &mask);

    // Centroids
    let (corner1, corner2) = find_corners(&mask);
    let (centroid1, centroid2) = find_centroids(&mask, corner1, corner2);
    let mut centroids_hud = DynamicImage::ImageLuma8(mask.clone()).to_rgb8();
    draw_centroids(
        &mut centroids_hud,
        corner1,
        corner2,
        centroid1,
        centroid2,
    );

    // Grow and enclose characters
    let char1 = grow_region(&mask, &centroid1, &corner1, &corner2);
    let char2 = grow_region(&mask, &centroid2, &corner1, &corner2);

    // Branching depending on how close characters are
    let disjoint = (char1.corner2.0 < char2.corner1.0)
        || (char1.corner1.0 > char2.corner2.0)
        || (char1.corner2.1 < char2.corner1.1)
        || (char1.corner1.1 > char2.corner2.1);

    // Check if characters have crossed sides
    let (char1, char2) = if disjoint {
        swap_if_needed(char1, char2, &histogram1, &histogram2, &cropped_frame)
    } else {
        (char1, char2)
    };

    let chars_hud = if disjoint {
        draw_framed_disjoint_chars(&char1, &char2)
    } else {
        draw_framed_overlapped_chars(&char1, &char2)
    };

    // Final segmentation
    if disjoint {
        // Experimental: Compute probs
        update_probabilities(&char1, &cropped_frame, char1_pixel_probability);
        update_probabilities(&char2, &cropped_frame, char2_pixel_probability);
        update_histograms(&char1, &char2, &masked_frame, histogram1, histogram2);
    }
    let segmented_frame = if disjoint {
        draw_disjoint_chars(&char1, &char2)
    } else {
        let segmented_frame = segment_overlapped_chars_by_histogram(
            &masked_frame,
            &corner1,
            &corner2,
            histogram1,
            histogram2,
        );
        median_filter(&segmented_frame, filter_radius, filter_radius)
    };

    // Experimental: Segment by probability
    let segmented_by_probability = segment_by_probability(
        &cropped_frame,
        &char1_pixel_probability,
        &char2_pixel_probability,
        char1_probability_threshold,
        char2_probability_threshold,
    );
    // segment each char separately from mask
    // Don't median, dilate
    // merge

    // when merged
    // get segment from blob
    // dilate
    // merge somehow (use violet?)

    // Vision stages
    let mask = DynamicImage::ImageLuma8(mask).to_rgb8();
    let vision_stages = VisionStages::new(
        cropped_frame,
        contrast_frame,
        mask,
        masked_frame,
        centroids_hud,
        chars_hud,
        segmented_frame.clone(),
    );

    let frame_abstraction = segmented_frame;

    // Discard bad abstractions
    //if get_detected_amount(&mask) < 0.02 {
    //    println!("Discarded");
    //    return (None, vision_stages);
    //}

    //Down-size, so compute time doesn't explode
    //let frame = DynamicImage::ImageRgb8(frame);
    //let frame = frame.resize_exact(100, 100, image::imageops::FilterType::Lanczos3);
    (Some(frame_abstraction), vision_stages)
}

pub fn crop_frame(frame: RgbImage) {
    DynamicImage::ImageRgb8(frame.clone()).crop(0, 100, 368, 480);
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

pub fn apply_thresholds(
    img: &RgbImage,
    red_thresholds: [u8; 2],
    green_thresholds: [u8; 2],
    blue_thresholds: [u8; 2],
) -> RgbImage {
    // Remove life bars
    //let img = DynamicImage::ImageRgb8(img.clone()).crop(0, 100, 368, 480);
    //let img = img.to_rgb8();
    let mut img_out = RgbImage::new(img.width(), img.height());
    // Avoid some annoying white dots at the right of the frame
    for x in 0..img.width() - 1 {
        for y in 0..img.height() {
            let pixel = img.get_pixel(x, y);
            let r = if pixel[0] < red_thresholds[0] || pixel[0] > red_thresholds[1] {
                pixel[0]
            } else {
                0
            };
            let g = if pixel[1] < green_thresholds[0] || pixel[1] > green_thresholds[1] {
                pixel[1]
            } else {
                0
            };
            let b = if pixel[2] < blue_thresholds[0] || pixel[2] > blue_thresholds[1] {
                pixel[2]
            } else {
                0
            };
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

fn find_corners(img: &GrayImage) -> ((u32, u32), (u32, u32)) {
    let mut corner1 = (img.width() - 1, img.height() - 1);
    let mut corner2 = (0, 0);
    for x in 0..img.width() {
        for y in 0..img.height() {
            if img.get_pixel(x, y)[0] != 0 {
                if x < corner1.0 {
                    corner1.0 = x;
                }
                if y < corner1.1 {
                    corner1.1 = y;
                }
                if x > corner2.0 {
                    corner2.0 = x;
                }
                if y > corner2.1 {
                    corner2.1 = y;
                }
            }
        }
    }

    (corner1, corner2)
}

fn find_centroids(
    img: &GrayImage,
    corner1: (u32, u32),
    corner2: (u32, u32),
) -> ((u32, u32), (u32, u32)) {
    let half_x = corner1.0 + (corner2.0 - corner1.0) / 2 as u32;

    // Centroid1
    let mut centroid1 = (0, 0);
    let mut max_count = 0;
    for x in corner1.0..half_x {
        let mut count = 0;
        for y in corner1.1..corner2.1 {
            count += if img.get_pixel(x, y)[0] != 0 { 1 } else { 0 };
        }
        if count > max_count {
            centroid1.0 = x;
            max_count = count;
        }
    }
    let mut max_count = 0;
    for y in corner1.1..corner2.1 {
        let mut count = 0;
        for x in corner1.0..half_x {
            count += if img.get_pixel(x, y)[0] != 0 { 1 } else { 0 };
        }
        if count > max_count {
            centroid1.1 = y;
            max_count = count;
        }
    }

    // Centroid2
    let mut centroid2 = (0, 0);
    let mut max_count = 0;
    for x in half_x..corner2.0 {
        let mut count = 0;
        for y in corner1.1..corner2.1 {
            count += if img.get_pixel(x, y)[0] != 0 { 1 } else { 0 };
        }
        if count > max_count {
            centroid2.0 = x;
            max_count = count;
        }
    }
    let mut max_count = 0;
    for y in corner1.1..corner2.1 {
        let mut count = 0;
        for x in half_x..corner2.0 {
            count += if img.get_pixel(x, y)[0] != 0 { 1 } else { 0 };
        }
        if count > max_count {
            centroid2.1 = y;
            max_count = count;
        }
    }

    (centroid1, centroid2)
}

fn draw_centroids(
    img: &mut RgbImage,
    corner1: (u32, u32),
    corner2: (u32, u32),
    centroid1: (u32, u32),
    centroid2: (u32, u32),
) {
    // Green border
    for x in corner1.0..corner2.0 {
        img.put_pixel(x, corner1.1, Rgb([0, 255, 0]));
        img.put_pixel(x, corner2.1, Rgb([0, 255, 0]));
    }
    for y in corner1.1..corner2.1 {
        img.put_pixel(corner1.0, y, Rgb([0, 255, 0]));
        img.put_pixel(corner2.0, y, Rgb([0, 255, 0]));
    }

    // Centroids
    for x in corner1.0..corner2.0 {
        img.put_pixel(x, centroid1.1, Rgb([255, 0, 0]));
        img.put_pixel(x, centroid2.1, Rgb([0, 0, 255]));
    }
    for y in corner1.1..corner2.1 {
        img.put_pixel(centroid1.0, y, Rgb([255, 0, 0]));
        img.put_pixel(centroid2.0, y, Rgb([0, 0, 255]));
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

#[allow(dead_code)]
pub fn get_x_limits(img: &RgbImage) -> (u32, u32) {
    let mut min_x = img.width();
    let mut max_x = 0;
    for x in 0..img.width() {
        for y in 0..img.height() {
            let pixel = img.get_pixel(x, y);
            if pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0 {
                min_x = cmp::min(x, min_x);
                max_x = cmp::max(x, max_x);
            }
        }
    }
    (min_x, max_x)
}

pub fn draw_x_limits(img: &mut RgbImage, x_limits: (u32, u32)) {
    let color = Rgb([0, 128, 0]);
    for y in 0..img.height() {
        img.put_pixel(x_limits.0, y, color);
        img.put_pixel(x_limits.1, y, color);
    }
}

pub fn draw_border(img: &mut RgbImage) {
    let color = Rgb([128, 0, 128]);
    for x in 0..img.width() {
        img.put_pixel(x, 0, color);
        img.put_pixel(x, img.height() - 1, color);
    }
    for y in 0..img.height() {
        img.put_pixel(0, y, color);
        img.put_pixel(img.width() - 1, y, color);
    }
}

fn update_histograms(
    char1: &Character,
    char2: &Character,
    img: &RgbImage,
    histogram1: &mut HashMap<Rgb<u8>, f64>,
    histogram2: &mut HashMap<Rgb<u8>, f64>,
) {
    for (x, y, present) in char1.mask.enumerate_pixels() {
        if present[0] > 0 {
            let pixel = img.get_pixel(x, y);
            *histogram1.entry(*pixel).or_insert(0.0) += 1.0;
        }
    }
    for (x, y, present) in char2.mask.enumerate_pixels() {
        if present[0] > 0 {
            let pixel = img.get_pixel(x, y);
            *histogram2.entry(*pixel).or_insert(0.0) += 1.0;
        }
    }
}

pub fn identify_fighters_by_side(
    mask: &GrayImage,
    corner1: &(u32, u32),
    corner2: &(u32, u32),
) -> RgbImage {
    let mut img_out = RgbImage::new(mask.width(), mask.height());
    let half_x = corner1.0 + (corner2.0 - corner1.0) / 2 as u32;
    for x in corner1.0..half_x {
        for y in corner1.1..corner2.1 {
            if mask.get_pixel(x, y)[0] > 0 {
                img_out.put_pixel(x, y, Rgb([255, 0, 0]));
            }
        }
    }
    for x in half_x..corner2.0 {
        for y in corner1.1..corner2.1 {
            if mask.get_pixel(x, y)[0] > 0 {
                img_out.put_pixel(x, y, Rgb([0, 0, 255]));
            }
        }
    }
    img_out
}

fn grow_region(
    mask: &GrayImage,
    centroid: &(u32, u32),
    corner1: &(u32, u32),
    corner2: &(u32, u32),
) -> Character {
    let mut mask_out = GrayImage::new(mask.width(), mask.height());
    let mut region_corner1 = (mask.width(), mask.height());
    let mut region_corner2 = (0, 0);

    if mask.is_empty() {
        return Character::new(mask_out, region_corner1, region_corner2);
    }

    let mut queue = VecDeque::new();
    let mut visited = vec![vec![false; mask.height() as usize]; mask.width() as usize];
    queue.push_back(centroid.clone());

    // Process queue
    while let Some((x, y)) = queue.pop_front() {
        // Mask
        mask_out.put_pixel(x, y, Luma([255u8]));
        // Bounding box
        if x < region_corner1.0 {
            region_corner1.0 = x;
        }
        if y < region_corner1.1 {
            region_corner1.1 = y;
        }
        if x > region_corner2.0 {
            region_corner2.0 = x;
        }
        if y > region_corner2.1 {
            region_corner2.1 = y;
        }

        // Explore the 4-connected neighbors (up, down, left, right)
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let (nx, ny) = ((x as i32 + dx) as u32, (y as i32 + dy) as u32);
            if nx >= corner1.0 && ny >= corner1.1 && nx < corner2.0 && ny < corner2.1 {
                if !visited[nx as usize][ny as usize] && mask.get_pixel(nx, ny)[0] > 0 {
                    visited[nx as usize][ny as usize] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }

    Character::new(mask_out, region_corner1, region_corner2)
}

fn swap_if_needed(
    char1: Character,
    char2: Character,
    histogram1: &HashMap<Rgb<u8>, f64>,
    histogram2: &HashMap<Rgb<u8>, f64>,
    frame: &RgbImage,
) -> (Character, Character) {
    // Character1
    let (mut count1, mut count2) = (0, 0);
    for (x, y, present) in char1.mask.enumerate_pixels() {
        if present[0] > 0 {
            let pixel = frame.get_pixel(x, y);
            if histogram1.contains_key(pixel) && histogram2.contains_key(pixel) {
                count1 += if histogram1[pixel] > histogram2[pixel] {
                    1
                } else {
                    0
                };
                count2 += if histogram2[pixel] > histogram1[pixel] {
                    1
                } else {
                    0
                };
            }
        }
    }

    // If char1 matches better with histogram2,
    // we need to swap characters.
    if count2 > count1 {
        (char2, char1)
    } else {
        (char1, char2)
    }
}

fn merge_chars_masks(char1_mask: &GrayImage, char2_mask: &GrayImage) -> GrayImage {
    let mut mask = GrayImage::new(char1_mask.width(), char2_mask.height());
    for (x, y, pixel) in mask.enumerate_pixels_mut() {
        let pixel1 = char1_mask.get_pixel(x, y)[0];
        let pixel2 = char2_mask.get_pixel(x, y)[0];
        let value = if pixel1 > 0 || pixel2 > 0 { 255 } else { 0 };
        *pixel = Luma([value]);
    }
    mask
}

fn draw_framed_disjoint_chars(char1: &Character, char2: &Character) -> RgbImage {
    let mut img = RgbImage::new(char1.mask.width(), char1.mask.height());

    // Draw char1 in red
    for (x, y, pixel) in char1.mask.enumerate_pixels() {
        if pixel[0] > 0 {
            img.put_pixel(x, y, Rgb([255, 0, 0]));
        }
    }
    for x in char1.corner1.0..char1.corner2.0 {
        img.put_pixel(x, char1.corner1.1, Rgb([255, 0, 0]));
        img.put_pixel(x, char1.corner2.1, Rgb([255, 0, 0]));
    }
    for y in char1.corner1.1..char1.corner2.1 {
        img.put_pixel(char1.corner1.0, y, Rgb([255, 0, 0]));
        img.put_pixel(char1.corner2.0, y, Rgb([255, 0, 0]));
    }

    // Draw char2 in blue
    for (x, y, pixel) in char2.mask.enumerate_pixels() {
        if pixel[0] > 0 {
            img.put_pixel(x, y, Rgb([0, 0, 255]));
        }
    }
    for x in char2.corner1.0..char2.corner2.0 {
        img.put_pixel(x, char2.corner1.1, Rgb([0, 0, 255]));
        img.put_pixel(x, char2.corner2.1, Rgb([0, 0, 255]));
    }
    for y in char2.corner1.1..char2.corner2.1 {
        img.put_pixel(char2.corner1.0, y, Rgb([0, 0, 255]));
        img.put_pixel(char2.corner2.0, y, Rgb([0, 0, 255]));
    }

    img
}

fn draw_framed_overlapped_chars(char1: &Character, char2: &Character) -> RgbImage {
    let mut img = RgbImage::new(char1.mask.width(), char1.mask.height());

    let corner1 = (
        cmp::min(char1.corner1.0, char2.corner1.0),
        cmp::min(char1.corner1.1, char2.corner1.1),
    );
    let corner2 = (
        cmp::max(char1.corner2.0, char2.corner2.0),
        cmp::max(char1.corner2.1, char2.corner2.1),
    );

    // Draw whole thing in purple
    for x in corner1.0..corner2.0 {
        for y in corner1.1..corner2.1 {
            if char1.mask.get_pixel(x, y)[0] > 0 || char2.mask.get_pixel(x, y)[0] > 0 {
                img.put_pixel(x, y, Rgb([255, 0, 255]));
            }
        }
    }
    for x in corner1.0..corner2.0 {
        img.put_pixel(x, corner1.1, Rgb([255, 0, 255]));
        img.put_pixel(x, corner2.1, Rgb([255, 0, 255]));
    }
    for y in corner1.1..corner2.1 {
        img.put_pixel(corner1.0, y, Rgb([255, 0, 255]));
        img.put_pixel(corner2.0, y, Rgb([255, 0, 255]));
    }

    img
}

fn draw_disjoint_chars(char1: &Character, char2: &Character) -> RgbImage {
    let mut img = RgbImage::new(char1.mask.width(), char1.mask.height());

    // Draw char1 in red
    for (x, y, pixel) in char1.mask.enumerate_pixels() {
        if pixel[0] > 0 {
            img.put_pixel(x, y, Rgb([255, 0, 0]));
        }
    }

    // Draw char2 in blue
    for (x, y, pixel) in char2.mask.enumerate_pixels() {
        if pixel[0] > 0 {
            img.put_pixel(x, y, Rgb([0, 0, 255]));
        }
    }

    img
}

pub fn segment_overlapped_chars_by_histogram(
    img: &RgbImage,
    corner1: &(u32, u32),
    corner2: &(u32, u32),
    histogram1: &HashMap<Rgb<u8>, f64>,
    histogram2: &HashMap<Rgb<u8>, f64>,
) -> RgbImage {
    let mut img_out = RgbImage::new(img.width(), img.height());
    for x in corner1.0..corner2.0 {
        for y in corner1.1..corner2.1 {
            let pixel = img.get_pixel(x, y);
            if pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0 {
                if !histogram1.contains_key(pixel) {
                    img_out.put_pixel(x, y, Rgb([0, 0, 255]));
                } else if !histogram2.contains_key(pixel) {
                    img_out.put_pixel(x, y, Rgb([255, 0, 0]));
                } else {
                    if histogram1[pixel] > histogram2[pixel] {
                        img_out.put_pixel(x, y, Rgb([255, 0, 0]));
                    } else {
                        img_out.put_pixel(x, y, Rgb([0, 0, 255]));
                    }
                }
            }
        }
    }
    img_out
}

fn update_probabilities(
    char: &Character,
    img: &RgbImage,
    char_pixel_probability: &mut HashMap<Rgb<u8>, (u64, u64)>,
) {
    for (x, y, pixel) in img.enumerate_pixels() {
        let pixel = img.get_pixel(x, y);
        if !char_pixel_probability.contains_key(pixel) {
            char_pixel_probability.insert(pixel.clone(), (0, 0));
        }
        let (count, total) = &mut char_pixel_probability.get_mut(pixel).unwrap();
        *count += if char.mask.get_pixel(x, y)[0] > 0 {
            1
        } else {
            0
        };
        *total += 1;
    }
}

fn segment_by_probability(
    img: &RgbImage,
    char1_pixel_probability: &HashMap<Rgb<u8>, (u64, u64)>,
    char2_pixel_probability: &HashMap<Rgb<u8>, (u64, u64)>,
    char1_prob_threshold: f64,
    char2_prob_threshold: f64,
) -> RgbImage {
    let mut segmented_img = RgbImage::new(img.width(), img.height());
    for (x, y, pixel) in img.enumerate_pixels() {
        let pixel = img.get_pixel(x, y);

        // Char1
        let (count, total) = if char1_pixel_probability.contains_key(pixel) {
            char1_pixel_probability[pixel]
        } else {
            (0, 1)
        };
        let prob1 = count as f64 / total as f64;

        // Char2
        let (count, total) = if char2_pixel_probability.contains_key(pixel) {
            char2_pixel_probability[pixel]
        } else {
            (0, 1)
        };
        let prob2 = count as f64 / total as f64;

        if (prob1 > char1_prob_threshold) && (prob2 > char2_prob_threshold) {
            segmented_img.put_pixel(x, y, Rgb([255, 0, 255]));
        } else if prob1 > char1_prob_threshold {
            segmented_img.put_pixel(x, y, Rgb([255, 0, 0]));
        } else if prob2 > char2_prob_threshold {
            segmented_img.put_pixel(x, y, Rgb([0, 0, 255]));
        }
    }

    segmented_img
}
