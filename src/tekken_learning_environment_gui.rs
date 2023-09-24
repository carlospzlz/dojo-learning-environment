use egui::{Color32, ColorImage};
use egui_file::FileDialog;
use image::{Rgb, RgbImage};
use log::error;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

// Utils to "see" the screen
mod vision;
// Emu system
mod psx;

use psx::System;
use vision::LifeInfo;

const SIDE_PANEL_WIDTH: f32 = 170.0;
const STATES_DIR: &str = "states";

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`)
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        error!("Usage: {} <bios> <game>", args[0]);
        return Ok(());
    }
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(900.0, 480.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Tekken Learning Environment",
        options,
        Box::new(move |cc| {
            let bios = args[1].clone();
            let game = args[2].clone();
            Box::new(MyApp::new(cc, bios, game))
        }),
    )
}

#[derive(Clone, Debug, PartialEq)]
enum Character {
    Lei,
    Paul,
    Yoshimitsu,
    Random,
}

struct MyApp {
    bios: String,
    game: String,
    system: System,
    frame: RgbImage,
    is_running: bool,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
    saved_file: Option<PathBuf>,
    save_file_dialog: Option<FileDialog>,
    vision: bool,
    character1: Character,
    character2: Character,
    current_combat: Option<[Character; 2]>,
    ai_life_info: LifeInfo,
    opponent_life_info: LifeInfo,
}

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>, bios: String, game: String) -> Self {
        let mut system = System::new(&bios, &game);
        system.reset();
        Self {
            bios,
            game,
            system,
            frame: RgbImage::default(),
            is_running: false,
            opened_file: None,
            open_file_dialog: None,
            saved_file: None,
            save_file_dialog: None,
            vision: false,
            character1: Character::Yoshimitsu,
            character2: Character::Lei,
            current_combat: None,
            ai_life_info: LifeInfo::default(),
            opponent_life_info: LifeInfo::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.bottom_panel(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.central_panel(ctx);

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
            self.process_frame();
            ctx.request_repaint()
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

impl MyApp {
    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            //// Get frame buffer
            //let (width, height) = self.system.get_display_size();
            //let (width, height) = (width as usize, height as usize);
            //let mut framebuffer = vec![0; width * height * 3].into_boxed_slice();
            //self.system.get_framebuffer(&mut framebuffer, false);

            //// Scale up
            //let mut img = RgbImage::new(width as u32, height as u32);
            //for (x, y, pixel) in img.enumerate_pixels_mut() {
            //    let offset = ((y as u32 * width as u32 + x as u32) * 3) as usize;
            //    let r = framebuffer[offset];
            //    let g = framebuffer[offset + 1];
            //    let b = framebuffer[offset + 2];
            //    *pixel = Rgb([r, g, b]);
            //}

            let mut img = self.frame.clone();
            if self.vision {
                img = vision::visualize_life_bars(img);
            }

            // Fill all available space
            let asize = ui.available_size();
            let new_width = asize[0].round() as u32;
            let new_height = asize[1].round() as u32;

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
    }

    fn bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("my_bottom_panel").show(ctx, |ui| {
            let asize = ui.available_size();
            let available_width = asize[0];
            //ui.horizontal(|ui| {
            //    // Emulator Controls
            //    if ui.button("Start").clicked() {
            //        self.is_running = true;
            //    }
            //    if ui.button("Stop").clicked() {
            //        self.is_running = false;
            //    }
            //    if ui.button("Next").clicked() {
            //        if !self.is_running {
            //            self.system.run_frame();
            //        }
            //    }
            //    if ui.button("Reset").clicked() {
            //        self.system.reset();
            //    }
            //    if ui.button("Hard Reset").clicked() {
            //        self.system = System::new(&self.bios, &self.game);
            //        self.system.reset();
            //    }
            //    // File Controls
            //    if ui.button("Load").clicked() {
            //        // If user is about to load, probably stop emu
            //        self.is_running = false;
            //        let dialog = FileDialog::open_file(self.opened_file.clone());
            //        let dialog = dialog.title("Load State");
            //        let mut dialog = dialog.default_size(Vec2 { x: 300.0, y: 200.0 });
            //        dialog.open();
            //        self.open_file_dialog = Some(dialog);
            //    }
            //    if ui.button("Save").clicked() {
            //        // Stop emu, to save the current state
            //        self.is_running = false;
            //        let dialog = FileDialog::save_file(self.saved_file.clone());
            //        let dialog = dialog.title("Save State");
            //        let mut dialog = dialog.default_size(Vec2 { x: 300.0, y: 200.0 });
            //        dialog.open();
            //        self.save_file_dialog = Some(dialog);
            //    }
            //    ui.checkbox(&mut self.vision, "Vision");
            //    let emu_controls_width = 410.0;
            //    let space = available_width - emu_controls_width;
            //    let space = space.max(0.0);
            //    ui.add_space(space);
            //    if self.is_running {
            //        ui.label(RichText::new("⏺").color(Color32::LIGHT_GREEN));
            //    } else {
            //        ui.label(RichText::new("⏺").color(Color32::GRAY));
            //    }
            //});
            ////ui.horizontal(|_ui| {});
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
            // Try grid here
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("my_left_panel")
            .default_width(SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                // General
                ui.horizontal(|ui| {
                    ui.label("General");
                    let separator = egui::Separator::default();
                    ui.add(separator.horizontal());
                });
                egui::Grid::new("general_options").show(ui, |ui| {
                    ui.label("AI agent");
                    egui::ComboBox::from_id_source("ai_character")
                        .selected_text(format!("{:?}", self.character1))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.character1, Character::Lei, "Lei");
                            ui.selectable_value(
                                &mut self.character1,
                                Character::Yoshimitsu,
                                "Yoshimitsu",
                            );
                            ui.selectable_value(&mut self.character1, Character::Paul, "Paul");
                        });
                    ui.end_row();
                    ui.label("Opponent");
                    egui::ComboBox::from_id_source("psx_character")
                        .selected_text(format!("{:?}", self.character2))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.character2,
                                Character::Yoshimitsu,
                                "Yoshimitsu",
                            );
                            ui.selectable_value(&mut self.character2, Character::Paul, "Paul");
                        });
                    ui.end_row();
                    ui.label("Vision");
                    ui.checkbox(&mut self.vision, "");
                });
                ui.horizontal(|_ui| {});

                // Reinforcement Learning
                ui.horizontal(|ui| {
                    ui.label("Reinforcement Learning");
                    let separator = egui::Separator::default();
                    ui.add(separator.horizontal());
                });
                ui.horizontal(|ui| {
                    // Emulator Controls
                    if ui.button("Start").clicked() {
                        if self.current_combat.is_none() {
                            self.load_current_combat();
                        }
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
                });
                ui.horizontal(|_ui| {});

                // Simulation
                ui.horizontal(|ui| {
                    ui.label("Simulation");
                    let separator = egui::Separator::default();
                    ui.add(separator.horizontal());
                });
            });
    }

    fn load_current_combat(&mut self) {
        let name1 = format!("{:?}", self.character1).to_lowercase();
        let name2 = format!("{:?}", self.character2).to_lowercase();
        let filepath = format!("{}/{}_vs_{}.bin", STATES_DIR, name1, name2);
        println!("Loading {} ...", filepath);
        let mut bytes = Vec::new();
        let mut file = File::open(&filepath).unwrap();
        let _ = file.read_to_end(&mut bytes).unwrap();
        // 'bios' and 'game' filepaths will come from the state
        self.system = bincode::deserialize(&bytes).unwrap();
        self.current_combat = Some([self.character1.clone(), self.character2.clone()]);
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("my_right_panel")
            .exact_width(SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                egui::Grid::new("system_info").show(ui, |ui| {
                    ui.label("Life");
                    ui.label(format!("{:.4}", self.ai_life_info.life));
                    ui.label(format!("{:.4}", self.opponent_life_info.life));
                    ui.end_row();
                    ui.label("Damage");
                    ui.label(format!("{:.4}", self.ai_life_info.damage));
                    ui.label(format!("{:.4}", self.opponent_life_info.damage));
                    ui.end_row();
                });
                //ui.horizontal(|ui| {
                //    // Emulator Controls
                //    if ui.button("Start").clicked() {
                //        self.is_running = true;
                //    }
                //    if ui.button("Stop").clicked() {
                //        self.is_running = false;
                //    }
                //    if ui.button("Next").clicked() {
                //        if !self.is_running {
                //            self.system.run_frame();
                //        }
                //    }
                //});
            });
    }

    fn process_frame(&mut self) {
        self.system.run_frame();
        // Get frame buffer
        let (width, height) = self.system.get_display_size();
        let mut framebuffer = vec![0; width as usize * height as usize * 3].into_boxed_slice();
        self.system.get_framebuffer(&mut framebuffer, false);
        self.frame = convert_framebuffer_to_rgb_image(&framebuffer, width, height);
        // Get life info
        let lifes_info = vision::get_life_info(self.frame.clone());
        // Check for end of combat
        if lifes_info.0.life == 0.0 || lifes_info.1.life == 0.0 {
            println!("End of combat");
            self.load_current_combat();
            return;
        }
        // Feed AI agent
        self.ai_life_info = lifes_info.0;
        self.opponent_life_info = lifes_info.1;
    }
}

fn convert_framebuffer_to_rgb_image(framebuffer: &[u8], width: u32, height: u32) -> RgbImage {
    let mut img = RgbImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let offset = ((y as u32 * width + x) * 3) as usize;
        let r = framebuffer[offset];
        let g = framebuffer[offset + 1];
        let b = framebuffer[offset + 2];
        *pixel = Rgb([r, g, b]);
    }
    img
}
