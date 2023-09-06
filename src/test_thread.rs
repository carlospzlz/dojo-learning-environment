use std::sync::{Arc, Mutex};
use std::thread;

// Emu system
mod system;
use system::System;

fn main() {
    let mut system = Arc::new(Mutex::new(System::new()));

    let handle = thread::spawn(move || {
        let mut system = system.lock().unwrap();
        system.boot_system();
        for i in 0..200 {
            println!("Step");
            system.execute();
        }
    });

    handle.join().unwrap();
}
