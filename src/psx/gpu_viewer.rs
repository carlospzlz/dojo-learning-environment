use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct GpuVertex {
    pub position: (i16, i16),
    pub texcoord: (u8, u8),
    pub colour: (u8, u8, u8),
}

impl GpuVertex {
    pub fn new() -> Self {
        Self {
            position: (0, 0),
            texcoord: (0, 0),
            colour: (0, 0, 0),
        }
    }

    #[allow(dead_code)]
    pub fn position(&self) -> (f32, f32) {
        (self.position.0 as f32, self.position.1 as f32)
    }

    #[allow(dead_code)]
    pub fn texcoord(&self, texpage: u16) -> (f32, f32) {
        let tpx = (texpage & 0xf) << 6;
        let tpy = (texpage & 0x10) << 4;

        let depth = (texpage & 0x180) >> 7;

        let mut u = self.texcoord.0 as u16;
        let v = self.texcoord.1 as u16;

        if depth == 0 {
            u >>= 2;
        } else if depth == 1 {
            u >>= 1;
        }

        ((tpx + u) as f32, (tpy + v) as f32)
    }

    #[allow(dead_code)]
    pub fn colour(&self) -> [f32; 3] {
        let r = (self.colour.0 as f32) / 255.0;
        let g = (self.colour.1 as f32) / 255.0;
        let b = (self.colour.2 as f32) / 255.0;

        [r, g, b]
    }
}

#[derive(Serialize, Deserialize)]
pub struct GpuPolygon {
    pub vertices: [GpuVertex; 4],
    pub texpage: u16,

    pub shaded: bool,
    pub quad: bool,
    pub textured: bool,
    pub semi_transparent: bool,
    pub raw_texture: bool,
}

impl GpuPolygon {
    pub fn new() -> Self {
        Self {
            vertices: [GpuVertex::new(); 4],
            texpage: 0,

            shaded: false,
            quad: false,
            textured: false,
            semi_transparent: false,
            raw_texture: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum GpuCommand {
    Polygon(GpuPolygon),
}

impl GpuCommand {
    #[allow(dead_code)]
    pub fn name(command: &GpuCommand) -> &'static str {
        match command {
            GpuCommand::Polygon(p) => match (p.shaded, p.textured, p.quad) {
                (false, false, false) => "Monochrome Triangle",
                (false, false, true) => "Monochrome Quad",
                (false, true, false) => "Textured Triangle",
                (false, true, true) => "Textured Quad",
                (true, false, false) => "Shaded Triangle",
                (true, false, true) => "Shaded Quad",
                (true, true, false) => "Shaded Textured Triangle",
                (true, true, true) => "Shaded Textured Quad",
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GpuFrame {
    pub commands: Vec<GpuCommand>,
}

impl GpuFrame {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add(&mut self, command: GpuCommand) {
        self.commands.push(command);
    }

    #[allow(dead_code)]
    pub fn take(&mut self, frame: &mut GpuFrame) {
        self.commands = frame.commands.drain(..).collect();
    }
}
