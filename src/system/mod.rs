mod bios;
mod bus;
mod cpu;
mod cpu_types;
mod dma;
mod interrupt_controller;

use bios::Bios;
use bus::Bus;
use cpu::CPU;

use std::collections::HashMap;
use std::result::Result;
use std::string::String;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Eq, Hash, PartialEq)]
pub enum Event {
    SystemBootComplete,
    SystemStartComplete,
    SystemStopComplete,
    SystemShutdownComplete,
    InstructionComplete,
    FrameComplete,
}

// Define a type for the callback functions.
type Callback = Box<dyn Fn() + Send + 'static>;

pub struct System {
    inner_system: Arc<Mutex<InnerSystem>>,
    processing_thread: Option<thread::JoinHandle<()>>,
    is_on: Arc<Mutex<bool>>,
    is_running: Arc<Mutex<bool>>,
    run_next_instruction: Arc<Mutex<bool>>,
    run_next_frame: Arc<Mutex<bool>>,
    callbacks: Arc<Mutex<HashMap<Event, Callback>>>,
}

struct InnerSystem {
    bus: Bus,
    cpu: CPU,
}

// To run in thread
fn run_inner_system(
    inner_system: Arc<Mutex<InnerSystem>>,
    is_on: Arc<Mutex<bool>>,
    is_running: Arc<Mutex<bool>>,
    run_next_instruction: Arc<Mutex<bool>>,
    run_next_frame: Arc<Mutex<bool>>,
    callbacks: Arc<Mutex<HashMap<Event, Callback>>>,
) -> () {
    while *is_on.lock().unwrap() {
        if *is_running.lock().unwrap() {
            inner_system.lock().unwrap().next_instruction();
            if let Some(callback) = callbacks.lock().unwrap().get(&Event::InstructionComplete) {
                callback();
            }
        } else if *run_next_instruction.lock().unwrap() {
            inner_system.lock().unwrap().next_instruction();
            let mut run_next_instruction = run_next_instruction.lock().unwrap();
            *run_next_instruction = false;
        }
        thread::sleep(std::time::Duration::from_millis(20));
    }
}

impl System {
    pub fn new() -> Self {
        Self {
            inner_system: Arc::new(Mutex::new(InnerSystem::new())),
            processing_thread: None,
            is_on: Arc::new(Mutex::new(false)),
            is_running: Arc::new(Mutex::new(false)),
            run_next_instruction: Arc::new(Mutex::new(false)),
            run_next_frame: Arc::new(Mutex::new(false)),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn boot(&mut self) -> () {
        // Boot inner system
        self.inner_system.lock().unwrap().boot();
        if let Some(callback) = self
            .callbacks
            .lock()
            .unwrap()
            .get(&Event::SystemBootComplete)
        {
            callback();
        }
        // We are on!
        let mut is_on = self.is_on.lock().unwrap();
        *is_on = true;
        // Launch processing thread
        let inner_system = Arc::clone(&self.inner_system);
        let is_on = Arc::clone(&self.is_on);
        let is_running = Arc::clone(&self.is_running);
        let run_next_instruction = Arc::clone(&self.run_next_instruction);
        let run_next_frame = Arc::clone(&self.run_next_frame);
        let callbacks = Arc::clone(&self.callbacks);
        let handle = thread::spawn(move || {
            run_inner_system(
                inner_system,
                is_on,
                is_running,
                run_next_instruction,
                run_next_frame,
                callbacks,
            );
        });
        self.processing_thread = Some(handle);
    }

    pub fn register_callback<F>(&mut self, event: Event, callback: F)
    where
        F: Fn() + Send + 'static,
    {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.insert(event, Box::new(callback));
    }

    pub fn register_on_frame_completed_callback(&mut self) -> () {}

    pub fn start(&mut self) -> () {
        let mut is_running = self.is_running.lock().unwrap();
        *is_running = true;
    }

    pub fn next_instruction(&mut self) -> () {
        let mut run_next_instruction = self.run_next_instruction.lock().unwrap();
        *run_next_instruction = true;
    }

    pub fn next_frame(&mut self) -> () {}

    pub fn stop(&mut self) -> () {
        let mut is_running = self.is_running.lock().unwrap();
        *is_running = false;
        if let Some(callback) = self
            .callbacks
            .lock()
            .unwrap()
            .get(&Event::SystemStopComplete)
        {
            callback();
        }
    }

    pub fn reset(&mut self) -> () {}

    pub fn shutdown(&mut self) -> () {
        // We need a scope to avoid deadlock on thread
        {
            let mut is_on = self.is_on.lock().unwrap();
            *is_on = false;
        }
        if let Some(handle) = self.processing_thread.take() {
            handle.join().unwrap();
        }
        if let Some(callback) = self
            .callbacks
            .lock()
            .unwrap()
            .get(&Event::SystemShutdownComplete)
        {
            callback();
        }
    }

    pub fn is_on(&self) -> bool {
        *self.is_on.lock().unwrap()
    }

    pub fn is_running(&self) -> bool {
        *self.is_running.lock().unwrap()
    }

    pub fn get_cycle(&self) -> usize {
        self.inner_system.lock().unwrap().get_cycle()
    }

    pub fn get_instruction(&self) -> u32 {
        self.inner_system.lock().unwrap().get_instruction()
    }
}

impl InnerSystem {
    pub fn new() -> Self {
        Self {
            bus: Bus::new(),
            cpu: CPU::new(),
        }
    }

    pub fn boot(&mut self) -> () {
        self.load_bios().expect("Failed to load Bios");
        self.initialize().expect("Failed to initialize");
    }

    pub fn next_instruction(&mut self) -> Result<(), String> {
        self.run_frame()
        // Render display
    }

    pub fn get_cycle(&self) -> usize {
        self.cpu.get_cycle()
    }

    pub fn get_instruction(&self) -> u32 {
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
