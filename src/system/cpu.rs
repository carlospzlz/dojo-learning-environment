use log::{debug, error, info, warn};
use std::io::{self, BufRead};

use crate::system::bios::BIOS_BASE;
use crate::system::bios::BIOS_MASK;
use crate::system::bios::BIOS_SIZE;
use crate::system::bus::Bus;
use crate::system::cpu_types::Cop0Instruction;
use crate::system::cpu_types::Cop0Reg;
use crate::system::cpu_types::Cop0Registers;
use crate::system::cpu_types::CopCommonInstruction;
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
    instruction: Instruction,
    registers: Registers,
    cop0_registers: Cop0Registers,
    frame_done: bool,

    // Caution - Load Delay
    //
    // The loaded data is NOT available to the next opcode, ie. the target
    // register isn't updated until the next opcode has completed. So, if the
    // next opcode tries to read from the load destination register, then it
    // would (usually) receive the OLD value of that register (unless an IRQ
    // occurs between the load and next opcode, in that case the load would
    // complete during IRQ handling, and so, the next opcode would receive the
    // NEW value).
    //
    //MFC2/CFC2 also have a 1-instruction delay until the target register is
    //loaded with its new value (more info in the GTE section).
    //load_delay_register: u8,
    //load_dalay_value: u32,
    //next_load_delay_register: u8,
    //next_load_dalay_value: u32,
}

impl CPU {
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    pub fn execute(&mut self, bus: &mut Bus) -> Result<(), String> {
        self.fetch_instruction(bus);
        self.execute_instruction(bus);
        self.state.dump();
        println!();

        //let stdin = io::stdin();
        //let mut buffer = String::new();
        //stdin.lock().read_line(&mut buffer);

        Ok(())
    }

    fn fetch_instruction(&mut self, bus: &Bus) -> Result<(), String> {
        let address = self.state.registers.pc;
        let bits = bus.fetch_instruction(address).unwrap();
        self.state.instruction.bits = bits;
        self.state.registers.pc = self.state.registers.npc;
        self.state.registers.npc += std::mem::size_of::<u32>() as u32;
        Ok(())
    }

    fn execute_instruction(&mut self, bus: &mut Bus) -> Result<(), String> {
        let instruction = &self.state.instruction;
        debug!(
            "{:x}  {} ({:?})",
            instruction.bits,
            instruction.to_string(),
            instruction.get_op_code()
        );
        match instruction.get_op_code() {
            InstructionOp::FUNCT => {
                debug!("FUNCT: {:?}", instruction.get_funct());
                match instruction.get_funct() {
                    InstructionFunct::ADDU => self.execute_addu(),
                    InstructionFunct::JR => self.execute_jr(),
                    InstructionFunct::MFHI => self.execute_mfhi(),
                    InstructionFunct::OR => self.execute_or(),
                    InstructionFunct::SLL => self.execute_sll(),
                    InstructionFunct::SLTU => self.execute_sltu(),
                    _ => todo!(),
                }
            }
            InstructionOp::ADDI => self.execute_addi(),
            InstructionOp::ADDIU => self.execute_addiu(),
            InstructionOp::ANDI => self.execute_andi(),
            InstructionOp::BEQ => self.execute_beq(),
            InstructionOp::BNE => self.execute_bne(),
            InstructionOp::COP0 => self.execute_cop0(),
            InstructionOp::LUI => self.execute_lui(),
            InstructionOp::LW => self.execute_lw(bus),
            InstructionOp::J => self.execute_j(),
            InstructionOp::JAL => self.execute_jal(),
            InstructionOp::ORI => self.execute_ori(),
            InstructionOp::SB => self.execute_sb(bus),
            InstructionOp::SW => self.execute_sw(bus),
            InstructionOp::SH => self.execute_sh(bus),

            // No-Ops
            InstructionOp::COP1 => {}
            InstructionOp::COP3 => {}
            InstructionOp::LWC0 => {}
            InstructionOp::LWC1 => {}
            InstructionOp::LWC3 => {}
            InstructionOp::SWC0 => {}
            InstructionOp::SWC1 => {}
            InstructionOp::SWC3 => {}

            _ => todo!(),
        };
        Ok(())
    }

