use crate::system::bios::BIOS_SIZE;

use log::{debug, error, info, warn};
use std::cmp;

const PHYSICAL_MEMORY_ADDRESS_MASK: u32 = 0x1FFFFFFF;

// Memory Map
//
//  KUSEG     KSEG0     KSEG1
//  00000000h 80000000h A0000000h  2048K  Main RAM (first 64K reserved for BIOS)
//  1F000000h 9F000000h BF000000h  8192K  Expansion Region 1 (ROM/RAM)
//  1F800000h 9F800000h    --      1K     Scratchpad (D-Cache used as Fast RAM)
//  1F801000h 9F801000h BF801000h  4K     I/O Ports
//  1F802000h 9F802000h BF802000h  8K     Expansion Region 2 (I/O Ports)
//  1FA00000h 9FA00000h BFA00000h  2048K  Expansion Region 3 (SRAM BIOS region for DTL cards)
//  1FC00000h 9FC00000h BFC00000h  512K   BIOS ROM (Kernel) (4096K max)
//        FFFE0000h (in KSEG2)     0.5K   Internal CPU control registers (Cache Control)
//
// Kernel Memory: KSEG1 is the normal physical memory (uncached), KSEG0 is a
// mirror thereof (but with cache enabled). KSEG2 is usually intended to contain
// virtual kernel memory, but in the PSX it's containing Cache Control hardware
// registers.
//
// User Memory: KUSEG is intended to contain 2GB virtual memory (on extended
// MIPS processors), the PSX doesn't support virtual memory, and KUSEG simply
// contains a mirror of KSEG0/KSEG1
//
// 2MB RAM can be mirrored to the first 8MB (strangely, enabled by default)
//
// From https://psx-spx.consoledev.net/memorymap/

mod memory_map {
    //const RAM_BASE = 0x00000000,
    pub const RAM_2MB_SIZE: u32 = 0x200000; // 2048 KB
    pub const RAM_2MB_MASK: u32 = RAM_2MB_SIZE - 1;
    //const RAM_8MB_SIZE = 0x800000,
    //const RAM_8MB_MASK = RAM_8MB_SIZE - 1,
    pub const RAM_MIRROR_END: u32 = 0x800000; // 8 * 2^20 = 8 MB
    pub const EXP1_BASE: u32 = 0x1F000000;
    pub const EXP1_SIZE: u32 = 0x800000;
    pub const EXP1_MASK: u32 = EXP1_SIZE - 1;
    pub const MEMCTRL_BASE: u32 = 0x1F801000;
    pub const MEMCTRL_SIZE: u32 = 0x40; // 64 KB
    pub const MEMCTRL_MASK: u32 = MEMCTRL_SIZE - 1;
    //const PAD_BASE = 0x1F801040,
    //const PAD_SIZE = 0x10,
    //const PAD_MASK = PAD_SIZE - 1,
    //const SIO_BASE = 0x1F801050,
    //const SIO_SIZE = 0x10,
    //const SIO_MASK = SIO_SIZE - 1,
    //const MEMCTRL2_BASE = 0x1F801060,
    //const MEMCTRL2_SIZE = 0x10,
    //const MEMCTRL2_MASK = MEMCTRL2_SIZE - 1,
    //const INTERRUPT_CONTROLLER_BASE = 0x1F801070,
    //const INTERRUPT_CONTROLLER_SIZE = 0x10,
    //const INTERRUPT_CONTROLLER_MASK = INTERRUPT_CONTROLLER_SIZE - 1,
    //const DMA_BASE = 0x1F801080,
    //const DMA_SIZE = 0x80,
    //const DMA_MASK = DMA_SIZE - 1,
    //const TIMERS_BASE = 0x1F801100,
    //const TIMERS_SIZE = 0x40,
    //const TIMERS_MASK = TIMERS_SIZE - 1,
    //const CDROM_BASE = 0x1F801800,
    //const CDROM_SIZE = 0x10,
    //const CDROM_MASK = CDROM_SIZE - 1,
    //const GPU_BASE = 0x1F801810,
    //const GPU_SIZE = 0x10,
    //const GPU_MASK = GPU_SIZE - 1,
    //const MDEC_BASE = 0x1F801820,
    //const MDEC_SIZE = 0x10,
    //const MDEC_MASK = MDEC_SIZE - 1,
    //const SPU_BASE = 0x1F801C00,
    //const SPU_SIZE = 0x400,
    //const SPU_MASK = 0x3FF,
    //const EXP2_BASE = 0x1F802000,
    //const EXP2_SIZE = 0x2000,
    //const EXP2_MASK = EXP2_SIZE - 1,
    //const EXP3_BASE = 0x1FA00000,
    //const EXP3_SIZE = 0x1,
    //const EXP3_MASK = EXP3_SIZE - 1,
    pub const BIOS_BASE: u32 = 0x1FC00000;
    pub const BIOS_SIZE: u32 = 0x80000; // 512 KB
    pub const BIOS_MASK: u32 = 0x7FFFF;
}

