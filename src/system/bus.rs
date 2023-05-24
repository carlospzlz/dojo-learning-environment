use crate::system::bios::BIOS_SIZE;

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

mod MemoryMap {
    //const RAM_BASE = 0x00000000,
    pub const RAM_2MB_SIZE: u32 = 0x200000; // 2048 KB
    pub const RAM_2MB_MASK: u32 = RAM_2MB_SIZE - 1;
    //const RAM_8MB_SIZE = 0x800000,
    //const RAM_8MB_MASK = RAM_8MB_SIZE - 1,
    pub const RAM_MIRROR_END: u32 = 0x800000;
    //const EXP1_BASE = 0x1F000000,
    //const EXP1_SIZE = 0x800000,
    //const EXP1_MASK = EXP1_SIZE - 1,
    //const MEMCTRL_BASE = 0x1F801000,
    //const MEMCTRL_SIZE = 0x40,
    //const MEMCTRL_MASK = MEMCTRL_SIZE - 1,
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
    pub bios: [u8; MemoryMap::BIOS_SIZE as usize],
    pub ram: [u8; MemoryMap::RAM_2MB_SIZE as usize],
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bios: [0; MemoryMap::BIOS_SIZE as usize],
            ram: [0; MemoryMap::RAM_2MB_SIZE as usize],
        }
    }

    pub fn initialize(&self) {}

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
        if address < MemoryMap::RAM_MIRROR_END {
            //println!("Address: {:x} (RAM)", address);
            let address = address & MemoryMap::RAM_2MB_MASK;
            debug_assert!(false);
            return Ok(self.ram[address as usize] as u32);
        }

        // Mapped BIOS
        if address >= (MemoryMap::BIOS_BASE)
            && address < (MemoryMap::BIOS_BASE + MemoryMap::BIOS_SIZE)
        {
            //println!("Address: {:x} (BIOS)", address);
            let address = ((address - MemoryMap::BIOS_BASE) & MemoryMap::BIOS_MASK) as usize;
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

    pub fn read_memory(address: u32) -> Result<u32, String> {
        Ok(0)
    }
}
