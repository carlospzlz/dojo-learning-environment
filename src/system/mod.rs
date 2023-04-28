mod bios;
mod bus;

use bios::Bios;
use bus::Bus;
use std::result::Result;
use std::string::String;

pub struct System {
    bus: Bus,
}

impl System {
    pub fn new() -> Self {
        Self { bus: Bus::new() }
    }

    pub fn boot_system(&mut self) -> Result<(), String> {
        self.load_bios();
        self.initialize();
        //s_cpu_thread_handler = GetForCallingThread()
    }

    pub fn execute(&mut self) -> Result<(), String> {

    }

    fn load_bios(&mut self) -> Result<(), String> {
        let bios = Bios::from_file("bios/scph1001.bin").expect("Something went wrong!");

        // Load the bios data to the bus
        self.bus.bios = bios.data();

        Ok(())
    }

    fn initialize(&self) -> Result<(), String> {
        //self.cpu.iniatialize();
        self.bus.initialize();
        //self.gpu.initialize();
        Ok(())
    }

}
