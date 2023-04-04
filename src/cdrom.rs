use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::result::Result;

const SECTOR_SIZE: usize = 2354; // Size of a PSX CD sector
const SECTOR_HEADER_SIZE: usize = 24; // 24 bytes of header
const SECTOR_DATA_SIZE: usize = 2048; // 2048 of actual game data
const SECTOR_ECC_SIZE: usize = 282; // 282 of error dection and correction

pub struct CdRom {
    data: Vec<Sector>,
    index: usize,
}

struct Sector {
    header: [u8; SECTOR_HEADER_SIZE],
    data: [u8; SECTOR_DATA_SIZE],
    ecc: [u8; SECTOR_ECC_SIZE],
}

impl Sector {
    pub fn new() -> Self {
        Self {
            header: [0; SECTOR_HEADER_SIZE],
            data: [0; SECTOR_DATA_SIZE],
            ecc: [0; SECTOR_ECC_SIZE],
        }
    }
}

impl CdRom {
    pub fn from_file(filename: &str) -> Result<Self, Error> {
        let mut file = File::open(filename)?;
        let mut buffer = [0; SECTOR_SIZE];
        let mut sectors = Vec::new();
        loop {
            let mut sector = Sector::new();
            match file.read_exact(&mut buffer) {
                Ok(_) => {
                    sector.header.copy_from_slice(&buffer[0..24]);
                    sector.data.copy_from_slice(&buffer[24..2072]);
                    sector.ecc.copy_from_slice(&buffer[2072..2354]);
                    sectors.push(sector);
                }
                Err(ref err) if err.kind() == ErrorKind::UnexpectedEof => break,
                Err(err) => return Err(err),
            }
        }
        println!("Read {} sectors from file", sectors.len());
        Ok(Self {
            data: sectors,
            index: 0,
        })
    }
}
