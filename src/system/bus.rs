use crate::system::cpu::CPU;
use crate::system::dma::DMA;

use log::{debug, error, info, warn};
use sha2::{Digest, Sha256};
use std::cmp;

use crate::system::interrupt_controller::InterruptController;

type TickCount = i32;

const PHYSICAL_MEMORY_ADDRESS_MASK: u32 = 0x1FFFFFFF;
const RAM_READ_TICKS: TickCount = 6;

// Memory Map
// ============================================================================
//
//  KUSEG     KSEG0     KSEG1
//  00000000 80000000 A0000000  2048K  Main RAM (first 64K reserved for BIOS)
//  1F000000 9F000000 BF000000  8192K  Expansion Region 1 (ROM/RAM)
//  1F800000 9F800000    --     1K     Scratchpad (D-Cache used as Fast RAM)
//  1F801000 9F801000 BF801000  4K     I/O Ports
//  1F802000 9F802000 BF802000  8K     Expansion Region 2 (I/O Ports)
//  1FA00000 9FA00000 BFA00000  2048K  Expansion Region 3 (SRAM BIOS region for DTL cards)
//  1FC00000 9FC00000 BFC00000  512K   BIOS ROM (Kernel) (4096K max)
//
//                      KSEG2
//                    FFFE0000  0.5K   Internal CPU control registers (Cache Control)
//
// Kernel Memory: KSEG1 is the normal physical memory (uncached), KSEG0 is a
// mirror thereof (but with cache enabled). KSEG2 is usually intended to contain
// virtual kernel memory, but in the PSX it's containing Cache Control hardware
// registers.
//
// User Memory: KUSEG is intended to contain 2GB virtual memory (on extended
// MIPS processors), the PSX doesn't support virtual memory, and KUSEG simply
// contains a mirror of KSEG0/KSEG1.
//
// As described above, the 512Mbyte KUSEG, KSEG0, and KSEG1 regions are mirrors
// of each other. Additional mirrors within these 512MB regions are:
//
//  - 2MB RAM can be mirrored to the first 8MB (strangely, enabled by default)
//  - 512K BIOS ROM can be mirrored to the last 4MB (disabled by default)
//  - Expansion hardware (if any) may be mirrored within expansion region
//  - The seven DMA Control Registers at 1F8010x8h are mirrored to 1F8010xCh
//
// You will see this very often in the code. The address tag is the first 3 bits.
//
// Tag: 0x00 => Address: 0x00000000    KUSEG    0                   0M-512M      512M    User Memory
// Tag: 0x01 => Address: 0x20000000    KUSEG    2^29 = 512M         512M-1024M   512M    User Memory
// Tag: 0x02 => Address: 0x40000000    KUSEG    2^30 = 1024M        1024-1536M   512M    User Memory
// Tag: 0x03 => Address: 0x60000000    KUSEG    3 * 2^29 = 1536M    1536M-2048M  512M    User Memory
// Tag: 0x04 => Address: 0x80000000    KSEG0    2 * 2^30 = 2024M    2048M-2560M  512M    Kernel Memory (Physical Memory Cached)
// Tag: 0x05 => Address: 0xA0000000    KSEG1    5 * 2^29 = 2560M    2560M-3072M  512M    Kernel Memory (Physical Memory Uncached)
// Tag: 0x06 => Address: 0xC0000000    KSEG2    3 * 2^30 = 3072M    3072M-3584M  512M    Kernel Memory (Cache Control)
// Tag: 0x07 => Address: 0xE0000000    KSEG2    7 * 2^29 = 3584M    3584M-END     -      Kernel Memory (Cache Control)
//
// From https://psx-spx.consoledev.net/memorymap/

