const BIOS_SIZE: usize = 0x80000; // Size of the bios (512 KB)

pub struct Bus {
    pub bios: [u8; BIOS_SIZE],
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bios: [0; BIOS_SIZE],
        }
    }
}
