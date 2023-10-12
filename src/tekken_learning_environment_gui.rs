use egui::{Color32, ColorImage};
use image::{DynamicImage, Rgb, RgbImage};
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
        initial_window_size: Some(egui::vec2(750.0, 550.0)),
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
    Law,
    Lei,
    Paul,
    Yoshimitsu,
    Xiaoyu,
    Random,
}

#[derive(Debug, PartialEq)]
enum Vision {
    PSX,
    Life,
    Agent,
    Contrast,
}

struct FrameTime {
    total_time: Duration,
    ui_time: Duration,
    psx_time: Duration,
    agent_time: Duration,
}

impl Default for FrameTime {
    fn default() -> Self {
        Self {
            total_time: Duration::ZERO,
            ui_time: Duration::ZERO,
            psx_time: Duration::ZERO,
            agent_time: Duration::ZERO,
        }
    }
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
    split_view: bool,
    character1: Character,
    character2: Character,
    current_combat: Option<[Character; 2]>,
    agent_life_info: LifeInfo,
    opponent_life_info: LifeInfo,
    replay: Option<std::time::Duration>,
    agent: Agent,
    observation_frequency: u32,
    time_from_last_observation: std::time::Duration,
    frame_time: FrameTime,
    learning_rate: f32,
    discount_factor: f32,
    red_thresholds: [u8; 2],
    green_thresholds: [u8; 2],
    blue_thresholds: [u8; 2],
    dilate_k: u8,
    max_mse: f32,
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
            vision: Vision::Agent,
            split_view: true,
            character1: Character::Yoshimitsu,
            character2: Character::Lei,
            current_combat: None,
            agent_life_info: LifeInfo::default(),
            opponent_life_info: LifeInfo::default(),
            replay: None,
            agent: Agent::new(),
            observation_frequency: 50,
            time_from_last_observation: Duration::from_secs(1),
            frame_time: FrameTime::default(),
            learning_rate: 0.5,
            discount_factor: 0.9,
            red_thresholds: [0, 173],
            green_thresholds: [15, 165],
            blue_thresholds: [15, 156],
            dilate_k: 6,
            max_mse: 0.04,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let start_time = Instant::now();
        self.bottom_panel(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.central_panel(ctx);
        self.frame_time.ui_time = Instant::now() - start_time;

        // Processing
        if self.is_running {
            self.process_frame();

            // Request repaint
            ctx.request_repaint()
        }
        self.frame_time.total_time = Instant::now() - start_time;
    }
}

impl MyApp {
    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Fill all available space
            let asize = ui.available_size();
            let new_width = asize[0].round() as u32;
            let new_height = if self.split_view {
                asize[1].round() / 2.0
            } else {
                asize[1].round()
            } as u32;

