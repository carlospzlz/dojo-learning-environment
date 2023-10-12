//extern crate opencv;
use std::env;

use opencv::{
    core::{Mat, Scalar, CV_8U},
    dnn::{read_net_from_caffe, Dict},
    imgcodecs::imread,
    imgproc::{COLOR_BGR2RGB, cvt_color},
    prelude::*,
    types::{VectorOfString},
};

fn main() -> opencv::Result<()> {
    // Load the pre-trained human pose estimation model
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Usage: {} <model> <config>", args[0]);
    }

    let model_path = &args[0]; // Replace with the actual path to your model file
    let config_path = &args[1]; // Replace with the actual path to your config file
    let mut net = read_net_from_caffe(&model_path, &config_path)?;

    // Load an image for pose estimation
    let image_path = "frames/yoshimitsu_vs_lei.png"; // Replace with the path to your input image
    let mut in_image = imread(image_path, opencv::imgcodecs::IMREAD_COLOR)?;
    let mut image = Mat::default();

    // Convert the image to the appropriate format (BGR to RGB)
    cvt_color(&mut in_image, &mut image, COLOR_BGR2RGB, 0)?;

    // Prepare the image for input to the neural network
    let blob = opencv::dnn::blob_from_image(&image, 1.0, Default::default(), Scalar::default(), false, false, CV_8U)?;

    // Set the input for the network
    net.set_input(&blob, "image", 1.0, Scalar::default())?;

    // Forward pass to perform pose estimation
    let mut output_blobs = Mat::default();
    let mut out_blob_names = VectorOfString::new();
    out_blob_names.push("net_output");
    let output = net.forward(&mut output_blobs, &out_blob_names);

    // Process the output to extract human pose information
    // You can access and analyze the pose keypoints from the 'output' here

    // Perform any further processing or visualization as needed

    Ok(())
}
