use log::{debug, error, info, warn};
use std::io::{self, BufRead};

use crate::system::bios::BIOS_BASE;
use crate::system::bios::BIOS_MASK;
use crate::system::bios::BIOS_SIZE;
use crate::system::bus::Bus;
use crate::system::bus::ReadByte;
use crate::system::bus::ReadWord;
use crate::system::bus::WriteByte;
use crate::system::bus::WriteHalfWord;
use crate::system::bus::WriteWord;
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
    // Probably CPU and State can be merged
    state: State,
}

struct State {
    cycle: usize,
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

        // 79400 - 79500
        let start = 86000;
        let amount = 10000;
        if self.state.cycle > start && self.state.cycle < (start + amount) {
            self.state.dump();
            println!("RAM SHA-256: {}", bus.get_ram_hash());
            //bus.dump_ram();
            //bus.dump_mem_ctrl_registers();
            println!();
        }

        //let stdin = io::stdin();
        //let mut buffer = String::new();
        //stdin.lock().read_line(&mut buffer);

        self.state.cycle += 1;

        Ok(())
    }

    fn fetch_instruction(&mut self, bus: &mut Bus) -> Result<(), String> {
        let address = self.state.registers.pc;
        let bits = bus.fetch_instruction(address);
        self.state.instruction.bits = bits;
        self.state.registers.pc = self.state.registers.npc;
        self.state.registers.npc += std::mem::size_of::<u32>() as u32;
        Ok(())
    }

    fn execute_instruction(&mut self, bus: &mut Bus) -> Result<(), String> {
        let instruction = &self.state.instruction;
        info!(
            "[Cycle={}] {:x}  {} ({:?})",
            self.state.cycle,
            instruction.bits,
            instruction.to_string(),
            instruction.get_op_code()
        );
        match instruction.get_op_code() {
            InstructionOp::FUNCT => {
                debug!("FUNCT: {:?}", instruction.get_funct());
                match instruction.get_funct() {
                    InstructionFunct::ADD => self.execute_add(),
                    InstructionFunct::ADDU => self.execute_addu(),
                    InstructionFunct::AND => self.execute_and(),
                    InstructionFunct::JALR => self.execute_jalr(),
                    InstructionFunct::JR => self.execute_jr(),
                    InstructionFunct::MFHI => self.execute_mfhi(),
                    InstructionFunct::OR => self.execute_or(),
                    InstructionFunct::SLL => self.execute_sll(),
                    InstructionFunct::SLTU => self.execute_sltu(),
                    InstructionFunct::SRA => self.execute_sra(),
                    InstructionFunct::SUBU => self.execute_subu(),
                    _ => todo!("FUNCT no implemented: {:?}", instruction.get_funct()),
                }
            }
            InstructionOp::ADDI => self.execute_addi(),
            InstructionOp::ADDIU => self.execute_addiu(),
            InstructionOp::ANDI => self.execute_andi(),
            InstructionOp::B => self.execute_b(),
            InstructionOp::BEQ => self.execute_beq(),
            InstructionOp::BGTZ => self.execute_bgtz(),
            InstructionOp::BLEZ => self.execute_blez(),
            InstructionOp::BNE => self.execute_bne(),
            InstructionOp::COP0 => self.execute_cop0(),
            InstructionOp::LB => self.execute_lb(bus),
            InstructionOp::LBU => self.execute_lbu(bus),
            InstructionOp::LUI => self.execute_lui(),
            InstructionOp::LW => self.execute_lw(bus),
            InstructionOp::J => self.execute_j(),
            InstructionOp::JAL => self.execute_jal(),
            InstructionOp::ORI => self.execute_ori(),
            InstructionOp::SB => self.execute_sb(bus),
            InstructionOp::SLTI => self.execute_slti(),
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

            _ => todo!("OP not implemented: {:?}", instruction.get_op_code()),
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

    fn execute_add(&mut self) -> () {
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

    fn execute_and(&mut self) -> () {
        // AND rd, rt, rs
        let rd = self.state.instruction.get_rd();
        let rt = self.state.instruction.get_rt();
        let rs = self.state.instruction.get_rs();
        debug!("AND rd={} rt={}, rs={}", rd, rt, rs);
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let result = rs_value & rt_value;
        self.state.registers.write_register(rd, result);
    }

    fn execute_b(&mut self) -> () {
        // B rs, offset
        //
        // This instruction seems to be under BcondZ:
        //
        // BGEZ    0000 01ss sss0 0001 ffff ffff ffff ffff
        // BGEZAL  0000 01ss sss1 0001 ffff ffff ffff ffff
        // BGEZALL 0000 01ss sss1 0011 ffff ffff ffff ffff
        // BGEZL   0000 01ss sss0 0011 ffff ffff ffff ffff
        //
        // BLTZ    0000 01ss sss0 0000 ffff ffff ffff ffff
        // BLTZAL  0000 01ss sss1 0000 ffff ffff ffff ffff
        // BLTZALL 0000 01ss sss1 0010 ffff ffff ffff ffff
        // BLTZL   0000 01ss sss0 0010 ffff ffff ffff ffff
        //
        // I needed some help from Duckstation for this instruction
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        let offset = self.state.instruction.get_offset();
        debug!("B rs={}, rt={}, offset={}]", rs, rt, offset);

        // Link (it's done even if branch isn't taken)
        // Not sure what's going on here, it seems that ALL and L don't link
        if (rt & 0x1E) == 0x10 {
            self.state.registers.write_register(31, self.state.registers.npc);
        }

        // Jump
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let is_bgez = (rt & 1) == 1;
        // XOR, these two must conditions be different
        // (from duckstation, pretty smart)
        if ((rs_value as i32) < 0) ^ is_bgez {
            // Sign extend to i32 and multiply by 4 (4 bytes)
            let offset = (offset as i16 as i32) << 2;
            // Add, PC interpreted as i32
            let npc = self.state.registers.pc as i32 + offset;
            self.state.registers.npc = npc as u32;
        }
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
            // Sign extend to i32
            let offset = (offset as i16 as i32) << 2;
            // Add, PC interpreted as i32
            let npc = self.state.registers.pc as i32 + offset;
            self.state.registers.npc = npc as u32;
        }
    }

    fn execute_bgtz(&mut self) -> () {
        // BGTZ rs, offset
        let rs = self.state.instruction.get_rs();
        let offset = self.state.instruction.get_offset();
        debug!("BGTZ rs={}, offset={}]", rs, offset);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        if rs_value > 0 {
            // Sign extend to i32
            let offset = (offset as i16 as i32) << 2;
            // Add, PC interpreted as i32
            let npc = self.state.registers.pc as i32 + offset;
            self.state.registers.npc = npc as u32;
        }
    }

    fn execute_blez(&mut self) -> () {
        // BLEZ rs, offset
        let rs = self.state.instruction.get_rs();
        let offset = self.state.instruction.get_offset();
        debug!("BGTZ rs={}, offset={}]", rs, offset);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        if rs_value <= 0 {
            // Sign extend to i32
            let offset = (offset as i16 as i32) << 2;
            // Add, PC interpreted as i32
            let npc = self.state.registers.pc as i32 + offset;
            self.state.registers.npc = npc as u32;
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

    fn execute_jalr(&mut self) -> () {
        // JALR rs
        let rd = self.state.instruction.get_rd();
        let rs = self.state.instruction.get_rs();
        debug!("JALR rd={} rs={}", rd, rs);
        // Link Register
        self.state
            .registers
            .write_register(rd, self.state.registers.npc);
        // Jump
        let rs_value = self.state.registers.read_register(rs).unwrap();
        self.state.registers.npc = rs_value;
    }

    fn execute_jr(&mut self) -> () {
        // JR rs
        let rs = self.state.instruction.get_rs();
        let rs_value = self.state.registers.read_register(rs).unwrap();
        debug!("[rs={}]", rs);
        self.state.registers.npc = rs_value;
    }

    fn execute_lb(&mut self, bus: &mut Bus) -> () {
        // LB rt, offset(base)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let base_value = self.state.registers.read_register(base).unwrap();
        let sext_offset = offset as i16 as i32;
        let address = ((base_value as i32) + sext_offset) as u32;
        let mut value: u32 = u32::default();
        let cache_is_isolated = self.state.cop0_registers.sr.get_isc();
        bus.access_memory::<ReadByte>(address, &mut value, cache_is_isolated);
        let sext_value = ((value & 0xFF) as i8 as i32) as u32;
        self.state.registers.write_register(rt, sext_value);
    }

    fn execute_lbu(&mut self, bus: &mut Bus) -> () {
        // LBU rt, offset(base)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let base_value = self.state.registers.read_register(base).unwrap();
        let sext_offset = offset as i16 as i32;
        let address = ((base_value as i32) + sext_offset) as u32;
        let mut value = u32::default();
        let cache_is_isolated = self.state.cop0_registers.sr.get_isc();
        bus.access_memory::<ReadByte>(address, &mut value, cache_is_isolated);
        self.state.registers.write_register(rt, value);
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
        let base_value = self.state.registers.read_register(base).unwrap();
        let sext_offset = offset as i16 as i32;
        let address = ((base_value as i32) + sext_offset) as u32;
        let mut word: u32 = 0;
        let cache_is_isolated = self.state.cop0_registers.sr.get_isc();
        bus.access_memory::<ReadWord>(address, &mut word, cache_is_isolated);
        self.state.registers.write_register(rt, word);
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

    fn execute_slti(&mut self) -> () {
        // SLTI rs, rt, immediate
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        let immediate = self.state.instruction.get_immediate();
        debug!("SLTI rs={}, rt={} immediate={}", rs, rt, immediate);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let sext_immediate = immediate as i16 as i32;
        let result = ((rs_value as i32) < sext_immediate) as u32;
        self.state.registers.write_register(rt, result);
    }

    fn execute_sltu(&mut self) -> () {
        // SLTU rd, rs, rt
        let rd = self.state.instruction.get_rd();
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        debug!("[rd={}, rs={}, rt={}]", rd, rs, rt);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let result = (rs_value < rt_value) as u32;
        self.state.registers.write_register(rd, result);
    }

    fn execute_sra(&mut self) -> () {
        // SRA rd, rt, shamt
        let rd = self.state.instruction.get_rd();
        let rt = self.state.instruction.get_rt();
        let shamt = self.state.instruction.get_shamt();
        debug!("SRA rd={}, rt={}, shamt={}", rd, rt, shamt);
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let result = rt_value >> shamt;
        self.state.registers.write_register(rd, result);
    }

    fn execute_sb(&mut self, bus: &mut Bus) -> () {
        // SB rt, base(offset)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let base_value = self.state.registers.read_register(base).unwrap();
        let sext_offset = offset as i16 as i32;
        let address = ((base_value as i32) + sext_offset) as u32;
        let ts_value = self.state.registers.read_register(rt).unwrap();
        let mut value = ts_value & 0x000000FF;
        let cache_is_isolated = self.state.cop0_registers.sr.get_isc();
        bus.access_memory::<WriteByte>(address, &mut value, cache_is_isolated);
    }

    fn execute_sh(&mut self, bus: &mut Bus) -> () {
        // SH rt, base(offset)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        debug!("[rt={}, offset={}]", rt, offset);
        let base_value = self.state.registers.read_register(base).unwrap();
        let sext_offset = offset as i16 as i32;
        let address = ((base_value as i32) + sext_offset) as u32;
        let ts_value = self.state.registers.read_register(rt).unwrap();
        let mut value = ts_value & 0x0000FFFF;
        let cache_is_isolated = self.state.cop0_registers.sr.get_isc();
        bus.access_memory::<WriteHalfWord>(address, &mut value, cache_is_isolated);
    }

    fn execute_subu(&mut self) -> () {
        // SLTU rd, rs, rt
        let rd = self.state.instruction.get_rd();
        let rs = self.state.instruction.get_rs();
        let rt = self.state.instruction.get_rt();
        debug!("SUBU rd={}, rs={}, rt={}", rd, rs, rt);
        let rs_value = self.state.registers.read_register(rs).unwrap();
        let rt_value = self.state.registers.read_register(rt).unwrap();
        let result = rs_value - rt_value;
        self.state.registers.write_register(rd, result);
    }

    fn execute_sw(&mut self, bus: &mut Bus) -> () {
        // SW rt, base(offset)
        let rt = self.state.instruction.get_rt();
        let base = self.state.instruction.get_base();
        let offset = self.state.instruction.get_offset();
        info!("[rt={}, offset={}]", rt, offset);
        let base_value = self.state.registers.read_register(base).unwrap();
        let sext_offset = offset as i16 as i32;
        let address = ((base_value as i32) + sext_offset) as u32;
        let mut rt_value = self.state.registers.read_register(rt).unwrap();
        let cache_is_isolated = self.state.cop0_registers.sr.get_isc();
        info!(
            "Base: {:x} Offset: {:x} Address: {:x}",
            base as u32, offset, address
        );
        bus.access_memory::<WriteWord>(address, &mut rt_value, cache_is_isolated);
    }

    // Coprocessor Instructions

    fn execute_cop0(&mut self) -> () {
        if self.state.instruction.is_cop_common_instruction() {
            match self.state.instruction.get_cop_common_op() {
                CopCommonInstruction::MFCN => self.execute_cop_common_instruction_mfcn(),
                CopCommonInstruction::MTCN => self.execute_cop_common_instruction_mtcn(),
                _ => panic!("Unhandled common COP0 instruction"),
            }
        } else {
            match self.state.instruction.get_cop0_op() {
                Cop0Instruction::RFE => self.execute_cop0_instruction_rfe(),
                // No-Op
                Cop0Instruction::TLBR => {}
                Cop0Instruction::TLBWI => {}
                Cop0Instruction::TLBWR => {}
                Cop0Instruction::TLBP => {}
                _ => todo!(),
            }
        }
    }

    fn execute_cop_common_instruction_mfcn(&mut self) -> () {
        // MFCN rt, rd
        let cop_number = self.state.instruction.get_cop_number();
        let rt = self.state.instruction.get_rt();
        let cop0_reg = Cop0Reg::from(self.state.instruction.get_rd());
        debug!("COP{} rt={}, rd={:?}]", cop_number, rt, cop0_reg);
        let cop0_value = self.state.cop0_registers.read_register(cop0_reg);
        self.state.registers.write_register(rt, cop0_value);
    }

    fn execute_cop_common_instruction_mtcn(&mut self) -> () {
        // MTCN rt, rd
        let cop_number = self.state.instruction.get_cop_number();
        let rt = self.state.instruction.get_rt();
        let cop0_reg = Cop0Reg::from(self.state.instruction.get_rd());
        debug!("COP{} rt={}, rd={:?}]", cop_number, rt, cop0_reg);
        let rt_value = self.state.registers.read_register(rt).unwrap();
        self.state.cop0_registers.write_register(cop0_reg, rt_value)
    }

    fn execute_cop0_instruction_rfe(&mut self) -> () {
        // Return From Exception
        let mode_bits = self.state.cop0_registers.sr.get_mode_bits();
        // Restore mode (From Duckstation)
        let mode_bits = (mode_bits & 0b110000) | (mode_bits >> 2);
        self.state.cop0_registers.sr.set_mode_bits(mode_bits);
    }
}

impl State {
    fn new() -> Self {
        Self {
            cycle: 0,
            instruction: Instruction::new(),
            registers: Registers::new(),
            cop0_registers: Cop0Registers::new(),
            frame_done: false,
            //load_delay_register: 0,
            //load_dalay_value: 0,
            //next_load_delay_register: 0,
            //next_load_dalay_value: 0,
        }
    }

    fn dump(&self) -> () {
        println!("* Cycle {}", self.cycle);
        println!("PC={:x} NPC={:x}", self.registers.pc, self.registers.npc);
        println!("Inst={:x}", self.instruction.bits);
        self.registers.dump();
        println!("---");
    }
}
