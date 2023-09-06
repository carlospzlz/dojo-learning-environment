mod bios;
mod bus;
mod cpu;
mod cpu_types;
mod dma;
mod interrupt_controller;

use bios::Bios;
use bus::Bus;
use cpu::CPU;

use std::result::Result;
use std::string::String;

pub struct System {
    bus: Bus,
    cpu: CPU,
}

impl System {
    pub fn new() -> Self {
        Self {
            bus: Bus::new(),
            cpu: CPU::new(),
        }
    }

    pub fn boot_system(&mut self) -> Result<(), String> {
        self.load_bios().expect("Failed to load Bios");
        self.initialize().expect("Failed to initialize");
        //s_cpu_thread_handler = GetForCallingThread()
        Ok(())
    }

    pub fn execute(&mut self) -> Result<(), String> {
        self.run_frame()
        // Render display
    }

    pub fn get_cycle(&mut self) -> usize {
        self.cpu.get_cycle()
    }

    pub fn get_instruction(&mut self) -> u32 {
        self.cpu.get_instruction()
    }

    fn run_frame(&mut self) -> Result<(), String> {
        // GPU restore
        self.cpu.execute(&mut self.bus)
        // GPU reset
    }

    fn load_bios(&mut self) -> Result<(), String> {
        let bios = Bios::from_file("bios/scph1001.bin").expect("Could not load bios!");

        // Load the bios data to the bus
        self.bus.bios = bios.data();

        Ok(())
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.bus.initialize();
        //self.cpu.iniatialize(self.bus);
        //self.gpu.initialize();
        Ok(())
    }
}
