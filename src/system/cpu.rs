use crate::system::bios::BIOS_BASE;
use crate::system::bios::BIOS_MASK;
use crate::system::bios::BIOS_SIZE;
use crate::system::bus::Bus;

type PhysicalMemoryAddress = u32; // R3000A is 32 bits CPU

const PHYSICAL_MEMORY_ADDRESS_MASK: PhysicalMemoryAddress = 0x1FFFFFFF;
const RAM_MASK: PhysicalMemoryAddress = (RAM_SIZE as u32) - 1; // Mask of relevant bits
const RAM_MIRROR_END: u32 = 0x80000000; // 2^31 - ~2GB
const RAM_SIZE: usize = 0x200000; // Size of the RAM (2 MB)
const RESET_VECTOR: PhysicalMemoryAddress = 0xBFC00000;

pub struct CPU {
    state: State,
}

struct State {
    registers: Registers,
    next_instruction: Instruction,
}

struct Instruction {
    bits: u32,
}

struct Registers {
    npc: PhysicalMemoryAddress,
}

impl CPU {
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    pub fn execute(&mut self, bus: &mut Bus) -> Result<(), String> {
        println!("CPU::Execute");
        self.fetch_instruction(&bus);
        self.state.registers.npc +=
            std::mem::size_of::<PhysicalMemoryAddress>() as PhysicalMemoryAddress;
        Ok(())
    }

    fn fetch_instruction(&mut self, bus: &Bus) -> Result<(), String> {
        let address = self.state.registers.npc;
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
                self.state.next_instruction.bits = self.do_instruction_read(address, &bus).unwrap();
            }
            0x05 => {
                self.state.next_instruction.bits = self.do_instruction_read(address, &bus).unwrap();
            }
            _ => panic!("Address out of bounds: {:x}", address),
        };
        println!("Address tag: {:x}", address);
        Ok(())
    }

    fn do_instruction_read(
        &mut self,
        address: PhysicalMemoryAddress,
        bus: &Bus,
    ) -> Result<u32, String> {
        let address = address & PHYSICAL_MEMORY_ADDRESS_MASK;

        // RAM
        if address < RAM_MIRROR_END {
            let address = address & RAM_MASK;
            return Ok(bus.ram[address as usize] as u32);
        }

        // Mapped BIOS
        if address >= BIOS_BASE && address < (BIOS_BASE + BIOS_SIZE as u32) {
            let address = (address - BIOS_BASE) & BIOS_MASK;
            return Ok(bus.bios[address as usize] as u32);
        }

        Err(format!("Can't read instruction: {}", address))
    }
}

impl State {
    fn new() -> Self {
        Self {
            registers: Registers::new(),
            next_instruction: Instruction::new(),
        }
    }
}

impl Registers {
    fn new() -> Self {
        Self { npc: RESET_VECTOR }
    }
}

impl Instruction {
    fn new() -> Self {
        Self { bits: 0x0 }
    }
}
