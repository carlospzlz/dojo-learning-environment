// Dojo Learning Environment
// Copyright (C) 2023-2024 Carlos Perez-Lopez
//
// This project is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>
//
// You can contact the author via carlospzlz@gmail.com

use egui::{Color32, ColorImage, RichText, Vec2};
use egui_file::FileDialog;
use image::{Rgb, RgbImage};
use log::error;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};

// Emu system
mod psx;

use psx::System;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`)
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        error!("Usage: {} <bios> <game>", args[0]);
        return Ok(());
    }
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(480.0, 460.0)),
        ..Default::default()
    };
    eframe::run_native(
        "PSX GUI",
        options,
        Box::new(move |cc| {
            let bios = args[1].clone();
            let game = args[2].clone();
            Box::new(MyApp::new(cc, bios, game))
        }),
    )
}

struct MyApp {
    bios: String,
    game: String,
    system: System,
    is_running: bool,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
    saved_file: Option<PathBuf>,
    save_file_dialog: Option<FileDialog>,
}

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>, bios: String, game: String) -> Self {
        // Make game path absolute, so state can be loaded from anywhere
        let game_path = match fs::canonicalize(Path::new(&game)) {
            Ok(game_path) => game_path,
            Err(e) => {
                panic!("Error resolving to absolute path: {}: {}", game, e);
            }
        };
        let mut system = System::new(&bios, &game_path.to_string_lossy());
        system.reset();
        Self {
            bios,
            game,
            system,
            is_running: true,
            opened_file: None,
            open_file_dialog: None,
            saved_file: None,
            save_file_dialog: None,
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
            // Adjust so other panels don't occlude it
            let bottom_panel_height = 110;
            let new_width = asize[0].round() as u32;
            let new_height = asize[1].round() as u32 - bottom_panel_height;

            // Load texture
            //let img = ColorImage::from_rgb([width, height], &framebuffer);
            let img = image::imageops::resize(
                &img,
                new_width,
                new_height,
                image::imageops::FilterType::Lanczos3,
            );
            let img = ColorImage::from_rgb([new_width as usize, new_height as usize], img.as_raw());
            let texture = ctx.load_texture("psx_screen", img, Default::default());

            // Show frame
            ui.horizontal(|ui| {
                ui.image(&texture, texture.size_vec2());
            });
        });

        egui::TopBottomPanel::bottom("my_bottom_panel").show(ctx, |ui| {
            let asize = ui.available_size();
            let available_width = asize[0];
            ui.horizontal(|ui| {
                // Emulator Controls
                if ui.button("Start").clicked() {
                    self.is_running = true;
                }
                if ui.button("Stop").clicked() {
                    self.is_running = false;
                }
                if ui.button("Next").clicked() {
                    if !self.is_running {
                        self.system.run_frame();
                    }
                }
                if ui.button("Reset").clicked() {
                    self.system.reset();
                }
                if ui.button("Hard Reset").clicked() {
                    self.system = System::new(&self.bios, &self.game);
                    self.system.reset();
                }
                // File Controls
                if ui.button("Load").clicked() {
                    // If user is about to load, probably stop emu
                    self.is_running = false;
                    let dialog = FileDialog::open_file(self.opened_file.clone());
                    let dialog = dialog.title("Load State");
                    let mut dialog = dialog.default_size(Vec2 { x: 300.0, y: 200.0 });
                    dialog.open();
                    self.open_file_dialog = Some(dialog);
                }
                if ui.button("Save").clicked() {
                    // Stop emu, to save the current state
                    self.is_running = false;
                    let dialog = FileDialog::save_file(self.saved_file.clone());
                    let dialog = dialog.title("Save State");
                    let mut dialog = dialog.default_size(Vec2 { x: 300.0, y: 200.0 });
                    dialog.open();
                    self.save_file_dialog = Some(dialog);
                }
                let emu_controls_width = 350.0;
                let space = available_width - emu_controls_width;
                let space = space.max(0.0);
                ui.add_space(space);
                if self.is_running {
                    ui.label(RichText::new("⏺").color(Color32::LIGHT_GREEN));
                } else {
                    ui.label(RichText::new("⏺").color(Color32::GRAY));
                }
            });
            //ui.horizontal(|_ui| {});
            let controller_half_size = 50.0;
            ui.horizontal(|ui| {
                // Virtual Controller
                ui.add_space(available_width / 2.0 - controller_half_size + 14.0);
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
                ui.add_space(available_width / 2.0 - controller_half_size);
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
                if ui.button("○").clicked() {
                    self.system.get_controller().button_circle = true;
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(available_width / 2.0 - controller_half_size + 14.0);
                if ui.button("⏷").clicked() {
                    self.system.get_controller().button_dpad_down = true;
                }
                ui.add_space(29.0);
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(124, 178, 232));
                if ui.button("🗙").clicked() {
                    self.system.get_controller().button_cross = true;
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(available_width / 2.0 - controller_half_size);
                if ui.button("SELECT").clicked() {
                    self.system.get_controller().button_select = true;
                }
                if ui.button("START").clicked() {
                    self.system.get_controller().button_start = true;
                }
            });
        });
        // File dialogs
        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    let filepath = file.to_str().unwrap();
                    println!("Loading {} ...", filepath);
                    let mut bytes = Vec::new();
                    let mut file = File::open(&filepath).unwrap();
                    let _ = file.read_to_end(&mut bytes).unwrap();
                    // 'bios' and 'game' filepaths will come from the state
                    self.system = bincode::deserialize(&bytes).unwrap();
                    self.is_running = true;
                }
            }
        }
        if let Some(dialog) = &mut self.save_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    let filepath = file.to_str().unwrap();
                    println!("Saving {} ...", filepath);
                    match File::create(&filepath) {
                        Ok(mut file) => {
                            let bytes = bincode::serialize(&self.system).unwrap();
                            let _ = file.write_all(&bytes).unwrap();
                            self.is_running = true;
                        }
                        Err(err) => {
                            error!("{}", err);
                        }
                    }
                }
            }
        }

        // Processing
        if self.is_running {
            self.system.run_frame();
            ctx.request_repaint();
        }

        // Reset controller
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