pub struct Bus {
    pub bios: [u8; memory_map::BIOS_SIZE as usize],
    pub ram: [u8; memory_map::RAM_2MB_SIZE as usize],
    mem_ctrl_registers: MemCtrlRegisters,
    bios_access_time: AccessTimes,
    cdrom_access_time: AccessTimes,
}

type TickCount = i32;

struct AccessTimes {
    pub byte: TickCount,
    pub halfword: TickCount,
    pub word: TickCount,
}

struct MemDelayReg {
    //  0-3   Write Delay        (00h..0Fh=01h..10h Cycles)
    //  4-7   Read Delay         (00h..0Fh=01h..10h Cycles)
    //  8     Recovery Period    (0=No, 1=Yes, uses COM0 timings)
    //  9     Hold Period        (0=No, 1=Yes, uses COM1 timings)
    //  10    Floating Period    (0=No, 1=Yes, uses COM2 timings)
    //  11    Pre-strobe Period  (0=No, 1=Yes, uses COM3 timings)
    //  12    Data Bus-width     (0=8bits, 1=16bits)
    //  13    Auto Increment     (0=No, 1=Yes)
    //  14-15 Unknown (R/W)
    //  16-20 Memory Window Size (1 SHL N bytes) (0..1Fh = 1 byte ... 2 gigabytes)
    //  21-23 Unknown (always zero)
    //  24-27 DMA timing override
    //  28    Address error flag. Write 1 to it to clear it.
    //  29    DMA timing select  (0=use normal timings, 1=use bits 24-27)
    //  30    Wide DMA           (0=use bit 12, 1=override to full 32 bits)
    //  31    Wait               (1=wait on external device before being ready)
    bits: u32,
}

impl MemDelayReg {
    const WRITE_MASK: u32 = 0b1010_1111_0001_1111_1111_1111_1111_1111;

    fn get_access_time(&self) -> u8 {
        ((self.bits >> 4) & 0xF) as u8
    }

    fn uses_com0_time(&self) -> bool {
        ((self.bits >> 8) & 0x1) == 1
    }

    fn uses_com1_time(&self) -> bool {
        ((self.bits >> 9) & 0x1) == 1
    }

    fn uses_com2_time(&self) -> bool {
        ((self.bits >> 10) & 0x1) == 1
    }

    fn uses_com3_time(&self) -> bool {
        ((self.bits >> 11) & 0x1) == 1
    }

    fn is_data_bus_16bit(&self) -> bool {
        ((self.bits >> 12) & 0x1) == 1
    }

    fn get_memory_window_size(&self) -> u8 {
        ((self.bits >> 16) & 0x1F) as u8
    }
}

struct ComDelayReg {
    //  0-3   COM0 - Recovery period cycles
    //  4-7   COM1 - Hold period cycles
    //  8-11  COM2 - Floating release cycles
    //  12-15 COM3 - Strobe active-going edge delay
    //  16-31 Unknown/unused (read: always 0000h)
    bits: u32,
}

impl ComDelayReg {
    const WRITE_MASK: u32 = 0b0000_0000_0000_0011_1111_1111_1111_1111;

    fn get_com0_time(&self) -> u8 {
        ((self.bits >> 0) & 0xF) as u8
    }

    fn get_com1_time(&self) -> u8 {
        ((self.bits >> 4) & 0xF) as u8
    }

    fn get_com2_time(&self) -> u8 {
        ((self.bits >> 8) & 0xF) as u8
    }

    fn get_com3_time(&self) -> u8 {
        ((self.bits >> 12) & 0xF) as u8
    }

