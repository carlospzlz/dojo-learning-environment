use generic_array::{typenum::U16, GenericArray};
use md5::{Digest, Md5};
use std::fs::File;
use std::io::{Error, Read};
use std::result::Result;

const BIOS_SIZE: usize = 0x80000; // Size of the bios, 524288 bytes (512 KB)
const HASH: &'static str = "924e392ed05558ffdb115408c263dccf"; // SCPH-1001 NTSC_U

pub struct Bios {
    bytes: [u8; BIOS_SIZE],
}

impl Bios {
    pub fn from_file(filename: &str) -> Result<Self, Error> {
        let mut file = File::open(filename)?;
        let mut bytes = [0; BIOS_SIZE];
        file.read_exact(&mut bytes)?;
        println!("Read {} bytes from file", bytes.len());
        Ok(Self { bytes: bytes })
    }

    pub fn get_hash(&self) -> GenericArray<u8, U16> {
        let mut hasher = Md5::new();
        hasher.update(self.bytes);
        hasher.finalize()
    }

    pub fn data(&self) -> [u8; BIOS_SIZE] {
        self.bytes
    }
}
