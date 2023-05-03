use crate::system::bios::BIOS_SIZE;

const RAM_SIZE: usize = 0x200000; // Size of the RAM (2 MB)

pub struct Bus {
    pub bios: [u8; BIOS_SIZE],
    pub ram: [u8; RAM_SIZE],
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bios: [0; BIOS_SIZE],
            ram: [0; RAM_SIZE],
        }
    }

    pub fn initialize(&self) {}
}