// NOTES:
//
// R3000A is little endian! So the most significant bytes are stored in lower
// memory addresses. Funny enough, if you brutely read a u32 in C++ on the host
// (which usually is little endian), bytes will be arranged like [3, 2, 1, 0]
// and the word will be formed correctly.

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
    pub const PAD_BASE: u32 = 0x1F801040;
    pub const PAD_SIZE: u32 = 0x10;
    pub const PAD_MASK: u32 = PAD_SIZE - 1;
    pub const SIO_BASE: u32 = 0x1F801050;
    pub const SIO_SIZE: u32 = 0x10;
    pub const SIO_MASK: u32 = SIO_SIZE - 1;
    pub const MEMCTRL2_BASE: u32 = 0x1F801060;
    pub const MEMCTRL2_SIZE: u32 = 0x10;
    pub const MEMCTRL2_MASK: u32 = MEMCTRL2_SIZE - 1;
    pub const INTERRUPT_CONTROLLER_BASE: u32 = 0x1F801070;
    pub const INTERRUPT_CONTROLLER_SIZE: u32 = 0x10;
    pub const INTERRUPT_CONTROLLER_MASK: u32 = INTERRUPT_CONTROLLER_SIZE - 1;
    pub const DMA_BASE: u32 = 0x1F801080;
    pub const DMA_SIZE: u32 = 0x80;
    pub const DMA_MASK: u32 = DMA_SIZE - 1;
    pub const TIMERS_BASE: u32 = 0x1F801100;
    pub const TIMERS_SIZE: u32 = 0x40;
    pub const TIMERS_MASK: u32 = TIMERS_SIZE - 1;
    pub const CDROM_BASE: u32 = 0x1F801800;
    pub const CDROM_SIZE: u32 = 0x10;
    pub const CDROM_MASK: u32 = CDROM_SIZE - 1;
    pub const GPU_BASE: u32 = 0x1F801810;
    pub const GPU_SIZE: u32 = 0x10;
    pub const GPU_MASK: u32 = GPU_SIZE - 1;
    pub const MDEC_BASE: u32 = 0x1F801820;
    pub const MDEC_SIZE: u32 = 0x10;
    pub const MDEC_MASK: u32 = MDEC_SIZE - 1;
    pub const SPU_BASE: u32 = 0x1F801C00;
    pub const SPU_SIZE: u32 = 0x400;
    pub const SPU_MASK: u32 = SPU_SIZE - 1;
    pub const EXP2_BASE: u32 = 0x1F802000;
    pub const EXP2_SIZE: u32 = 0x2000;
    pub const EXP2_MASK: u32 = EXP2_SIZE - 1;
    pub const EXP3_BASE: u32 = 0x1FA00000;
    pub const EXP3_SIZE: u32 = 0x1;
    pub const EXP3_MASK: u32 = EXP3_SIZE - 1;
    pub const BIOS_BASE: u32 = 0x1FC00000;
    pub const BIOS_SIZE: u32 = 0x80000; // 512 KB
    pub const BIOS_MASK: u32 = BIOS_SIZE - 1;
}

// Data Cache related constants
mod d_cache {
    pub const LOCATION: usize = 0x1F800000;
    pub const LOCATION_MASK: usize = 0xFFFFFC00;
    pub const OFFSET_MASK: usize = 0x000003FF;
    pub const SIZE: usize = 0x400; // 1KB
}

// Instruction Cache related constants
mod i_cache {
    pub const SIZE: usize = 0x1000; // 4KB
    pub const SLOTS: usize = SIZE / std::mem::size_of::<u32>();
    pub const LINE_SIZE: usize = 16;
    pub const LINES: usize = SIZE / LINE_SIZE;
    pub const SLOTS_PER_LINE: usize = SLOTS / LINES;
    pub const TAG_ADDRESS_MASK: usize = 0xFFFFFFF0;
    pub const INVALID_BITS: usize = 0x0F;
}

pub struct Bus {
    pub bios: [u8; memory_map::BIOS_SIZE as usize],
    pub ram: [u8; memory_map::RAM_2MB_SIZE as usize],
    mem_ctrl_registers: MemCtrlRegisters,
    bios_access_time: AccessTimes,
    cdrom_access_time: AccessTimes,

    // Caches
    icache_tags: [u32; i_cache::LINES],
    icache_data: [u8; i_cache::SIZE],
    dcache: [u8; d_cache::SIZE],

    // Controllers
    interrupt_controller: InterruptController,

    // Direct Memory Access
    dma: DMA,
}

// Unit-like structs for the Memory Access trait
pub struct ReadByte;
pub struct ReadHalfWord;
pub struct ReadWord;
pub struct WriteByte;
pub struct WriteHalfWord;
pub struct WriteWord;

