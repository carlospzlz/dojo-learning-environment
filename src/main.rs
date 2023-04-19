//mod basic_reader;
mod cdrom;

//use basic_reader::read;
use cdrom::CdRom;

fn main() {
    // Read to memory
    let cdrom = CdRom::from_file("roms/tekken.bin").expect("Failed to read");

    // ECC
    cdrom.check_ecc();

    // Print info
    let game_title = cdrom.get_title().expect("Something went wrong");
    println!("Game Title: {}", game_title);
    let game_developer = cdrom.get_developer().expect("Something went wrong");
    println!("Game Developer: {}", game_developer);
    let game_publisher = cdrom.get_publisher().expect("Something went wrong");
    println!("Game Publisher: {}", game_publisher);
    let game_platform = cdrom.get_platform().expect("Something went wrong");
    println!("Game Platform: {}", game_platform);

    // Images
    cdrom.write_tim_images();
}
