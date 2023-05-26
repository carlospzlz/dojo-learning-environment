use log::{debug, error, info, warn};

//mod basic_reader;
//mod cdrom;
//mod bios;
mod system;

//use basic_reader::read;
//use cdrom::CdRom;
//use bios::Bios;
use system::System;

fn main() {
    env_logger::init();

    //// Read to memory
    //let cdrom = CdRom::from_file("roms/tekken.bin").expect("Failed to read");

    //// ECC
    //cdrom.check_ecc();

    //// Print info
    //let game_title = cdrom.get_title().expect("Something went wrong");
    //println!("Game Title: {}", game_title);
    //let game_developer = cdrom.get_developer().expect("Something went wrong");
    //println!("Game Developer: {}", game_developer);
    //let game_publisher = cdrom.get_publisher().expect("Something went wrong");
    //println!("Game Publisher: {}", game_publisher);
    //let game_platform = cdrom.get_platform().expect("Something went wrong");
    //println!("Game Platform: {}", game_platform);

    //// Images
    //cdrom.write_tim_images();

    // Read BIOS
    //let bios = Bios::from_file("bios/scph1001.bin").expect("Something went wrong!");

    // Get hash
    //let hash = bios.get_hash();
    //println!("BIOS hash: {:x}", hash);

    // System
    let mut system = System::new();
    system.boot_system();
    for i in 0..25000 {
        system.execute();
    }
}
