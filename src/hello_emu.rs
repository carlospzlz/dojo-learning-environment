mod system;

use system::System;
use std::thread;

fn main() {
    let mut system = System::new();
    system.boot();
    system.start();
    thread::sleep(std::time::Duration::from_secs(1));
    system.stop();
    thread::sleep(std::time::Duration::from_secs(1));
    system.next_instruction();
    thread::sleep(std::time::Duration::from_secs(1));
    system.next_instruction();
    thread::sleep(std::time::Duration::from_secs(1));
    system.start();
    thread::sleep(std::time::Duration::from_secs(1));
    system.shutdown();
}
