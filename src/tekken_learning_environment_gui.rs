use egui::plot::{Line, Plot, PlotPoints};
use egui::{Color32, ColorImage, Vec2};
use egui_file::FileDialog;
use image::{DynamicImage, Rgb, RgbImage};
use log::error;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
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
    Nina,
    Random,
}

#[derive(Debug, PartialEq)]
enum Vision {
    PSX,
    Life,
    Agent,
    Crop,
    Contrast,
    Mask,
    Masked,
    Centroids,
    Chars,
    Segmented,
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
    is_running_next_frame: bool,
    last_vision_stages: vision::VisionStages,
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
    max_mse: f64,
    char1_pixel_probability: HashMap<Rgb<u8>, (u64, u64)>,
    char2_pixel_probability: HashMap<Rgb<u8>, (u64, u64)>,
    char1_probability_threshold: f64,
    char2_probability_threshold: f64,
    char1_dilate_k: u8,
    char2_dilate_k: u8,
    previous_trace_abstraction: RgbImage,
    trace: u8,
    radius: u32,
    show_states_plot: bool,
    opened_agent: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
    saved_file: Option<PathBuf>,
    save_file_dialog: Option<FileDialog>,
}

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>, bios: String, game: String) -> Self {
        let mut system = System::new(&bios, &game);
        system.reset();
        let radius = 30;
        let mut agent = Agent::new();
        agent.set_radius(radius);
        Self {
            bios,
            game,
            system,
            frame: RgbImage::default(),
            is_running: false,
            is_running_next_frame: false,
            last_vision_stages: vision::VisionStages::default(),
            vision: Vision::Agent,
            split_view: true,
            character1: Character::Yoshimitsu,
            character2: Character::Lei,
            current_combat: None,
            agent_life_info: LifeInfo::default(),
            opponent_life_info: LifeInfo::default(),
            replay: None,
            agent,
            observation_frequency: 10,
            time_from_last_observation: Duration::from_secs(1),
            frame_time: FrameTime::default(),
            learning_rate: 0.5,
            discount_factor: 0.9,
            red_thresholds: [0, 173],
            green_thresholds: [15, 165],
            blue_thresholds: [15, 156],
            dilate_k: 12,
            max_mse: 2000.0,
            char1_pixel_probability: HashMap::new(),
            char2_pixel_probability: HashMap::new(),
            char1_probability_threshold: 0.7,
            char2_probability_threshold: 0.7,
            char1_dilate_k: 2,
            char2_dilate_k: 2,
            previous_trace_abstraction: RgbImage::default(),
            trace: 6,
            radius,
            show_states_plot: false,
            opened_agent: None,
            open_file_dialog: None,
            saved_file: None,
            save_file_dialog: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let start_time = Instant::now();
        self.menu_bar(ctx);
        self.show_states_plot(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.bottom_panel(ctx);
        self.central_panel(ctx);
        self.file_dialogs(ctx);
        self.frame_time.ui_time = Instant::now() - start_time;

        // Processing
        if self.is_running {
            self.process_frame();
        } else if self.is_running_next_frame {
            self.is_running_next_frame = !self.process_frame();
        } else {
            // Even if not running update vision
            let (_, vision_stages) = vision::get_frame_abstraction(
                &self.frame.clone(),
                self.red_thresholds,
                self.green_thresholds,
                self.blue_thresholds,
                self.dilate_k,
                &mut self.char1_pixel_probability.clone(),
                &mut self.char2_pixel_probability.clone(),
                self.char1_probability_threshold,
                self.char2_probability_threshold,
                self.char1_dilate_k,
                self.char2_dilate_k,
            );
            self.last_vision_stages = vision_stages;
        }

        // Request repaint
        ctx.request_repaint();

        self.frame_time.total_time = Instant::now() - start_time;
        self.agent.add_training_time(self.frame_time.total_time);
    }
}

impl MyApp {
    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load Agent").clicked() {
                        self.is_running = false;
                        let dialog = FileDialog::open_file(self.opened_agent.clone());
                        let dialog = dialog.title("Load Agent");
                        let mut dialog = dialog.default_size(Vec2 { x: 300.0, y: 200.0 });
                        dialog.open();
                        self.open_file_dialog = Some(dialog);
                        ui.close_menu();
                    }
                    if ui.button("Save Agent").clicked() {
                        self.is_running = false;
                        let dialog = FileDialog::save_file(self.saved_file.clone());
                        let dialog = dialog.title("Save Agent");
                        let mut dialog = dialog.default_size(Vec2 { x: 300.0, y: 200.0 });
                        dialog.open();
                        self.save_file_dialog = Some(dialog);
                        ui.close_menu();
                    }
                });

                // Additional menus can be added here, like Edit, View, etc.
                ui.menu_button("Advanced", |ui| {
                    if ui.button("Open States Plot").clicked() {
                        self.show_states_plot = true;
                        ui.close_menu();
                    }
                });
            });
        });
    }

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
                Vision::Life => img = vision::visualize_life_bars(img),
                Vision::Agent => img = self.agent.get_last_state_abstraction(),
                Vision::Crop => img = self.last_vision_stages.cropped_frame.clone(),
                Vision::Contrast => img = self.last_vision_stages.contrast_frame.clone(),
                Vision::Mask => img = self.last_vision_stages.mask.clone(),
                Vision::Masked => img = self.last_vision_stages.masked_frame.clone(),
                Vision::Centroids => img = self.last_vision_stages.centroids_hud.clone(),
                Vision::Chars => img = self.last_vision_stages.chars_hud.clone(),
                Vision::Segmented => img = self.last_vision_stages.segmented_frame.clone(),
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
            ui.horizontal(|_ui| {});
            // General
            //ui.horizontal(|ui| {
            //    ui.label("General");
            //    let separator = egui::Separator::default();
            //    ui.add(separator.horizontal());
            //});
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
                        ui.selectable_value(&mut self.character1, Character::Nina, "Nina");
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
                        ui.selectable_value(&mut self.character2, Character::Nina, "Nina");
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
                        ui.selectable_value(&mut self.vision, Vision::Crop, "Crop");
                        ui.selectable_value(&mut self.vision, Vision::Contrast, "Contrast");
                        ui.selectable_value(&mut self.vision, Vision::Mask, "Mask");
                        ui.selectable_value(&mut self.vision, Vision::Masked, "Masked");
                        ui.selectable_value(&mut self.vision, Vision::Centroids, "Centroids HUD");
                        ui.selectable_value(&mut self.vision, Vision::Chars, "Chars HUD");
                        ui.selectable_value(&mut self.vision, Vision::Segmented, "Segmented");
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
            ui.label("Contrast Thresholds");
            egui::Grid::new("contrast_thresholds").show(ui, |ui| {
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
            });
            ui.label("Contrast Mask");
            egui::Grid::new("contrast_mask").show(ui, |ui| {
                ui.label("Dilate");
                ui.add(egui::Slider::new(&mut self.dilate_k, 0..=20));
            });
            ui.label("Character 1");
            egui::Grid::new("char1").show(ui, |ui| {
                ui.label("Thres.");
                ui.add(egui::Slider::new(
                    &mut self.char1_probability_threshold,
                    0.0..=1.0,
                ));
                ui.end_row();
                ui.label("Dilate");
                ui.add(egui::Slider::new(&mut self.char1_dilate_k, 0..=20));
            });
            ui.label("Character 2");
            egui::Grid::new("char2").show(ui, |ui| {
                ui.label("Thres.");
                ui.add(egui::Slider::new(
                    &mut self.char2_probability_threshold,
                    0.0..=1.0,
                ));
                ui.end_row();
                ui.label("Dilate");
                ui.add(egui::Slider::new(&mut self.char2_dilate_k, 0..=20));
            });
            ui.label("Motion");
            egui::Grid::new("motion").show(ui, |ui| {
                ui.label("Trace");
                ui.add(egui::Slider::new(&mut self.trace, 0..=255));
            });
            ui.label("State Comparison");
            egui::Grid::new("state_comparison").show(ui, |ui| {
                ui.label("Radius");
                if ui
                    .add(egui::Slider::new(&mut self.radius, 0..=255))
                    .changed()
                {
                    self.agent.set_radius(self.radius);
                }
                ui.end_row();
                ui.label("MSE");
                ui.add(egui::Slider::new(&mut self.max_mse, 0.0..=60000.0).max_decimals(3));
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
            ui.horizontal(|_ui| {});
            //ui.horizontal(|ui| {
            //    ui.label("Profiling");
            //    let separator = egui::Separator::default();
            //    ui.add(separator.horizontal());
            //});
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
                ui.label("Training Time:");
                let total_seconds = self.agent.get_training_time().as_secs();
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;
                ui.label(format!("{:02}:{:02}:{:02}", hours, minutes, seconds));
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
                    self.is_running_next_frame = true;
                    //if !self.is_running {
                    //    self.process_frame();
                    //    // Apparently this is not needed, it actually seems
                    //    // to produce some unsynching
                    //    //ctx.request_repaint();
                    //}
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

    fn show_states_plot(&mut self, ctx: &egui::Context) {
        if self.show_states_plot {
            egui::Window::new("States")
                .open(&mut self.show_states_plot) // Bind visibility to flag
                .show(ctx, |ui| {
                    ui.label("States per iteration");

                    // Create plot from states per iteration
                    let states_per_iteration = self.agent.get_states_per_iteration();
                    let points = PlotPoints::from_iter(states_per_iteration);
                    let line = Line::new(points);
                    Plot::new("states_per_iteration")
                        .view_aspect(2.0)
                        .show(ui, |plot_ui| plot_ui.line(line));
                });
        }
    }

    fn file_dialogs(&mut self, ctx: &egui::Context) {
        // Load Agent
        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    let path = file.to_str().unwrap();
                    self.agent = q_learning::load_agent(path);
                }
            }
        }

        // Save Agent
        if let Some(dialog) = &mut self.save_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    let path = file.to_str().unwrap();
                    q_learning::save_agent(&self.agent, path);
                }
            }
        }
    }

    fn process_frame(&mut self) -> bool {
        // Run frame
        self.run_frame();
        if self.replay.is_some() {
            self.update_replay(self.frame_time.psx_time.clone());
            return false;
        }

        // Get life info
        let lifes_info = vision::get_life_info(self.frame.clone());
        self.agent_life_info = lifes_info.0;
        self.opponent_life_info = lifes_info.1;

        // Check for end of combat
        if self.agent_life_info.life == 0.0 || self.opponent_life_info.life == 0.0 {
            println!("End of combat");
            self.replay = Some(Duration::ZERO);
            return false;
        }

        self.reset_controller();

        // Feed AI agent
        if self.observation_frequency == 0 {
            return false;
        }
        let start_time = Instant::now();
        self.time_from_last_observation += self.frame_time.total_time;
        let period = Duration::from_secs_f32(1.0 / self.observation_frequency as f32);
        let mut processed = false;
        if self.time_from_last_observation > period {
            // VISION PIPELINE
            let (mut frame_abstraction, vision_stages) = vision::get_frame_abstraction(
                &self.frame.clone(),
                self.red_thresholds,
                self.green_thresholds,
                self.blue_thresholds,
                self.dilate_k,
                &mut self.char1_pixel_probability,
                &mut self.char2_pixel_probability,
                self.char1_probability_threshold,
                self.char2_probability_threshold,
                self.char1_dilate_k,
                self.char2_dilate_k,
            );
            if self.previous_trace_abstraction.is_empty() {
                self.previous_trace_abstraction = RgbImage::new(
                    frame_abstraction.frame.width(),
                    frame_abstraction.frame.height(),
                )
            };
            let trace_abstraction = vision::add_to_trace(
                &frame_abstraction.frame,
                &self.previous_trace_abstraction,
                self.trace,
            );
            self.previous_trace_abstraction = trace_abstraction.clone();
            frame_abstraction.frame = trace_abstraction;

            // REWARD
            let reward = self.opponent_life_info.damage - self.agent_life_info.damage;
            let action = self
                .agent
                .visit_state(frame_abstraction, reward, self.max_mse);
            self.last_vision_stages = vision_stages;
            self.set_controller(action);
            self.time_from_last_observation = Duration::ZERO;
            processed = true;
        }
        self.frame_time.agent_time = Instant::now() - start_time;
        processed
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
