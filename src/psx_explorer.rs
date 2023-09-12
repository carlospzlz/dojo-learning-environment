use eframe::egui;
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

//fn load_image_from_memory(image_data: &[u8]) -> Result<ColorImage, image::ImageError> {
//    let image = image::load_from_memory(image_data)?;
//    let size = [image.width() as _, image.height() as _];
//    let image_buffer = image.to_rgba8();
//    let pixels = image_buffer.as_flat_samples();
//    Ok(ColorImage::from_rgba_unmultiplied(
//        size,
//        pixels.as_slice(),
//    ))
//}

fn main() -> () {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(890.0, 554.0)),
        //multisampling: 4,
        //renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Emu Explorer",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    );
}

struct MyApp {
    system: System,
    //rotating_triangle: Arc<Mutex<RotatingTriangle>>,
    angle: f32,
    texture: Option<egui::TextureHandle>,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        //let gl = cc
        //    .gl
        //    .as_ref()
        //    .expect("You need to run eframe with the glow backend");
        let bios_filepath = "bios/scph1001.bin";
        let game_filepath = "roms/tekken.bin";
        let mut system = System::new(&bios_filepath, &game_filepath);
        //cc.egui_ctx.re
        // Register callbacks here
        system.reset();
        Self {
            system,
            //rotating_triangle: Arc::new(Mutex::new(RotatingTriangle::new(gl))),
            angle: 0.0,
            texture: Default::default(),
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
            // 113 width of right panel
            let new_width = asize[0].round() as u32 - 240;
            let new_height = asize[1].round() as u32 - 40;

            // Load texture
            //let img = ColorImage::from_rgb([width, height], &framebuffer);
            let img = image::imageops::resize(&img, new_width, new_height, image::imageops::FilterType::Lanczos3);
            let img = ColorImage::from_rgb([new_width as usize, new_height as usize], img.as_raw());
            let texture = ctx.load_texture(
                    "my-image",
                    //egui::ColorImage::example(),
                    //img.as_flat_samples(),
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
            if ui.button("Boot").clicked() {
                //register_repaint_callback(ctx, &mut self.system, Event::SystemBootComplete);
                //self.system.boot();
            }
            if ui.button("Start").clicked() {
                //register_repaint_callback(ctx, &mut self.system, Event::SystemStartComplete);
                //register_repaint_callback(ctx, &mut self.system, Event::InstructionComplete);
                //self.system.start();
            }
            if ui.button("Stop").clicked() {
                //register_repaint_callback(ctx, &mut self.system, Event::SystemStopComplete);
                //self.system.stop();
            }
            if ui.button("Next Instruction").clicked() {
                //register_repaint_callback(ctx, &mut self.system, Event::InstructionComplete);
                //self.system.next_instruction();
            }
            if ui.button("Shutdown").clicked() {
                //register_repaint_callback(ctx, &mut self.system, Event::SystemShutdownComplete);
                //self.system.shutdown();
            }
            // File Controls
            ui.button("Load");
            ui.button("Save");
        });
        egui::SidePanel::right("my_right_panel").show(ctx, |ui| {
            //ui.horizontal(|ui| {
            //    if self.system.is_on() {
            //        ui.label(RichText::new("⚡").color(Color32::YELLOW));
            //    } else {
            //        ui.label(RichText::new("⚡").color(Color32::GRAY));
            //    }
            //    if self.system.is_running() {
            //        ui.label(RichText::new("⏺").color(Color32::LIGHT_GREEN));
            //    } else {
            //        ui.label(RichText::new("⏺").color(Color32::GRAY));
            //    }
            //});
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
        self.system.run_frame();
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

//fn register_repaint_callback(
//    ctx: &egui::Context,
//    system: &mut system::System,
//    event: system::Event,
//) {
//    let ctx_clone = ctx.clone();
//    system.register_callback(event, move || {
//        ctx_clone.request_repaint();
//    });
//}

//impl MyApp {
//    fn custom_painting(&mut self, ui: &mut egui::Ui) {
//        let (rect, response) =
//            ui.allocate_exact_size(egui::Vec2::new(640.0, 480.0), egui::Sense::drag());
//
//        self.angle += response.drag_delta().x * 0.01;
//
//        // Clone locals so we can move them into the paint callback:
//        let angle = self.angle;
//        let rotating_triangle = self.rotating_triangle.clone();
//
//        let callback = egui::PaintCallback {
//            rect,
//            callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
//                rotating_triangle.lock().unwrap().paint(painter.gl(), angle);
//            })),
//        };
//        ui.painter().add(callback);
//    }
//}
//
//struct RotatingTriangle {
//    program: glow::Program,
//    vertex_array: glow::VertexArray,
//}
//
//impl RotatingTriangle {
//    fn new(gl: &glow::Context) -> Self {
//        use glow::HasContext as _;
//
//        let shader_version = if cfg!(target_arch = "wasm32") {
//            "#version 300 es"
//        } else {
//            "#version 330"
//        };
//
//        unsafe {
//            let program = gl.create_program().expect("Cannot create program");
//
//            let (vertex_shader_source, fragment_shader_source) = (
//                r#"
//                    const vec2 verts[3] = vec2[3](
//                        vec2(0.0, 1.0),
//                        vec2(-1.0, -1.0),
//                        vec2(1.0, -1.0)
//                    );
//                    const vec4 colors[3] = vec4[3](
//                        vec4(1.0, 0.0, 0.0, 1.0),
//                        vec4(0.0, 1.0, 0.0, 1.0),
//                        vec4(0.0, 0.0, 1.0, 1.0)
//                    );
//                    out vec4 v_color;
//                    uniform float u_angle;
//                    void main() {
//                        v_color = colors[gl_VertexID];
//                        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
//                        gl_Position.x *= cos(u_angle);
//                    }
//                "#,
//                r#"
//                    precision mediump float;
//                    in vec4 v_color;
//                    out vec4 out_color;
//                    void main() {
//                        out_color = v_color;
//                    }
//                "#,
//            );
//
//            let shader_sources = [
//                (glow::VERTEX_SHADER, vertex_shader_source),
//                (glow::FRAGMENT_SHADER, fragment_shader_source),
//            ];
//
//            let shaders: Vec<_> = shader_sources
//                .iter()
//                .map(|(shader_type, shader_source)| {
//                    let shader = gl
//                        .create_shader(*shader_type)
//                        .expect("Cannot create shader");
//                    gl.shader_source(shader, &format!("{shader_version}\n{shader_source}"));
//                    gl.compile_shader(shader);
//                    assert!(
//                        gl.get_shader_compile_status(shader),
//                        "Failed to compile {shader_type}: {}",
//                        gl.get_shader_info_log(shader)
//                    );
//                    gl.attach_shader(program, shader);
//                    shader
//                })
//                .collect();
//
//            gl.link_program(program);
//            assert!(
//                gl.get_program_link_status(program),
//                "{}",
//                gl.get_program_info_log(program)
//            );
//
//            for shader in shaders {
//                gl.detach_shader(program, shader);
//                gl.delete_shader(shader);
//            }
//
//            let vertex_array = gl
//                .create_vertex_array()
//                .expect("Cannot create vertex array");
//
//            Self {
//                program,
//                vertex_array,
//            }
//        }
//    }
//
//    fn destroy(&self, gl: &glow::Context) {
//        use glow::HasContext as _;
//        unsafe {
//            gl.delete_program(self.program);
//            gl.delete_vertex_array(self.vertex_array);
//        }
//    }
//
//    fn paint(&self, gl: &glow::Context, angle: f32) {
//        use glow::HasContext as _;
//        //unsafe {
//        //    gl.use_program(Some(self.program));
//        //    gl.uniform_1_f32(
//        //        gl.get_uniform_location(self.program, "u_angle").as_ref(),
//        //        angle,
//        //    );
//        //    gl.bind_vertex_array(Some(self.vertex_array));
//        //    gl.draw_arrays(glow::TRIANGLES, 0, 3);
//        //}
//    }
//}
