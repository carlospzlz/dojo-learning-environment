use eframe::egui;
use egui::Color32;
use std::sync::{Arc, Mutex};
use std::thread;

// Emu system
mod system;
use system::System;

fn main() -> () {
    // Shared state
    let system = Arc::new(Mutex::new(System::new()));
    let is_running = Arc::new(Mutex::new(false));
    let execute_once = Arc::new(Mutex::new(false));

    // System thread
    let system_clone = Arc::clone(&system);
    let is_running_clone = Arc::clone(&is_running);
    let execute_once_clone = Arc::clone(&execute_once);
    let handle = thread::spawn(move || {
        run_system(system_clone, is_running_clone, execute_once_clone);
    });

    //// GUI thread
    //let options = eframe::NativeOptions {
    //    initial_window_size: Some(egui::vec2(890.0, 525.0)),
    //    multisampling: 4,
    //    renderer: eframe::Renderer::Glow,
    //    ..Default::default()
    //};
    //eframe::run_native(
    //    "Emu Explorer",
    //    options,
    //    Box::new(|cc| Box::new(MyApp::new(cc, system, is_running, execute_once))),
    //);
    //println!("hey");
    handle.join().unwrap();
}

// To run in thread
fn run_system(
    system: Arc<Mutex<System>>,
    is_running: Arc<Mutex<bool>>,
    execute_once: Arc<Mutex<bool>>,
) -> () {
    //system.lock().unwrap().boot_system();
    let mut system = System::new();
    //system.boot_system();
    system.execute();
    system.execute();
    system.execute();
    //loop {
    //    //if *is_running.lock().unwrap() {
    //    //    println!("About to execute");
    //    //    let mut system = system.lock().unwrap().execute();
    //    //    thread::sleep(std::time::Duration::from_secs(2));
    //    //} else if *execute_once.lock().unwrap() {
    //    //    println!("Execute once");
    //    //    system.lock().unwrap().execute();
    //    //    let mut execute_once = execute_once.lock().unwrap();
    //    //    *execute_once = false;
    //    //} else {
    //    //    thread::sleep(std::time::Duration::from_millis(20));
    //    //}
    //    //println!("Here");
    //    //let mut system = system.lock().unwrap();
    //    //system.execute();
    //    system.execute();
    //    thread::sleep(std::time::Duration::from_secs(2));
    //}
}

struct MyApp {
    system: Arc<Mutex<System>>,
    is_running: Arc<Mutex<bool>>,
    execute_once: Arc<Mutex<bool>>,
    rotating_triangle: Arc<Mutex<RotatingTriangle>>,
    angle: f32,
}

impl MyApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        system: Arc<Mutex<System>>,
        is_running: Arc<Mutex<bool>>,
        execute_once: Arc<Mutex<bool>>,
    ) -> Self {
        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        Self {
            system,
            is_running,
            execute_once,
            rotating_triangle: Arc::new(Mutex::new(RotatingTriangle::new(gl))),
            angle: 0.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //if self.is_running {
        //    self.system.execute();
        //}

        //egui::TopBottomPanel::bottom("my_bottom_panel").show(ctx, |ui| {
        //    ui.label("Debug info");
        //    let system = self.system.lock().unwrap();
        //    let cycle = system.get_cycle();
        //    let instruction = system.get_instruction();
        //    ui.label(format!("{}", cycle));
        //    ui.label(format!("0x{:08x}", instruction));
        //});
        //egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
        //    if ui.button("Next Instruction").clicked() {
        //        let is_running = self.is_running.lock().unwrap();
        //        if !(*is_running) {
        //            let mut execute_once = self.execute_once.lock().unwrap();
        //            *execute_once = true;
        //        }
        //    }
        //    if ui.button("Play").clicked() {
        //        let mut is_running = self.is_running.lock().unwrap();
        //        *is_running = true;
        //        println!("Passes");
        //    }
        //    if ui.button("Stop").clicked() {
        //        let mut is_running = self.is_running.lock().unwrap();
        //        *is_running = false;
        //    }
        //    ui.button("Next");

        //    // File Controls
        //    ui.button("Load");
        //    ui.button("Save");
        //});
        egui::SidePanel::right("my_right_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {});
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.button("⏶");
                ui.add_space(30.0);
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(64, 226, 160));
                ui.button("∆");
            });
            ui.horizontal(|ui| {
                ui.button("⏴");
                ui.button("⏵");
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(255, 105, 248));
                ui.button("◻");
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(255, 102, 102));
                ui.button("○");
            });
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.button("⏷");
                ui.add_space(29.0);
                ui.style_mut().visuals.override_text_color = Some(Color32::from_rgb(124, 178, 232));
                ui.button("🗙");
            });
            ui.horizontal(|ui| {});
            ui.horizontal(|ui| {
                ui.button("SELECT");
                ui.button("START");
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.custom_painting(ui);
            });
        });
    }
}

impl MyApp {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::new(640.0, 480.0), egui::Sense::drag());

        self.angle += response.drag_delta().x * 0.01;

        // Clone locals so we can move them into the paint callback:
        let angle = self.angle;
        let rotating_triangle = self.rotating_triangle.clone();

        let callback = egui::PaintCallback {
            rect,
            callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                rotating_triangle.lock().unwrap().paint(painter.gl(), angle);
            })),
        };
        ui.painter().add(callback);
    }
}

struct RotatingTriangle {
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

impl RotatingTriangle {
    fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        let shader_version = if cfg!(target_arch = "wasm32") {
            "#version 300 es"
        } else {
            "#version 330"
        };

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                r#"
                    const vec2 verts[3] = vec2[3](
                        vec2(0.0, 1.0),
                        vec2(-1.0, -1.0),
                        vec2(1.0, -1.0)
                    );
                    const vec4 colors[3] = vec4[3](
                        vec4(1.0, 0.0, 0.0, 1.0),
                        vec4(0.0, 1.0, 0.0, 1.0),
                        vec4(0.0, 0.0, 1.0, 1.0)
                    );
                    out vec4 v_color;
                    uniform float u_angle;
                    void main() {
                        v_color = colors[gl_VertexID];
                        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
                        gl_Position.x *= cos(u_angle);
                    }
                "#,
                r#"
                    precision mediump float;
                    in vec4 v_color;
                    out vec4 out_color;
                    void main() {
                        out_color = v_color;
                    }
                "#,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{shader_version}\n{shader_source}"));
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Failed to compile {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "{}",
                gl.get_program_info_log(program)
            );

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            Self {
                program,
                vertex_array,
            }
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&self, gl: &glow::Context, angle: f32) {
        use glow::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_angle").as_ref(),
                angle,
            );
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }
}