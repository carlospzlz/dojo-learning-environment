mod bin;
mod no_disk;

use std::path;

pub trait Container {
    #[allow(dead_code)]
    fn open(filepath: &path::Path) -> Result<Box<Self>, String>;
    #[allow(dead_code)]
    fn read(&mut self, lba: usize, buffer: &mut [u8; 2352]) -> Result<(), String>;
}
