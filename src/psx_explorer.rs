use egui::{Color32, RichText, ColorImage};
use std::sync::{Arc, Mutex};
use std::thread;
use image::{RgbImage, Rgb};


// Emu system
mod gpu_viewer;
mod psx;
mod queue;
mod util;

use psx::System;

fn main() -> () {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`)
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1000.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "RPSX GUI",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    );
}

struct MyApp {
    system: System,
    is_running: bool,
    next_frame: bool,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let bios_filepath = "bios/scph1001.bin";
        let game_filepath = "roms/tekken.bin";
        let mut system = System::new(&bios_filepath, &game_filepath);
        // Register callbacks here
        system.reset();
        Self {
            system,
            is_running: true,
            next_frame: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {
            // Get frame buffer
            let (width, height) = self.system.get_display_size();
            let (width, height) = (width as usize, height as usize);
            let mut framebuffer = vec![0; width * height * 3].into_boxed_slice();
            self.system.get_framebuffer(&mut framebuffer, false);

            // Scale up
            let mut img = RgbImage::new(width as u32, height as u32);
            for (x, y, pixel) in img.enumerate_pixels_mut() {
                let offset = ((y as u32 * width as u32 + x as u32) * 3) as usize;
                let r = framebuffer[offset];
                let g = framebuffer[offset + 1];
                let b = framebuffer[offset + 2];
                *pixel = Rgb([r, g, b]);
            }

            let asize = ui.available_size();
            // Adjust so other panels don't include it
            let new_width = asize[0].round() as u32 - 230;
            let new_height = asize[1].round() as u32 - 30;

            // Load texture
            //let img = ColorImage::from_rgb([width, height], &framebuffer);
            let img = image::imageops::resize(&img, new_width, new_height, image::imageops::FilterType::Lanczos3);
            let img = ColorImage::from_rgb([new_width as usize, new_height as usize], img.as_raw());
            let texture = ctx.load_texture(
                    "psx_screen",
                    img,
                    Default::default()
            );

            // Show frame
            ui.horizontal(|ui| {
                ui.add_space(113.0);
                ui.image(&texture, texture.size_vec2());
            });

            ctx.request_repaint();
        });

        egui::TopBottomPanel::bottom("my_bottom_panel").show(ctx, |ui| {
            ui.label("Debug info");
        });

        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            if ui.button("Start").clicked() {
                self.is_running = true;
            }
            if ui.button("Stop").clicked() {
                self.is_running = false;
            }
            if ui.button("Next").clicked() {
                self.next_frame = true;
            }
            // File Controls
            ui.button("Load");
            ui.button("Save");
        });
        egui::SidePanel::right("my_right_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                //if self.system.is_on() {
                //    ui.label(RichText::new("⚡").color(Color32::YELLOW));
                //} else {
                //    ui.label(RichText::new("⚡").color(Color32::GRAY));
                //}
                if self.is_running {
                    ui.label(RichText::new("⏺").color(Color32::LIGHT_GREEN));
                } else {
                    ui.label(RichText::new("⏺").color(Color32::GRAY));
                }
            });
            ui.horizontal(|ui| {});
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if ui.button("⏶").clicked() {
                    self.system.get_controller().button_dpad_up = true;
                }
                ui.add_space(30.0);
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(64, 226, 160));
                if ui.button("∆").clicked() {
                    self.system.get_controller().button_triangle = true;
                }
            });
            ui.horizontal(|ui| {
                if ui.button("⏴").clicked() {
                    self.system.get_controller().button_dpad_left = true;
                }
                if ui.button("⏵").clicked() {
                    self.system.get_controller().button_dpad_right = true;
                }
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(255, 105, 248));
                if ui.button("◻").clicked() {
                    self.system.get_controller().button_square = true;
                }
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(255, 102, 102));
                if ui.button("○").clicked()
                {
                    self.system.get_controller().button_circle = true;
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if ui.button("⏷").clicked() {
                    self.system.get_controller().button_dpad_down = true;
                }
                ui.add_space(29.0);
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(124, 178, 232));
                if ui.button("🗙").clicked() {
                    self.system.get_controller().button_cross = true;
                }
            });
            ui.horizontal(|ui| {});
            ui.horizontal(|ui| {
                if ui.button("SELECT").clicked() {
                    self.system.get_controller().button_select = true;
                }
                if ui.button("START").clicked() {
                    self.system.get_controller().button_start = true;
                }
            });
        });

        // Processing
        if self.is_running {
            self.system.run_frame();
        } else if self.next_frame {
            self.system.run_frame();
            self.next_frame = false;
        }
        self.system.get_controller().button_dpad_up = false;
        self.system.get_controller().button_dpad_down = false;
        self.system.get_controller().button_dpad_left = false;
        self.system.get_controller().button_dpad_right = false;
        self.system.get_controller().button_triangle = false;
        self.system.get_controller().button_square = false;
        self.system.get_controller().button_circle = false;
        self.system.get_controller().button_cross = false;
        self.system.get_controller().button_start = false;
        self.system.get_controller().button_select = false;
    }
}
