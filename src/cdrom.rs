use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::result::Result;

//use tim2png::{Tim, TimDecoder};

const SECTOR_SIZE: usize = 2354; // Size of a PSX CD sector
const SECTOR_HEADER_SIZE: usize = 24; // 24 bytes of header
const SECTOR_DATA_SIZE: usize = 2048; // 2048 of actual game data
const SECTOR_ECC_SIZE: usize = 282; // 282 of error dection and correction

const SYSTEM_IDENTIFIER_SECTOR: usize = 16;
const GAME_TITLE_OFFSET: usize = 8;
const GAME_DEVELOPER_OFFSET: usize = 286;
const GAME_PUBLISHER_OFFSET: usize = 414;
const GAME_PLATFORM_OFFSET: usize = 542;
const SYSTEM_INFO_LENGTH: usize = 20;

pub struct CdRom {
    sectors: Vec<Sector>,
    //index: usize,
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
            sectors: sectors,
            //index: 0,
        })
    }

    pub fn check_ecc(&self) -> Result<bool, String> {
        for i in 0..self.sectors.len() {
            let sector = &self.sectors[i];
            let ecc: u32 = sector.ecc.iter().fold(0, |acc, &x| acc + x as u32);
            if ecc != 0 {
                let err_msg = format!("Sector {} has ECC: {}", i, ecc);
                return Err(err_msg.to_string());
            }
        }
        Ok(true)
    }

    fn get_system_info(&self, offset: usize, length: usize) -> Result<String, String> {
        let sector = &self.sectors[SYSTEM_IDENTIFIER_SECTOR];
        let system_info_bytes = &sector.data[offset..offset + length];
        let system_info = String::from_utf8_lossy(system_info_bytes);
        let system_info = system_info.trim_end_matches(char::from(0));
        Ok(system_info.to_string())
    }

    pub fn get_title(&self) -> Result<String, String> {
        self.get_system_info(GAME_TITLE_OFFSET, SYSTEM_INFO_LENGTH)
    }

    pub fn get_developer(&self) -> Result<String, String> {
        self.get_system_info(GAME_DEVELOPER_OFFSET, SYSTEM_INFO_LENGTH)
    }

    pub fn get_publisher(&self) -> Result<String, String> {
        self.get_system_info(GAME_PUBLISHER_OFFSET, SYSTEM_INFO_LENGTH)
    }

    pub fn get_platform(&self) -> Result<String, String> {
        self.get_system_info(GAME_PLATFORM_OFFSET, SYSTEM_INFO_LENGTH)
    }

    pub fn write_tim_images(&self) -> Result<bool, String> {
        for i in 0..self.sectors.len() {
            let sector = &self.sectors[i];
            for j in 0..(sector.data.len() - 4) {
                if &sector.data[j..(j + 4)] == b"TIM1" {
                    println!("Sector {}, Byte {}: Bingo!", i, j);
                    let chunk = String::from_utf8_lossy(&sector.data[j..(j + 100)]);
                    println!("Data: {}", chunk);
                    // Parse
                    let img = self.parse_tim_image(i, j);
                }
            }
        }
        Ok(true)
    }

    fn parse_color_palette(data: &[u8]) -> Vec<image::Rgb<u8>> {
        let mut colors = Vec::with_capacity(data.len() / 2);
        for i in (0..data.len()).step_by(2) {
            let color = u16::from_be_bytes([data[i], data[i + 1]]);
            let r = ((color >> 10) & 0x1F) as u8;
            let g = ((color >> 5) & 0x1F) as u8;
            let b = (color & 0x1F) as u8;
            colors.push(image::Rgb([r << 3, g << 3, b << 3]));
        }
        colors
    }

    fn parse_tim_image(&self, sector_index: usize, byte_offset: usize) -> Option<image::RgbImage> {
        let sector_data = &self.sectors[sector_index].data[byte_offset..SECTOR_DATA_SIZE];

        // Parse the header
        let width = u16::from_be_bytes([sector_data[4], sector_data[5]]) as usize;
        println!("Width: {}", width);
        let height = u16::from_be_bytes([sector_data[6], sector_data[7]]) as usize;
        let clut_offset = u32::from_be_bytes([
            sector_data[8],
            sector_data[9],
            sector_data[10],
            sector_data[11],
        ]) as usize;
        println!("Height: {}", height);
        let clut_len = u32::from_be_bytes([
            sector_data[12],
            sector_data[13],
            sector_data[14],
            sector_data[15],
        ]) as usize;
        let pixel_data_offset = u32::from_be_bytes([
            sector_data[16],
            sector_data[17],
            sector_data[18],
            sector_data[19],
        ]) as usize;
        let pixel_data_len = width * height;

        // Parse the color palette
        let mut clut_data = vec![0u8; clut_len];
        clut_data.copy_from_slice(&sector_data[clut_offset..(clut_offset + clut_len)]);
        let clut = Self::parse_color_palette(&clut_data);

        // Parse the pixel data
        let pixel_data = &sector_data[pixel_data_offset..(pixel_data_offset + pixel_data_len)];
        let mut pixels = Vec::with_capacity(pixel_data_len * 3);
        for pixel in pixel_data.iter() {
            let color = clut[*pixel as usize];
            pixels.push(color[0]);
            pixels.push(color[1]);
            pixels.push(color[2]);
        }

        // Construct the image
        let img = image::RgbImage::from_raw(width as u32, height as u32, pixels);
        img
    }

}
