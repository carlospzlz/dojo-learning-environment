use crate::system::bios::BIOS_BASE;
use crate::system::bios::BIOS_MASK;
use crate::system::bios::BIOS_SIZE;
use crate::system::bus::Bus;
use crate::system::cpu_types::Instruction;
use crate::system::cpu_types::InstructionFunct;
use crate::system::cpu_types::InstructionOp;
use crate::system::cpu_types::Registers;

type PhysicalMemoryAddress = u32; // R3000A is 32 bits CPU

const PHYSICAL_MEMORY_ADDRESS_MASK: PhysicalMemoryAddress = 0x1FFFFFFF;
const RAM_MASK: PhysicalMemoryAddress = (RAM_SIZE as u32) - 1; // Mask of relevant bits
const RAM_MIRROR_END: u32 = 0x800000; // 8 * 2^20 - 8MB
const RAM_SIZE: usize = 0x200000; // Size of the RAM (2 MB)

pub struct CPU {
    state: State,
}

struct State {
    registers: Registers,
    instruction: Instruction,
}

impl CPU {
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    pub fn execute(&mut self, bus: &mut Bus) -> Result<(), String> {
        println!(
            "-- CPU::Execute (PC={:x}, NPC={:x}) --",
            self.state.registers.pc, self.state.registers.npc
        );
        self.fetch_instruction(bus);
        self.execute_instruction(bus);
        Ok(())
    }

    fn fetch_instruction(&mut self, bus: &Bus) -> Result<(), String> {
        let address = self.state.registers.pc;
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
                self.state.instruction.bits = self.do_instruction_read(address, &bus).unwrap();
            }
            0x05 => {
                self.state.instruction.bits = self.do_instruction_read(address, &bus).unwrap();
            }
            _ => panic!("Address out of bounds: {:x}", address),
        };
        self.state.registers.pc = self.state.registers.npc;
        self.state.registers.npc +=
            std::mem::size_of::<PhysicalMemoryAddress>() as PhysicalMemoryAddress;
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
            println!("Address: {:x} (RAM)", address);
            let address = address & RAM_MASK;
            debug_assert!(false);
            return Ok(bus.ram[address as usize] as u32);
        }

        // Mapped BIOS
        if address >= BIOS_BASE && address < (BIOS_BASE + BIOS_SIZE as u32) {
            println!("Address: {:x} (BIOS)", address);
            let address = ((address - BIOS_BASE) & BIOS_MASK) as usize;
            // R3000A is little endian! So the most significant bytes are
            // stored in lower memory addresses.
            // Funny enough, if you brutely read a u32 in C++ on the host
            // (which usually is little endian), bytes will be arranged like
            // [3, 2, 1, 0] and the instruction will be formed correctly.
            let instruction = bus
                .bios
                .get(address..address + 4)
                .map(|slice| u32::from_be_bytes([slice[3], slice[2], slice[1], slice[0]]))
                .unwrap();
            return Ok(instruction);
        }

        Err(format!("Can't read instruction: {}", address))
    }

    fn execute_instruction(&mut self, bus: &mut Bus) -> Result<(), String> {
        let instruction = &self.state.instruction;
        println!(
            "{:x}  {} ({:?})",
            instruction.bits,
            instruction.to_string(),
            instruction.get_op_code()
        );
        match instruction.get_op_code() {
            InstructionOp::FUNCT => {
                println!("{:?}", instruction.get_funct());
                match instruction.get_funct() {
                    InstructionFunct::SLL => self.execute_sll(),
                    _ => todo!(),
                }
            }
            InstructionOp::ADDI => self.execute_addi(),
            InstructionOp::ADDIU => self.execute_addiu(),
            InstructionOp::BEQ => self.execute_beq(),
            InstructionOp::LUI => self.execute_lui(),
            InstructionOp::ORI => self.execute_ori(),
            InstructionOp::SW => self.execute_sw(bus),
            _ => todo!(),
        };
        Ok(())
    }

    fn execute_addi(&mut self) -> () {
        // ADDI rt, rs, immediate
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        println!("[rt={}, rs={}, immediate={}]", rt, rs, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let result = (rs_value as i32) + (immediate as i32);
        self.state.registers.write_register(rt, result as u32);
    }

    fn execute_addiu(&mut self) -> () {
        // ADDIU rt, rs, immediate
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        println!("[rt={}, rs={}, immediate={}]", rt, rs, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let result = rs_value + (immediate as u32);
        self.state.registers.write_register(rt, result);
    }

    fn execute_beq(&mut self) -> () {
        // BEQ rs, rt, offset
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        let offset = self.state.instruction.get_offset();
        println!("[rs={}, rt={}, offset={}]", rs, rt, offset);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        if rs_value == rt_value {
            self.state.registers.npc = self.state.registers.pc + (offset as u32);
            println!("Branch!");
        }
    }

    fn execute_lui(&mut self) -> () {
        // LUI rs, immediate
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        println!("[rs={}, immediate={}]", rs, immediate);
        self.state.registers.write_register_upper(rs, immediate);
    }

    fn execute_ori(&mut self) -> () {
        // ORI rt, rs, immediate
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        println!("[rt={}, rs={}, immediate={}]", rt, rs, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let result = rs_value | (immediate as u32);
        self.state.registers.write_register(rt, result);
    }

    fn execute_sll(&mut self) -> () {
        // SLL rd, rt, shamt
        let rd = self.state.instruction.get_rd();
        let rt = self.state.instruction.get_rt();
        let shamt = self.state.instruction.get_shamt();
        println!("[rd={}, rt={}, shamt={}]", rd, rt, shamt);
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let result = rt_value << shamt;
        self.state.registers.write_register(rd, result);
    }

    fn execute_sw(&mut self, bus: &mut Bus) -> () {
        // SW rt, base(offset)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        println!("[rt={}, offset={}", rt, offset);
        let address = ((base as i32) + (offset as i32)) as usize;
        let rs_value = self.state.registers.read_register(rt).unwrap();
        // Little endian
        bus.ram[address + 0] = ((rs_value >> 24) & 0xFF) as u8;
        bus.ram[address + 1] = ((rs_value >> 16) & 0xFF) as u8;
        bus.ram[address + 2] = ((rs_value >> 8) & 0xFF) as u8;
        bus.ram[address + 3] = ((rs_value >> 0) & 0xFF) as u8;
    }
}

impl State {
    fn new() -> Self {
        Self {
            registers: Registers::new(),
            instruction: Instruction::new(),
        }
    }
}