    fn execute_addi(&mut self) -> () {
        // ADDI rt, rs, immediate
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        debug!("[rt={}, rs={}, immediate={}]", rt, rs, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let sext_immediate = immediate as i16 as i32;
        let result = ((rs_value as i32) + sext_immediate) as u32;
        self.state.registers.write_register(rt, result as u32);
    }

    fn execute_addiu(&mut self) -> () {
        // ADDIU rt, rs, immediate
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        debug!("[rt={}, rs={}, immediate={}]", rt, rs, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let sext_immediate = immediate as i16 as i32;
        //  While ADDI triggers an exception on overflow, ADDIU performs an
        //  unsigned wraparound on overflow
        let result = (rs_value as i32).wrapping_add(sext_immediate) as u32;
        self.state.registers.write_register(rt, result);
    }

    fn execute_andi(&mut self) -> () {
        // ANDI rt, rs, immediate
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        debug!("[rt={}, rs={}, immediate={}]", rt, rs, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let result = rs_value & (immediate as u32);
        self.state.registers.write_register(rt, result);
    }

    fn execute_addu(&mut self) -> () {
        // ADDU rd, rs, rt, immediate
        let rd = self.state.instruction.get_rd();
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        debug!("[rd={}, rs={}, rt={}]", rd, rt, rs);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let result = rs_value + rt_value;
        self.state.registers.write_register(rd, result);
    }

    fn execute_beq(&mut self) -> () {
        // BEQ rs, rt, offset
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        let offset = self.state.instruction.get_offset();
        debug!("[rs={}, rt={}, offset={}]", rs, rt, offset);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        if rs_value == rt_value {
            self.state.registers.npc = self.state.registers.pc + (offset as u32);
            println!("Branch!");
        }
    }

    fn execute_bne(&mut self) -> () {
        // BNE rs, rt, offset
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        let offset = self.state.instruction.get_offset();
        debug!("[rs={}, rt={}, offset={}]", rs, rt, offset);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        if rs_value != rt_value {
            // Sign extend to i32
            let offset = (offset as i16 as i32) << 2;
            // Add, PC interpreted as i32
            let npc = self.state.registers.pc as i32 + offset;
            self.state.registers.npc = npc as u32;
            println!("Branch!");
        }
    }

    fn execute_j(&mut self) -> () {
        // J target
        let target = self.state.instruction.get_target();
        let address = (self.state.registers.pc & 0xF0000000) | (target << 2);
        // Address must be multiple of 4
        assert!((address & 0x3) == 0);
        debug!("[target={}]", target);
        self.state.registers.npc = address;
    }

    fn execute_jal(&mut self) -> () {
        // JAL target
        // Link
        self.state
            .registers
            .write_register(31, self.state.registers.npc);
        let target = self.state.instruction.get_target();
        let address = (self.state.registers.pc & 0xF0000000) | (target << 2);
        // Address must be multiple of 4
        assert!((address & 0x3) == 0);
        debug!("[target={}]", target);
        self.state.registers.npc = address;
    }

    fn execute_jr(&mut self) -> () {
        // JR rs
        let rs = self.state.instruction.get_rs();
        let rs_value = self.state.registers.read_register(rs).unwrap();
        debug!("[rs={}", rs);
        self.state.registers.npc = rs_value;
    }

    fn execute_lui(&mut self) -> () {
        // LUI rt, immediate
        let rt = self.state.instruction.get_rt();
        let immediate = self.state.instruction.get_immediate();
        debug!("[rt={}, immediate={}]", rt, immediate);
        self.state.registers.write_register_upper(rt, immediate);
    }

    fn execute_lw(&mut self, bus: &mut Bus) -> () {
        // LW rt, offset(base)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let address = ((base as i32) + (offset as i32)) as u32;
        // Little endian
        //let word = ((bus.ram[address + 3] as u32) << 24)
        //    | ((bus.ram[address + 2] as u32) << 16)
        //    | ((bus.ram[address + 1] as u32) << 8)
        //    | ((bus.ram[address + 0] as u32) << 0);
        let word = bus.read_memory_word(address);
        //self.state.registers.write_register(rt, word);
        // This is not working
    }

    fn execute_mfhi(&mut self) -> () {
        // MFHI rd
        let rd = self.state.instruction.get_rd();
        debug!("[rd={}]", rd);
        let hi_value = self.state.registers.read_register_hi();
        self.state.registers.write_register(rd, hi_value);
    }

    fn execute_or(&mut self) -> () {
        // OR rd, rt, rs
        let rd = self.state.instruction.get_rd();
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        debug!("[rd={}, rt={}, rs={}]", rd, rt, rs);
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let result = rt_value | rs_value;
        self.state.registers.write_register(rd, result);
    }

    fn execute_ori(&mut self) -> () {
        // ORI rt, rs, immediate
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        let immediate = self.state.instruction.get_immediate();
        debug!("[rt={}, rs={}, immediate={}]", rt, rs, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let result = rs_value | (immediate as u32);
        self.state.registers.write_register(rt, result);
    }

    fn execute_sll(&mut self) -> () {
        // SLL rd, rt, shamt
        let rd = self.state.instruction.get_rd();
        let rt = self.state.instruction.get_rt();
        let shamt = self.state.instruction.get_shamt();
        debug!("[rd={}, rt={}, shamt={}]", rd, rt, shamt);
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let result = rt_value << shamt;
        self.state.registers.write_register(rd, result);
    }

    fn execute_sltu(&mut self) -> () {
        // SLTU rd, rs, rt
        let rd = self.state.instruction.get_rd();
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        debug!("[rd={}, rs={}, rt={}]", rd, rs, rt);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        if rs_value < rt_value {
            self.state.registers.write_register(rd, 1);
        } else {
            self.state.registers.write_register(rd, 0);
        }
    }

    fn execute_sb(&mut self, bus: &mut Bus) -> () {
        // SB rt, base(offset)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let sext_offset = offset as i16 as i32;
        let address = ((base as i32) + sext_offset) as u32;
        let ts_value = self.state.registers.read_register(rt).unwrap();
        let half_word = (ts_value & 0x000000FF) as u16;
        bus.write_memory_half_word(address, half_word);
    }

    fn execute_sh(&mut self, bus: &mut Bus) -> () {
        // SW rt, base(offset)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let sext_offset = offset as i16 as i32;
        let address = ((base as i32) + sext_offset) as u32;
        let ts_value = self.state.registers.read_register(rt).unwrap();
        let half_word = (ts_value & 0x0000FFFF) as u16;
        bus.write_memory_half_word(address, half_word);
    }

    fn execute_sw(&mut self, bus: &mut Bus) -> () {
        // SW rt, base(offset)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let sext_offset = offset as i16 as i32;
        let address = ((base as i32) + sext_offset) as u32;
        let rt_value = self.state.registers.read_register(rt).unwrap();
        // Little endian
        //bus.ram[address + 0] = ((rt_value >> 24) & 0xFF) as u8;
        //bus.ram[address + 1] = ((rt_value >> 16) & 0xFF) as u8;
        //bus.ram[address + 2] = ((rt_value >> 8) & 0xFF) as u8;
        //bus.ram[address + 3] = ((rt_value >> 0) & 0xFF) as u8;
        warn!("SW wrongly implemented");
        //bus.write_memory_word(address);
    }

    // Coprocessor Instructions

    fn execute_cop0(&mut self) -> () {
        if self.state.instruction.is_cop_common_instruction() {
            self.execute_cop_common_instruction()
        } else {
            match self.state.instruction.get_cop0_op() {
                _ => todo!(),
            }
        }
    }

    fn execute_cop_common_instruction(&mut self) -> () {
        // MTCN rt, rd
        let cop_number = self.state.instruction.get_cop_number();
        let cop_op = self.state.instruction.get_cop_common_op();
        debug!("COP{}: Common Op: {:?}", cop_number, cop_op);
        let rt = self.state.instruction.get_rt();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let cop0_reg = Cop0Reg::from(self.state.instruction.get_rd());
        self.state
            .cop0_registers
            .write_register(cop0_reg, rt_value)
            .unwrap();
    }
}

impl State {
    fn new() -> Self {
        Self {
            instruction: Instruction::new(),
            registers: Registers::new(),
            cop0_registers: Cop0Registers::new(),
            frame_done: false,
        }
    }

    fn dump(&self) -> () {
        println!("PC={:x} NPC={:x}", self.registers.pc, self.registers.npc);
        println!("Inst={:x}", self.instruction.bits);
        self.registers.dump();
    }
}
