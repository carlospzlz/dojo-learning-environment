use egui::{Color32, ColorImage};
use image::{Rgb, RgbImage};
use log::error;
use std::env;
use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};

// Utils to "see" the screen
mod vision;
// Emu system
mod psx;
// AI agent
mod q_learning;

use psx::System;
use q_learning::Agent;
use vision::LifeInfo;

const SIDE_PANEL_WIDTH: f32 = 170.0;
const STATES_DIR: &str = "states";
const REPLAY_DURATION: Duration = Duration::from_secs(2);

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
    Eddy,
    Jin,
    King,
    Lei,
    Paul,
    Yoshimitsu,
    Random,
}

#[derive(Debug, PartialEq)]
enum Vision {
    PSX,
    Life,
    Agent,
}

struct MyApp {
    #[allow(dead_code)]
    bios: String,
    #[allow(dead_code)]
    game: String,
    system: System,
    frame: RgbImage,
    is_running: bool,
    vision: Vision,
    character1: Character,
    character2: Character,
    current_combat: Option<[Character; 2]>,
    agent_life_info: LifeInfo,
    opponent_life_info: LifeInfo,
    replay: Option<std::time::Duration>,
    agent: Agent,
    observation_frequency: u32,
    time_from_last_observation: std::time::Duration,
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
            vision: Vision::PSX,
            character1: Character::Yoshimitsu,
            character2: Character::Lei,
            current_combat: None,
            agent_life_info: LifeInfo::default(),
            opponent_life_info: LifeInfo::default(),
            replay: None,
            agent: Agent::new(),
            observation_frequency: 1,
            time_from_last_observation: Duration::from_secs(1),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.bottom_panel(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.central_panel(ctx);

        // Processing
        if self.is_running {
            self.process_frame();

            // Request repaint
            ctx.request_repaint()
        }
    }
}

impl MyApp {
    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut img = self.frame.clone();
            match self.vision {
                Vision::Agent => img = self.agent.get_state(),
                Vision::Life => img = vision::visualize_life_bars(img),
                Vision::PSX => (),
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
                    ui.label("AI agent:");
                    egui::ComboBox::from_id_source("agent_character")
                        .selected_text(format!("{:?}", self.character1))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.character1, Character::Eddy, "Eddy");
                            ui.selectable_value(&mut self.character1, Character::Jin, "Jin");
                            ui.selectable_value(&mut self.character1, Character::King, "King");
                            ui.selectable_value(&mut self.character1, Character::Lei, "Lei");
                            ui.selectable_value(&mut self.character1, Character::Paul, "Paul");
                            ui.selectable_value(&mut self.character1, Character::Random, "Random");
                            ui.selectable_value(
                                &mut self.character1,
                                Character::Yoshimitsu,
                                "Yoshimitsu",
                            );
                        });
                    ui.end_row();
                    ui.label("Opponent:");
                    egui::ComboBox::from_id_source("opponent_character")
                        .selected_text(format!("{:?}", self.character2))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.character2, Character::Eddy, "Eddy");
                            ui.selectable_value(&mut self.character2, Character::Jin, "Jin");
                            ui.selectable_value(&mut self.character2, Character::King, "King");
                            ui.selectable_value(&mut self.character2, Character::Lei, "Lei");
                            ui.selectable_value(&mut self.character2, Character::Paul, "Paul");
                            ui.selectable_value(&mut self.character2, Character::Random, "Random");
                            ui.selectable_value(
                                &mut self.character2,
                                Character::Yoshimitsu,
                                "Yoshimitsu",
                            );
                        });
                    ui.end_row();
                    ui.label("Obs Freq (Hz):");
                    ui.add(egui::DragValue::new(&mut self.observation_frequency).speed(0.1));
                    ui.end_row();
                    ui.label("Vision");
                    egui::ComboBox::from_id_source("vision")
                        .selected_text(format!("{:?}", self.vision))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.vision, Vision::PSX, "PSX");
                            ui.selectable_value(&mut self.vision, Vision::Life, "Life");
                            ui.selectable_value(&mut self.vision, Vision::Agent, "Agent");
                        });
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
                            self.process_frame();
                            ctx.request_repaint();
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
        // TODO: Set bios and game
        self.system = bincode::deserialize(&bytes).unwrap();
        self.current_combat = Some([self.character1.clone(), self.character2.clone()]);
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("my_right_panel")
            .exact_width(SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Life Stats");
                    let separator = egui::Separator::default();
                    ui.add(separator.horizontal());
                });
                egui::Grid::new("life_stats").show(ui, |ui| {
                    ui.label("Life:");
                    ui.label(format!("{:.4}", self.agent_life_info.life));
                    ui.label(format!("{:.4}", self.opponent_life_info.life));
                    ui.end_row();
                    ui.label("Damage:");
                    ui.label(format!("{:.4}", self.agent_life_info.damage));
                    ui.label(format!("{:.4}", self.opponent_life_info.damage));
                    ui.end_row();
                });
                ui.horizontal(|_ui| {});
                ui.horizontal(|ui| {
                    ui.label("AI Agent");
                    let separator = egui::Separator::default();
                    ui.add(separator.horizontal());
                });
            });
    }

    fn process_frame(&mut self) {
        // Run frame
        let start_time = Instant::now();
        self.run_frame();
        let delta_time = Instant::now() - start_time;
        if self.replay.is_some() {
            self.update_replay(&delta_time);
            return;
        }

        // Get life info
        let lifes_info = vision::get_life_info(self.frame.clone());
        self.agent_life_info = lifes_info.0;
        self.opponent_life_info = lifes_info.1;

        // Check for end of combat
        if self.agent_life_info.life == 0.0 || self.opponent_life_info.life == 0.0 {
            println!("End of combat");
            self.replay = Some(Duration::ZERO);
            return;
        }

        self.reset_controller();

        // Feed AI agent
        if self.observation_frequency == 0 {
            return;
        }
        self.time_from_last_observation += delta_time;
        let period = Duration::from_secs_f32(1.0 / self.observation_frequency as f32);
        if self.time_from_last_observation > period {
            self.agent.add_state(self.frame.clone());
            self.time_from_last_observation = Duration::ZERO;
        }
    }

    fn run_frame(&mut self) {
        self.system.run_frame();
        // Get frame buffer
        let (width, height) = self.system.get_display_size();
        let mut framebuffer = vec![0; width as usize * height as usize * 3].into_boxed_slice();
        self.system.get_framebuffer(&mut framebuffer, false);
        self.frame = convert_framebuffer_to_rgb_image(&framebuffer, width, height);
    }

    fn update_replay(&mut self, delta_time: &Duration) {
        // Show for a certain duration and then load state
        let duration = self.replay.unwrap() + *delta_time;
        if duration > REPLAY_DURATION {
            self.replay = None;
            self.load_current_combat();
        } else {
            self.replay = Some(duration);
        }
    }

    fn reset_controller(&mut self) {
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