pub trait MemoryAccess {
    fn do_dma_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount;
    fn do_interrupt_controller_access(address: u32, value: &mut u32, bus: &mut Bus, cpu : &mut CPU) -> TickCount;
    fn do_ram_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount;
    fn do_memory_control_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount;
    fn do_bios_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount;
    fn is_write() -> bool;
}

impl MemoryAccess for ReadByte {
    fn do_dma_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = address & memory_map::DMA_MASK;
        // Remove "byte index" in word
        let word_offset = offset & !0x3;
        let word = bus.dma.read_register(word_offset);
        // Shift as many bytes as needed;
        *value = word >> ((offset & 0x3) * 8);
        2
    }

    fn do_interrupt_controller_access(address: u32, value: &mut u32, bus: &mut Bus, cpu: &mut CPU) -> TickCount {
        let offset = address & memory_map::INTERRUPT_CONTROLLER_MASK;
        // Remove "byte index" in word
        let word_offset = offset & !0x3;
        let word = bus.interrupt_controller.read_register(word_offset);
        // Shift as many bytes as needed;
        *value = word >> ((offset & 0x3) * 8);
        2
    }

    fn do_ram_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = address & memory_map::RAM_2MB_MASK;
        *value = bus.ram[offset as usize] as u32;
        RAM_READ_TICKS
    }

    fn do_memory_control_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = address & memory_map::MEMCTRL_MASK;
        // Each register is 4 bytes
        let index = offset >> 2;
        *value = bus.mem_ctrl_registers.regs[index as usize];
        2
    }

    fn do_bios_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = ((address - memory_map::BIOS_BASE) & memory_map::BIOS_MASK) as usize;
        *value = bus.bios[offset as usize] as u32;
        bus.bios_access_time.byte
    }

    fn is_write() -> bool {
        false
    }
}

impl MemoryAccess for ReadHalfWord {
    fn do_dma_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        assert!((address & 0x1) == 0);
        let offset = address & memory_map::DMA_MASK;
        // Remove "half-word index" in word
        let word_offset = offset & !0x2;
        let word = bus.dma.read_register(word_offset);
        // Shift as many half-words as needed;
        *value = word >> ((offset & 0x2) * 16);
        2
    }

    fn do_interrupt_controller_access(address: u32, value: &mut u32, bus: &mut Bus, cpu: &mut CPU) -> TickCount {
        assert!((address & 0x1) == 0);
        let offset = address & memory_map::INTERRUPT_CONTROLLER_MASK;
        // Remove "half-word index" in word
        let word_offset = offset & !0x2;
        let word = bus.interrupt_controller.read_register(word_offset);
        // Shift as many half-words as needed;
        *value = word >> ((offset & 0x2) * 16);
        2
    }

    fn do_ram_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = address & memory_map::RAM_2MB_MASK;
        // Little endian
        *value = ((bus.ram[(offset + 1) as usize] as u32) << 8)
            | ((bus.ram[(offset + 0) as usize] as u32) << 0);
        RAM_READ_TICKS
    }

    fn do_memory_control_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = address & memory_map::MEMCTRL_MASK;
        // Each register is 4 bytes
        let index = offset >> 2;
        *value = bus.mem_ctrl_registers.regs[index as usize];
        2
    }

    fn do_bios_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = ((address - memory_map::BIOS_BASE) & memory_map::BIOS_MASK) as usize;
        *value = ((bus.bios[(offset + 1) as usize] as u32) << 8)
            | ((bus.bios[(offset + 0) as usize] as u32) << 0);
        bus.bios_access_time.halfword
    }

    fn is_write() -> bool {
        false
    }
}

