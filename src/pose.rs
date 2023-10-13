//extern crate opencv;
use std::env;

use opencv::{
    core::{Mat, VecN, Scalar, Size, CV_8U, CV_32F, CV_8UC3},
    dnn::{read_net_from_caffe, Target},
    highgui::{imshow, wait_key},
    imgcodecs::{imread, imwrite},
    imgproc::{cvt_color, COLOR_BGR2RGB},
    prelude::*,
    types::{VectorOfString, VectorOfi32},
};

fn main() -> opencv::Result<()> {
    // Load the pre-trained human pose estimation model
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        panic!("Usage: {} <config> <model> <image>", args[0]);
    }

    let config_path = &args[1]; // Replace with the actual path to your config file
    let model_path = &args[2]; // Replace with the actual path to your model file
    let mut net = read_net_from_caffe(&config_path, &model_path)?;
    net.set_preferable_target(Target::DNN_TARGET_CPU as i32);

    // Load an image for pose estimation
    let image_path = &args[3]; // Replace with the path to your input image
    let mut in_image = imread(image_path, opencv::imgcodecs::IMREAD_COLOR)?;
    let mut image = Mat::default();

    // Convert the image to the appropriate format (BGR to RGB)
    //cvt_color(&mut in_image, &mut image, COLOR_BGR2RGB, 0)?;

    imshow("Test", &in_image);
    wait_key(1000);
    let image = in_image;

    // Prepare the image for input to the neural network
    //let image = Mat::new_rows_cols_with_default(10, 10, CV_8UC3, Scalar::all(0.0)).unwrap();
    println!("Image is {}x{}", image.cols(), image.rows());
    let size = Size {
        width: image.cols(),
        height: image.rows(),
    };
    let blob = opencv::dnn::blob_from_image(
        &image,
        1.0,
        size,
        VecN::new(0.0, 0.0, 0.0, 0.0),
        false,
        false,
        CV_8U,
    )?;

    // Set the input for the network
    net.set_input(&blob, "image", 1.0, Scalar::default())?;

    // Forward pass to perform pose estimation
    let mut output_blobs = Mat::default();
    let mut out_blob_names = VectorOfString::new();
    out_blob_names.push("net_output");
    let output = net.forward_single("net_output");
    println!(
        "Dims: {} {}x{}",
        output_blobs.dims(),
        output_blobs.rows(),
        output_blobs.cols()
    );

    match output {
        Ok(value) => {
            println!("Result is valid");
            imshow("Result!", &value);
            wait_key(1000);
            println!("{}", value.dims());
            let mat_size = value.mat_size();
            println!("size= {}", mat_size.len());
            println!("{}x{}x{}x{}", mat_size[0], mat_size[1], mat_size[2], mat_size[3]);
            for i in 0..25 {
                let title = format!("index_{}", i);
                unsafe {
                    let mut mat = Mat::new_rows_cols(mat_size[2], mat_size[3], CV_8U).unwrap();
                    for row in 0..mat_size[2] {
                        for col in 0..mat_size[3] {
                            let val = value.at_nd::<f32>(&[0, i, row, col]).unwrap();
                            *mat.at_2d_mut::<u8>(row, col).unwrap() = (*val * 255.0) as u8;
                        }
                    }
                    imshow(&title, &mat);
                    wait_key(1000);
                    let path = "pose_keypoints/".to_owned() + &title + ".png";
                    imwrite(&path, &mat, &VectorOfi32::new()).expect("Failed to save image");
                }
            }
        }
        Err(error) => {
            println!("Result is invalid: {}", error);
        }
    }


    // Process the output to extract human pose information
    // You can access and analyze the pose keypoints from the 'output' here

    //imshow("Result!", &output_blobs);
    //wait_key(1000);

    // Perform any further processing or visualization as needed

    Ok(())
}
