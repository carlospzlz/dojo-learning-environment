//mod basic_reader;
mod cdrom;

//use basic_reader::read;
use cdrom::CdRom;

fn main() {
    let _cdrom = CdRom::from_file("roms/tekken.bin");
}