impl MemoryAccess for ReadWord {
    fn do_dma_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
    }

    fn do_interrupt_controller_access(address: u32, value: &mut u32, bus: &mut Bus, cpu: &mut CPU) -> TickCount {
        assert!((address & 0x3) == 0);
        let offset = address & memory_map::INTERRUPT_CONTROLLER_MASK;
        *value = bus.interrupt_controller.read_register(offset);
        2
    }

    fn do_ram_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = address & memory_map::RAM_2MB_MASK;
        // Little endian
        *value = ((bus.ram[(offset + 3) as usize] as u32) << 24)
            | ((bus.ram[(offset + 2) as usize] as u32) << 16)
            | ((bus.ram[(offset + 1) as usize] as u32) << 8)
            | ((bus.ram[(offset + 0) as usize] as u32) << 0);
        RAM_READ_TICKS
    }

    fn do_memory_control_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = address & memory_map::MEMCTRL_MASK;
        // Each register is 4 bytes
        let index = offset >> 2;
        *value = bus.mem_ctrl_registers.regs[index as usize];
        2
    }

    fn do_bios_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let offset = ((address - memory_map::BIOS_BASE) & memory_map::BIOS_MASK) as usize;
        *value = ((bus.bios[(offset + 3) as usize] as u32) << 24)
            | ((bus.bios[(offset + 2) as usize] as u32) << 16)
            | ((bus.bios[(offset + 1) as usize] as u32) << 8)
            | ((bus.bios[(offset + 0) as usize] as u32) << 0);
        bus.bios_access_time.word
    }

    fn is_write() -> bool {
        false
    }
}

impl MemoryAccess for WriteByte {
    fn do_interrupt_controller_access(address: u32, value: &mut u32, bus: &mut Bus, cpu: &mut CPU) -> TickCount {
        let offset = address & memory_map::INTERRUPT_CONTROLLER_MASK;
        // Remove "byte index" in word
        let word_offset = offset & !0x3;
        let word = *value << ((offset & 0x3) * 8);
        // We actually write the whole word
        bus.interrupt_controller.write_register(word_offset, word, cpu);
        0
    }

    fn do_ram_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let address = address & memory_map::RAM_2MB_MASK;
        // Little endian
        bus.ram[(address + 0) as usize] = ((*value >> 0) & 0xFF) as u8;
        0
    }

    fn do_memory_control_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let address = address & memory_map::MEMCTRL_MASK;
        let index = address >> 2;
        let write_mask = if index == 8 {
            ComDelayReg::WRITE_MASK
        } else {
            MemDelayReg::WRITE_MASK
        };
        let new_value =
            (bus.mem_ctrl_registers.regs[index as usize] & !write_mask) | (*value & write_mask);
        if bus.mem_ctrl_registers.regs[index as usize] != new_value {
            bus.mem_ctrl_registers.regs[index as usize] = new_value;
            bus.recalculate_memory_timings();
        }
        0
    }

    fn do_bios_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        warn!("Trying to write to BIOS!");
        0
    }

    fn is_write() -> bool {
        true
    }
}

impl MemoryAccess for WriteHalfWord {
    fn do_interrupt_controller_access(address: u32, value: &mut u32, bus: &mut Bus, cpu: &mut CPU) -> TickCount {
        assert!((address & 0x1) == 0);
        let offset = address & memory_map::INTERRUPT_CONTROLLER_MASK;
        // Remove "half-word index" in word
        let word_offset = offset & !0x2;
        let word = *value << ((offset & 0x2) * 16);
        println!("value: {:x}; word: {:x}, offset: {:x} wo: {:x}", value, word, offset, word_offset);
        // We actually write the whole word
        bus.interrupt_controller.write_register(word_offset, word, cpu);
        0
    }

    fn do_ram_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let address = address & memory_map::RAM_2MB_MASK;
        // Little endian
        bus.ram[(address + 0) as usize] = ((*value >> 0) & 0xFF) as u8;
        bus.ram[(address + 1) as usize] = ((*value >> 8) & 0xFF) as u8;
        0
    }

    fn do_memory_control_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let address = address & memory_map::MEMCTRL_MASK;
        let index = address >> 2;
        let write_mask = if index == 8 {
            ComDelayReg::WRITE_MASK
        } else {
            MemDelayReg::WRITE_MASK
        };
        let new_value =
            (bus.mem_ctrl_registers.regs[index as usize] & !write_mask) | (*value & write_mask);
        if bus.mem_ctrl_registers.regs[index as usize] != new_value {
            bus.mem_ctrl_registers.regs[index as usize] = new_value;
            bus.recalculate_memory_timings();
        }
        0
    }

    fn do_bios_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        warn!("Trying to write to BIOS!");
        0
    }

    fn is_write() -> bool {
        true
    }
}