            // If split view, always show PSX view
            let img = self.frame.clone();
            if self.split_view {
                let img = DynamicImage::ImageRgb8(img);
                let img =
                    img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);
                let img = img.to_rgb8();
                let img =
                    ColorImage::from_rgb([new_width as usize, new_height as usize], img.as_raw());
                let texture = ctx.load_texture("psx_frame", img, Default::default());
                ui.image(&texture, texture.size_vec2());
            }

            // Show vision chosen by user
            let mut img = self.frame.clone();
            match self.vision {
                Vision::Agent => img = self.agent.get_last_state_abstraction(),
                Vision::Life => img = vision::visualize_life_bars(img),
                Vision::Contrast => {
                    img = vision::apply_thresholds(
                        &img,
                        self.red_thresholds,
                        self.green_thresholds,
                        self.blue_thresholds,
                    );
                }
                Vision::PSX => (),
            }

            let img = DynamicImage::ImageRgb8(img);
            let img =
                img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);
            let img = img.to_rgb8();

            // Load texture
            let img = ColorImage::from_rgb([new_width as usize, new_height as usize], img.as_raw());
            let texture = ctx.load_texture("psx_frame", img, Default::default());

            // Show frame
            ui.image(&texture, texture.size_vec2());
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
                let mut up_button = egui::Button::new("⏶");
                if self.system.get_controller().button_dpad_up {
                    up_button = up_button.fill(Color32::LIGHT_GREEN);
                }
                if ui.add(up_button).clicked() {
                    self.system.get_controller().button_dpad_up = true;
                }
                ui.add_space(30.0);
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(64, 226, 160));
                let mut triangle_button = egui::Button::new("∆");
                if self.system.get_controller().button_triangle {
                    triangle_button = triangle_button.fill(Color32::LIGHT_GREEN);
                }
                if ui.add(triangle_button).clicked() {
                    self.system.get_controller().button_triangle = true;
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(available_width / 2.0 - controller_half_size);
                // Left Arrow
                let mut left_button = egui::Button::new("⏴");
                if self.system.get_controller().button_dpad_left {
                    left_button = left_button.fill(Color32::LIGHT_GREEN);
                }
                if ui.add(left_button).clicked() {
                    self.system.get_controller().button_dpad_left = true;
                }
                // Right Arrow
                let mut right_button = egui::Button::new("⏵");
                if self.system.get_controller().button_dpad_right {
                    right_button = right_button.fill(Color32::LIGHT_GREEN);
                }
                if ui.add(right_button).clicked() {
                    self.system.get_controller().button_dpad_right = true;
                }
                // Square Button
                let mut square_button = egui::Button::new("◻");
                if self.system.get_controller().button_square {
                    square_button = square_button.fill(Color32::LIGHT_GREEN);
                }
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(255, 105, 248));
                if ui.add(square_button).clicked() {
                    self.system.get_controller().button_square = true;
                }
                // Circle Button
                let mut circle_button = egui::Button::new("○");
                if self.system.get_controller().button_circle {
                    circle_button = circle_button.fill(Color32::LIGHT_GREEN);
                }
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(255, 102, 102));
                if ui.add(circle_button).clicked() {
                    self.system.get_controller().button_circle = true;
                }
            });
            ui.horizontal(|ui| {
                // Down Arrow
                ui.add_space(available_width / 2.0 - controller_half_size + 14.0);
                let mut down_button = egui::Button::new("⏷");
                if self.system.get_controller().button_dpad_down {
                    down_button = down_button.fill(Color32::LIGHT_GREEN);
                }
                if ui.add(down_button).clicked() {
                    self.system.get_controller().button_dpad_down = true;
                }
                ui.add_space(29.0);
                // Cross Button
                let mut cross_button = egui::Button::new("🗙");
                if self.system.get_controller().button_cross {
                    cross_button = cross_button.fill(Color32::LIGHT_GREEN);
                }
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(124, 178, 232));
                if ui.add(cross_button).clicked() {
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
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
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
                        ui.selectable_value(&mut self.character1, Character::Law, "Law");
                        ui.selectable_value(&mut self.character1, Character::Lei, "Lei");
                        ui.selectable_value(&mut self.character1, Character::Paul, "Paul");
                        ui.selectable_value(&mut self.character1, Character::Xiaoyu, "Xiaoyu");
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
                        ui.selectable_value(&mut self.character2, Character::Law, "Law");
                        ui.selectable_value(&mut self.character2, Character::Lei, "Lei");
                        ui.selectable_value(&mut self.character2, Character::Paul, "Paul");
                        ui.selectable_value(&mut self.character2, Character::Xiaoyu, "Xiaoyu");
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
                        ui.selectable_value(&mut self.vision, Vision::Contrast, "Contrast");
                    });
                ui.end_row();
                ui.label("Split View");
                ui.checkbox(&mut self.split_view, "");
            });
            ui.horizontal(|_ui| {});

            // Vision Pipeline
            ui.horizontal(|ui| {
                ui.label("Vision Pipeline");
                let separator = egui::Separator::default();
                ui.add(separator.horizontal());
            });
            egui::Grid::new("vision_pipeline").show(ui, |ui| {
                ui.label("Red");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.red_thresholds[0]));
                    ui.add(egui::DragValue::new(&mut self.red_thresholds[1]));
                });
                ui.end_row();
                ui.label("Green");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.green_thresholds[0]));
                    ui.add(egui::DragValue::new(&mut self.green_thresholds[1]));
                });
                ui.end_row();
                ui.label("Blue");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.blue_thresholds[0]));
                    ui.add(egui::DragValue::new(&mut self.blue_thresholds[1]));
                });
                ui.end_row();
                ui.label("Dilate");
                ui.add(egui::Slider::new(&mut self.dilate_k, 0..=20));
                ui.end_row();
                ui.label("MSE");
                ui.add(egui::Slider::new(&mut self.max_mse, 0.0..=100.0).max_decimals(3));
            });
            ui.horizontal(|_ui| {});

            // Reinforcement Learning
            ui.horizontal(|ui| {
                ui.label("Reinforcement Learning");
                let separator = egui::Separator::default();
                ui.add(separator.horizontal());
            });
            egui::Grid::new("reinforcement_learning").show(ui, |ui| {
                ui.label("Learning Rate:");
                let learning_rate_widget = egui::DragValue::new(&mut self.learning_rate);
                let learning_rate_widget = learning_rate_widget.speed(0.01).clamp_range(0..=1);
                ui.add(learning_rate_widget);
                ui.end_row();
                ui.label("Discount Factor:");
                let discount_factor_widget = egui::DragValue::new(&mut self.discount_factor);
                let discount_factor_widget = discount_factor_widget.speed(0.01).clamp_range(0..=1);
                ui.add(discount_factor_widget);
            });
            ui.horizontal(|_ui| {});
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
        egui::SidePanel::right("my_right_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Profiling");
                let separator = egui::Separator::default();
                ui.add(separator.horizontal());
            });
            egui::Grid::new("profiling").show(ui, |ui| {
                ui.label("FPS:");
                if self.frame_time.total_time.as_millis() > 0 {
                    ui.label(format!(
                        "{:.2}",
                        (1000 / self.frame_time.total_time.as_millis())
                    ));
                } else {
                    ui.label("/0");
                }
                ui.end_row();
                ui.label("Total Time (ms):");
                ui.label(format!("{:.2}", self.frame_time.total_time.as_millis()));
                ui.end_row();
                ui.label("UI Time (ms):");
                ui.label(format!("{:.2}", self.frame_time.ui_time.as_millis()));
                ui.end_row();
                ui.label("PSX Time (ms):");
                ui.label(format!("{:.2}", self.frame_time.psx_time.as_millis()));
                ui.end_row();
                ui.label("Agent Time (ms):");
                ui.label(format!("{:.2}", self.frame_time.agent_time.as_millis()));
                ui.end_row();
            });
            ui.horizontal(|_ui| {});
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
            egui::Grid::new("ai_agent").show(ui, |ui| {
                ui.label("Total States:");
                let number_of_states = format!("{}", self.agent.get_number_of_states());
                ui.label(number_of_states);
                ui.end_row();
                ui.label("Revisited States:");
                let number_of_revisited_states =
                    format!("{}", self.agent.get_number_of_revisited_states());
                ui.label(number_of_revisited_states);
                ui.end_row();
                ui.label("Previous Next States:");
                let previous_next_states =
                    format!("{}", self.agent.get_number_of_previous_next_states());
                ui.label(previous_next_states);
            });
        });
    }

    fn process_frame(&mut self) {
        // Run frame
        self.run_frame();
        if self.replay.is_some() {
            self.update_replay(self.frame_time.psx_time.clone());
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
        let start_time = Instant::now();
        self.time_from_last_observation += self.frame_time.total_time;
        let period = Duration::from_secs_f32(1.0 / self.observation_frequency as f32);
        if self.time_from_last_observation > period {
            self.agent.set_red_thresholds(self.red_thresholds);
            self.agent.set_green_thresholds(self.green_thresholds);
            self.agent.set_blue_thresholds(self.blue_thresholds);
            self.agent.set_dilate_k(self.dilate_k);
            self.agent.set_max_mse(self.max_mse);
            // REWARD
            let reward = self.opponent_life_info.damage - self.agent_life_info.damage;
            let action = self.agent.visit_state(self.frame.clone(), reward);
            self.set_controller(action);
            self.time_from_last_observation = Duration::ZERO;
        }
        self.frame_time.agent_time = Instant::now() - start_time;
    }

    fn run_frame(&mut self) {
        let start_time = Instant::now();
        self.system.run_frame();
        self.frame_time.psx_time = Instant::now() - start_time;
        // Get frame buffer
        let (width, height) = self.system.get_display_size();
        let mut framebuffer = vec![0; width as usize * height as usize * 3].into_boxed_slice();
        self.system.get_framebuffer(&mut framebuffer, false);
        self.frame = convert_framebuffer_to_rgb_image(&framebuffer, width, height);
    }

    fn update_replay(&mut self, delta_time: Duration) {
        // Show for a certain duration and then load state
        let duration = self.replay.unwrap() + delta_time;
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

    fn set_controller(&mut self, action: u8) {
        self.system.get_controller().button_dpad_up = (action & 1 << 0) != 0;
        self.system.get_controller().button_dpad_down = (action & 1 << 1) != 0;
        self.system.get_controller().button_dpad_left = (action & 1 << 2) != 0;
        self.system.get_controller().button_dpad_right = (action & 1 << 3) != 0;
        self.system.get_controller().button_triangle = (action & 1 << 4) != 0;
        self.system.get_controller().button_square = (action & 1 << 5) != 0;
        self.system.get_controller().button_circle = (action & 1 << 6) != 0;
        self.system.get_controller().button_cross = (action & 1 << 7) != 0;
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