    fn get_comunk_time(&self) -> u8 {
        ((self.bits >> 16) & 0x3) as u8
    }
}

struct MemCtrlRegisters {
    // 1F801000 - Expansion 1 Base Address (usually 1F000000)
    // 1F801004 - Expansion 2 Base Address (usually 1F802000)
    // 1F801008 - Expansion 1 Delay/Size (usually 0013243F) (512Kbytes, 8bit bus)
    // 1F80100C - Expansion 3 Delay/Size (usually 00003022) (1 byte)
    // 1F801010 - BIOS ROM Delay/Size (usually 0013243F) (512Kbytes, 8bit bus)
    // 1F801014 - SPU Delay/Size (200931E1) (use 220931E1h for SPU-RAM reads)
    // 1F801018 - CDROM Delay/Size (00020843 or 00020943)
    // 1F80101C - Expansion 2 Delay/Size (usually 00070777) (128 bytes, 8bit bus)
    // 1F801020 - COM_DELAY / COMMON_DELAY (00031125 or 0000132C or 00001325)
    regs: [u32; 9],
}

impl MemCtrlRegisters {
    fn new() -> Self {
        Self {
            regs: [
                0x1F000000, 0x1F802000, 0x0013243F, 0x00003022, 0x0013243F, 0x200931E1, 0x00020843,
                0x00070777, 0x00031125,
            ],
        }
    }

    //fn write_register(&mut self, index: u8, value: u32) -> () {
    //    if index > 9 {
    //        panic!("Memory Control Register Out of bound! {} > 9", index);
    //    }
    //    self.regs[index as usize] = value;
    //}

    fn get_exp1_base(&self) -> u32 {
        self.regs[0]
    }

    fn get_exp2_base(&self) -> u32 {
        self.regs[1]
    }

    fn get_exp1_delay_size(&self) -> MemDelayReg {
        MemDelayReg { bits: self.regs[2] }
    }

    fn get_exp3_delay_size(&self) -> MemDelayReg {
        MemDelayReg { bits: self.regs[3] }
    }

    fn get_bios_delay_size(&self) -> MemDelayReg {
        MemDelayReg { bits: self.regs[4] }
    }

    fn get_spu_delay_size(&self) -> MemDelayReg {
        MemDelayReg { bits: self.regs[5] }
    }

    fn get_cdrom_delay_size(&self) -> MemDelayReg {
        MemDelayReg { bits: self.regs[6] }
    }