impl MemoryAccess for WriteWord {
    fn do_interrupt_controller_access(address: u32, value: &mut u32, bus: &mut Bus, cpu: &mut CPU) -> TickCount {
        assert!((address & 0x3) == 0);
        let offset = address & memory_map::INTERRUPT_CONTROLLER_MASK;
        bus.interrupt_controller.write_register(offset, *value, cpu);
        0
    }

    fn do_ram_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let address = address & memory_map::RAM_2MB_MASK;
        // Little endian
        bus.ram[(address + 0) as usize] = ((*value >> 0) & 0xFF) as u8;
        bus.ram[(address + 1) as usize] = ((*value >> 8) & 0xFF) as u8;
        bus.ram[(address + 2) as usize] = ((*value >> 16) & 0xFF) as u8;
        bus.ram[(address + 3) as usize] = ((*value >> 24) & 0xFF) as u8;
        0
    }

    fn do_memory_control_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        let address = address & memory_map::MEMCTRL_MASK;
        let index = address >> 2;
        let write_mask = if index == 8 {
            ComDelayReg::WRITE_MASK
        } else {
            MemDelayReg::WRITE_MASK
        };
        let new_value =
            (bus.mem_ctrl_registers.regs[index as usize] & !write_mask) | (*value & write_mask);
        if bus.mem_ctrl_registers.regs[index as usize] != new_value {
            bus.mem_ctrl_registers.regs[index as usize] = new_value;
            bus.recalculate_memory_timings();
        }
        0
    }

    fn do_bios_access(address: u32, value: &mut u32, bus: &mut Bus) -> TickCount {
        warn!("Trying to write to BIOS!");
        0
    }

    fn is_write() -> bool {
        true
    }
}

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
            icache_tags: [0; i_cache::LINES],
            icache_data: [0; i_cache::SIZE],
            dcache: [0; d_cache::SIZE],
            interrupt_controller: InterruptController::new(),
            dma: DMA::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.recalculate_memory_timings();
    }

    pub fn fetch_instruction(&mut self, address: u32) -> u32 {
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
                // Ignore cache access, access memory directly
                self.do_instruction_read(address)
            }
            0x05 => self.do_instruction_read(address),
            _ => panic!("Address out of bounds: {:x}", address),
        }
    }

    fn do_instruction_read(&mut self, address: u32) -> u32 {
        let address = address & PHYSICAL_MEMORY_ADDRESS_MASK;

        // Read instruction from RAM
        if address < memory_map::RAM_MIRROR_END {
            let mut value = u32::default();
            ReadWord::do_ram_access(address, &mut value, self);
            return value;
        }

        // Read instruction from BIOS
        if address >= (memory_map::BIOS_BASE)
            && address < (memory_map::BIOS_BASE + memory_map::BIOS_SIZE)
        {
            let mut value = u32::default();
            ReadWord::do_bios_access(address, &mut value, self);
            return value;
        };

        panic!("Can't read instruction: {}", address);
    }

    pub fn access_memory<T: MemoryAccess>(
        &mut self,
        address: u32,
        value: &mut u32,
        cache_is_isolated: bool,
        cpu: &mut CPU,
    ) -> TickCount {
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
                if cache_is_isolated && T::is_write() {
                    // With the cache isolated, no writes to memory occur.
                    return 0;
                }
                self.do_memory_access::<T>(address, value, cpu)
            }
            0x05 => self.do_memory_access::<T>(address, value, cpu),
            0x01 | 0x02 | 0x03 => panic!("Reading KUSEG above 512M!"),
            0x06 | 0x07 => {
                warn!("Reading KUSEG2 (cache control)");
                0
            }
            _ => panic!("Accessing out of bounds! {}", tag),
        }
    }

    fn do_memory_access<T: MemoryAccess>(&mut self, address: u32, value: &mut u32, cpu: &mut CPU) -> TickCount {
        let address = address & PHYSICAL_MEMORY_ADDRESS_MASK;

        if address < memory_map::RAM_MIRROR_END {
            debug!("Memory Access: RAM_MIRROR_END");
            return T::do_ram_access(address, value, self);
        } else if address >= memory_map::BIOS_BASE
            && address < (memory_map::BIOS_BASE + memory_map::BIOS_SIZE)
        {
            debug!("Memory Access: BIOS");
            return T::do_bios_access(address, value, self);
        } else if address < memory_map::EXP1_BASE {
            panic!("Invalid Address: BIOS < address < EXP1_BASE");
        } else if address < (memory_map::EXP1_BASE + memory_map::EXP1_SIZE) {
            warn!("Memory Access: EXP1");
            0
        } else if address < memory_map::MEMCTRL_BASE {
            panic!("Invalid Address: EXP1 < address < MEMCTRL_BASE");
        } else if address < (memory_map::MEMCTRL_BASE + memory_map::MEMCTRL_SIZE) {
            warn!("Memory Access: MEMCTRL");
            T::do_memory_control_access(address, value, self)
        } else if address < (memory_map::PAD_BASE + memory_map::PAD_SIZE) {
            panic!("Memory Access: PAD");
        } else if address < (memory_map::SIO_BASE + memory_map::SIO_SIZE) {
            panic!("Memory Access: SIO");
        } else if address < (memory_map::MEMCTRL2_BASE + memory_map::MEMCTRL2_SIZE) {
            warn!("Memory Access: MEMCTRL2");
            0
        } else if address
            < (memory_map::INTERRUPT_CONTROLLER_BASE + memory_map::INTERRUPT_CONTROLLER_SIZE)
        {
            debug!("Memory Access: INTERRUPT_CONTROLLER");
            T::do_interrupt_controller_access(address, value, self, cpu)
        } else if address < (memory_map::DMA_BASE + memory_map::DMA_SIZE) {
            warn!("Memory Access: DMA");
            T::do_dma_access(address, value, self)
        } else if address < (memory_map::TIMERS_BASE + memory_map::TIMERS_SIZE) {
            warn!("Memory Access: TIMERS");
            0
        } else if address < memory_map::CDROM_BASE {
            panic!("Invalid Address: TIMERS < address < CDROM");
        } else if address < (memory_map::CDROM_BASE + memory_map::CDROM_SIZE) {
            panic!("Memory Access: CDROM");
        } else if address < (memory_map::GPU_BASE + memory_map::GPU_SIZE) {
            warn!("Memory Access: GPU");
            0
        } else if address < (memory_map::MDEC_BASE + memory_map::MDEC_SIZE) {
            panic!("Memory Access: MDEC");
        } else if address < memory_map::SPU_BASE {
            panic!("Invalid Address: MDEC < address < SPU");
        } else if address < (memory_map::SPU_BASE + memory_map::SPU_SIZE) {
            debug!("SPU access");
            0
        } else if address < memory_map::EXP2_BASE {
            panic!("Invalid Address: SPU < address < EXP2");
        } else if address < (memory_map::EXP2_BASE + memory_map::EXP2_SIZE) {
            warn!("Memory Access: EXP2");
            0
        } else if address < memory_map::EXP3_BASE {
            panic!("Invalid Address: EXP2 < address < EXP3");
        } else if address < (memory_map::EXP3_BASE + memory_map::EXP3_SIZE) {
            panic!("Memory Access: EXP3");
        } else {
            panic!("Other Memory Access");
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

    pub fn dump_ram(&self) -> () {
        println!("RAM state");
        let mut count = 1;
        for i in 0..memory_map::RAM_2MB_SIZE {
            let byte = self.ram[i as usize];
            if byte != 0x00 {
                print!("{:x}:{:02x} ", i, byte);
                if count % 10 == 0 {
                    println!()
                }
                count += 1;
            }
        }
        println!();
        println!("---");
    }

    pub fn get_ram_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.ram);
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub fn dump_mem_ctrl_registers(&self) -> () {
        println!("Memory Control Registers state");
        let r = self.mem_ctrl_registers.regs;
        println!("{:8x} {:8x} {:8x} {:8x}", r[0], r[1], r[2], r[3]);
        println!("{:8x} {:8x} {:8x} {:8x}", r[4], r[5], r[6], r[7]);
        println!("{:8x}", r[8]);

        println!("Access Times");
        println!(
            "{} {} {}",
            self.bios_access_time.byte, self.bios_access_time.halfword, self.bios_access_time.word
        );
        println!(
            "{} {} {}",
            self.cdrom_access_time.byte,
            self.cdrom_access_time.halfword,
            self.cdrom_access_time.word
        );

        println!("---");
    }

    pub fn dump_interrupt_controller_registers(&self) -> () {
        self.interrupt_controller.dump_regs();
    }
}