    fn get_exp2_delay_size(&self) -> MemDelayReg {
        MemDelayReg { bits: self.regs[7] }
    }
    fn get_common_delay(&self) -> ComDelayReg {
        ComDelayReg { bits: self.regs[8] }
    }
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bios: [0; memory_map::BIOS_SIZE as usize],
            ram: [0; memory_map::RAM_2MB_SIZE as usize],
            mem_ctrl_registers: MemCtrlRegisters::new(),
            bios_access_time: AccessTimes {
                byte: 0,
                halfword: 0,
                word: 0,
            },
            cdrom_access_time: AccessTimes {
                byte: 0,
                halfword: 0,
                word: 0,
            },
        }
    }

    pub fn initialize(&mut self) {
        self.recalculate_memory_timings();
    }

    pub fn fetch_instruction(&self, address: u32) -> Result<u32, String> {
        let tag = address >> 29;
        match tag {
            // 0x00: KUSEG    0M- 512M
            // 0x01: KUSEG  512M-1024M
            // 0x02: KUSEG 1024M-1536M
            // 0x03: KUSEG 1536M-2048M
            // 0x04: KSEG  Physical Memory Cached
            // 0x05: KSEG  Physical Memory Uncached
            // 0x06: KSEG2
            // 0x07: KSEG2
            0x00 | 0x04 => self.do_instruction_read(address),
            0x05 => self.do_instruction_read(address),
            _ => panic!("Address out of bounds: {:x}", address),
        }
    }

    fn do_instruction_read(&self, address: u32) -> Result<u32, String> {
        let address = address & PHYSICAL_MEMORY_ADDRESS_MASK;

        // RAM
        if address < memory_map::RAM_MIRROR_END {
            //println!("Address: {:x} (RAM)", address);
            let address = address & memory_map::RAM_2MB_MASK;
            panic!("Dafuq: Reading instruction in RAM?");
            return Ok(self.ram[address as usize] as u32);
        }

        // Mapped BIOS
        if address >= (memory_map::BIOS_BASE)
            && address < (memory_map::BIOS_BASE + memory_map::BIOS_SIZE)
        {
            //println!("Address: {:x} (BIOS)", address);
            let address = ((address - memory_map::BIOS_BASE) & memory_map::BIOS_MASK) as usize;
            // R3000A is little endian! So the most significant bytes are
            // stored in lower memory addresses.
            // Funny enough, if you brutely read a u32 in C++ on the host
            // (which usually is little endian), bytes will be arranged like
            // [3, 2, 1, 0] and the instruction will be formed correctly.
            let instruction: u32 = ((self.bios[address + 3] as u32) << 24)
                | ((self.bios[address + 2] as u32) << 16)
                | ((self.bios[address + 1] as u32) << 8)
                | ((self.bios[address + 0] as u32) << 0);
            return Ok(instruction);
        }

        panic!("Can't read instruction: {}", address)
    }

    pub fn read_memory_word(&self, address: u32) -> u32 {
        let tag = address >> 29;
        match tag {
            // 0x00: KUSEG    0M- 512M
            // 0x01: KUSEG  512M-1024M
            // 0x02: KUSEG 1024M-1536M
            // 0x03: KUSEG 1536M-2048M
            // 0x04: KSEG  Physical Memory Cached
            // 0x05: KSEG  Physical Memory Uncached
            // 0x06: KSEG2
            // 0x07: KSEG2
            0x00 | 0x04 => {
                error!("Read cached memory");
                self.do_memory_word_read(address)
            }
            0x01 | 0x02 | 0x03 => panic!("Exception: Reading above 512 MB"),
            0x06 | 0x07 => todo!("Weird case"),
            0x05 => self.do_memory_word_read(address),
            _ => panic!("Address out of bounds: {:x}", address),
        }
    }

    fn do_memory_word_read(&self, address: u32) -> u32 {
        debug!("do_memory_read {:x}", address);
        let address = address & PHYSICAL_MEMORY_ADDRESS_MASK;

        if address < memory_map::RAM_MIRROR_END {
            debug!("Reading memory: RAM_MIRROR_END");
            return self.do_ram_word_read(address);
        } else if address >= memory_map::BIOS_BASE
            && address < (memory_map::BIOS_BASE + memory_map::BIOS_SIZE)
        {
            debug!("Reading memory: BIOS");
        } else if address < memory_map::EXP1_BASE {
            debug!("Reading memory: BIOS");
        } else {
            todo!("Reading memory");
        }

        ////// RAM
        //if address < memory_map::RAM_MIRROR_END {
        //    //println!("Address: {:x} (RAM)", address);
        //    let address = address & memory_map::RAM_2MB_MASK;
        //    debug_assert!(false);
        //    return Ok(self.ram[address as usize] as u32);
        //}

        //// Mapped BIOS
        //if address >= (memory_map::BIOS_BASE)
        //    && address < (memory_map::BIOS_BASE + memory_map::BIOS_SIZE)
        //{
        //    //println!("Address: {:x} (BIOS)", address);
        //    let address = ((address - memory_map::BIOS_BASE) & memory_map::BIOS_MASK) as usize;
        //    // R3000A is little endian! So the most significant bytes are
        //    // stored in lower memory addresses.
        //    // Funny enough, if you brutely read a u32 in C++ on the host
        //    // (which usually is little endian), bytes will be arranged like
        //    // [3, 2, 1, 0] and the instruction will be formed correctly.
        //    let instruction: u32 = ((self.bios[address + 3] as u32) << 24)
        //        | ((self.bios[address + 2] as u32) << 16)
        //        | ((self.bios[address + 1] as u32) << 8)
        //        | ((self.bios[address + 0] as u32) << 0);
        //    return Ok(instruction);
        //}

        panic!("Can't read memory: {}", address)
    }

    fn do_ram_word_read(&self, address: u32) -> u32 {
        let offset = address & memory_map::RAM_2MB_MASK;
        // Little endian
        ((self.ram[(offset + 3) as usize] as u32) << 24)
            | ((self.ram[(offset + 2) as usize] as u32) << 16)
            | ((self.ram[(offset + 1) as usize] as u32) << 8)
            | ((self.ram[(offset + 0) as usize] as u32) << 0)
    }

    pub fn write_memory_word(&mut self, address: u32, value: u32) -> () {
        let tag = address >> 29;
        match tag {
            // 0x00: KUSEG    0M- 512M
            // 0x01: KUSEG  512M-1024M
            // 0x02: KUSEG 1024M-1536M
            // 0x03: KUSEG 1536M-2048M
            // 0x04: KSEG  Physical Memory Cached
            // 0x05: KSEG  Physical Memory Uncached
            // 0x06: KSEG2
            // 0x07: KSEG2
            0x00 | 0x04 => {
                warn!("Word should be written to cache");
                self.do_memory_word_write(address, value)
            }
            0x01 | 0x02 | 0x03 => panic!("Exception: Writing above 512 MB"),
            0x06 | 0x07 => error!("Weird case"),
            0x05 => self.do_memory_word_write(address, value),
            _ => panic!("Address out of bounds: {:x}", address),
        }
    }

    fn do_memory_word_write(&mut self, address: u32, value: u32) -> () {
        let address = address & PHYSICAL_MEMORY_ADDRESS_MASK;

        if address < memory_map::RAM_MIRROR_END {
            debug!("Writing memory: RAM_MIRROR_END");
            return self.do_ram_word_write(address, value);
        } else if address >= memory_map::BIOS_BASE
            && address < (memory_map::BIOS_BASE + memory_map::BIOS_SIZE)
        {
            debug!("Writing memory: BIOS");
        } else if address < memory_map::EXP1_BASE {
            panic!("Invalid Address: BIOS < address < EXP1_BASE");
        } else if address < (memory_map::EXP1_BASE + memory_map::EXP1_SIZE) {
            todo!("EXP1 access");
        } else if address < memory_map::MEMCTRL_BASE {
            panic!("Invalid Address: EXP1 < address < MEMCTRL_BASE");
        } else if address < (memory_map::MEMCTRL_BASE + memory_map::MEMCTRL_SIZE) {
            self.do_memory_control_word_write(address, value);
        } else {
            debug!("Writing memory");
        }
    }

    fn do_ram_word_write(&mut self, address: u32, value: u32) -> () {
        let address = address & memory_map::RAM_2MB_MASK;
        // Little endian
        self.ram[(address + 0) as usize] = ((value >> 0) & 0xFF) as u8;
        self.ram[(address + 1) as usize] = ((value >> 8) & 0xFF) as u8;
        self.ram[(address + 2) as usize] = ((value >> 16) & 0xFF) as u8;
        self.ram[(address + 3) as usize] = ((value >> 24) & 0xFF) as u8;
    }

    fn do_memory_control_word_write(&mut self, address: u32, value: u32) -> () {
        debug!("do_memory_control_word_write");
        let address = address & memory_map::MEMCTRL_MASK;
        let index = address / 4;
        let write_mask = if index == 8 {
            ComDelayReg::WRITE_MASK
        } else {
            MemDelayReg::WRITE_MASK
        };
        let new_value =
            (self.mem_ctrl_registers.regs[index as usize] & !write_mask) | (value & write_mask);
        if self.mem_ctrl_registers.regs[index as usize] != new_value {
            self.mem_ctrl_registers.regs[index as usize] = new_value;
            self.recalculate_memory_timings();
        }
    }

    fn recalculate_memory_timings(&mut self) -> () {
        debug!("Recalculating memory timings!");
        // BIOS
        (
            self.bios_access_time.byte,
            self.bios_access_time.halfword,
            self.bios_access_time.word,
        ) = self.calculate_memory_timing(
            self.mem_ctrl_registers.get_bios_delay_size(),
            self.mem_ctrl_registers.get_common_delay(),
        );
        // CDROM
        (
            self.cdrom_access_time.byte,
            self.cdrom_access_time.halfword,
            self.cdrom_access_time.word,
        ) = self.calculate_memory_timing(
            self.mem_ctrl_registers.get_cdrom_delay_size(),
            self.mem_ctrl_registers.get_common_delay(),
        );
        // SPU ignored

        let data_bus = if self
            .mem_ctrl_registers
            .get_bios_delay_size()
            .is_data_bus_16bit()
        {
            16
        } else {
            8
        };
        info!(
            "BIOS Memory Timing: {} bit bus, byte={}, halfword={}, word={}",
            data_bus,
            self.bios_access_time.byte + 1,
            self.bios_access_time.halfword + 1,
            self.bios_access_time.word + 1
        );

        let data_bus = if self
            .mem_ctrl_registers
            .get_cdrom_delay_size()
            .is_data_bus_16bit()
        {
            16
        } else {
            8
        };
        info!(
            "CDROM Memory Timing: {} bit bus, byte={}, halfword={}, word={}",
            data_bus,
            self.cdrom_access_time.byte + 1,
            self.cdrom_access_time.halfword + 1,
            self.cdrom_access_time.word + 1
        );
    }

    fn calculate_memory_timing(
        &self,
        mem_delay: MemDelayReg,
        common_delay: ComDelayReg,
    ) -> (TickCount, TickCount, TickCount) {
        // From Duckstation project
        let mut first = 0;
        let mut seq = 0;
        let mut min = 0;
        if mem_delay.uses_com0_time() {
            first += (common_delay.get_com0_time() as i8 as i32) - 1;
            seq += (common_delay.get_com0_time() as i8 as i32) - 1;
        }
        if mem_delay.uses_com2_time() {
            first += common_delay.get_com2_time() as i8 as i32;
            seq += common_delay.get_com2_time() as i8 as i32;
        }
        if mem_delay.uses_com3_time() {
            min = common_delay.get_com3_time() as i8 as i32;
        }
        first += (first < 6) as i32;

        first = first + (mem_delay.get_access_time() as i8 as i32) + 2;
        seq = seq + (mem_delay.get_access_time() as i8 as i32) + 2;

        if first < (min + 6) {
            first = min + 6;
        }
        if seq < (min + 2) {
            seq = min + 2;
        }

        let byte_access_time: TickCount = first;
        let halfword_access_time: TickCount = if mem_delay.is_data_bus_16bit() {
            first
        } else {
            first + seq
        };
        let word_access_time: TickCount = if mem_delay.is_data_bus_16bit() {
            first + seq
        } else {
            first + 3 * seq
        };

        (
            cmp::max(byte_access_time - 1, 0),
            cmp::max(halfword_access_time - 1, 0),
            cmp::max(word_access_time - 1, 0),
        )
    }

    pub fn write_memory_half_word(&self, address: u32, value: u16) -> Result<u32, String> {
        let tag = address >> 29;
        match tag {
            // 0x00: KUSEG    0M- 512M
            // 0x01: KUSEG  512M-1024M
            // 0x02: KUSEG 1024M-1536M
            // 0x03: KUSEG 1536M-2048M
            // 0x04: KSEG  Physical Memory Cached
            // 0x05: KSEG  Physical Memory Uncached
            // 0x06: KSEG2
            // 0x07: KSEG2
            0x00 | 0x04 => {
                warn!("Read cached memory");
                Ok(0)
            }
            0x01 | 0x02 | 0x03 => panic!("Exception: Reading above 512 MB"),
            0x06 | 0x07 => todo!("Weird case"),
            0x05 => self.do_memory_half_word_read(address, value),
            _ => panic!("Address out of bounds: {:x}", address),
        }
    }

    fn do_memory_half_word_read(&self, address: u32, value: u16) -> Result<u32, String> {
        Ok(0)
    }

    pub fn dump_ram(&self) -> () {
        println!("RAM state");
        for i in 0..memory_map::RAM_2MB_SIZE {
            let byte = self.ram[i as usize];
            if byte != 0x00 {
                print!("{:02x}({:x}) ", byte, i);
            }
        }
        println!();
        println!("---");
    }

    pub fn dump_mem_ctrl_registers(&self) -> () {
        println!("Memory Control Registers state");
        let r = self.mem_ctrl_registers.regs;
        println!("{:8x} {:8x} {:8x} {:8x}", r[0], r[1], r[2], r[3]);
        println!("{:8x} {:8x} {:8x} {:8x}", r[4], r[5], r[6], r[7]);
        println!("{:8x}", r[8]);

        println!("Access Times");
        println!("{} {} {}", self.bios_access_time.byte, self.bios_access_time.halfword, self.bios_access_time.word);
        println!("{} {} {}", self.cdrom_access_time.byte, self.cdrom_access_time.halfword, self.cdrom_access_time.word);

        println!("---");
    }
}
