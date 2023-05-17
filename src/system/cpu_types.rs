use std::result::Result;

const RESET_VECTOR: u32 = 0xBFC00000;

// R3000A is based on the MIPS III instruction set architecture
#[derive(Debug)]
pub enum InstructionOp {
    FUNCT = 0,
    B = 1,
    J = 2,
    JAL = 3,
    BEQ = 4,
    BNE = 5,
    BLEZ = 6,
    BGTZ = 7,
    ADDI = 8,
    ADDIU = 9,
    SLTI = 10,
    SLTIU = 11,
    ANDI = 12,
    ORI = 13,
    XORI = 14,
    LUI = 15,
    COP0 = 16,
    COP1 = 17,
    COP2 = 18,
    COP3 = 19,
    LB = 32,
    LH = 33,
    LWL = 34,
    LW = 35,
    LBU = 36,
    LHU = 37,
    LWR = 38,
    SB = 40,
    SH = 41,
    SWL = 42,
    SW = 43,
    SWR = 46,
    LWC0 = 48,
    LWC1 = 49,
    LWC2 = 50,
    LWC3 = 51,
    SWC0 = 56,
    SWC1 = 57,
    SWC2 = 58,
    SWC3 = 59,
}

impl From<u8> for InstructionOp {
    fn from(value: u8) -> Self {
        match value {
            0 => InstructionOp::FUNCT,
            1 => InstructionOp::B,
            2 => InstructionOp::J,
            3 => InstructionOp::JAL,
            4 => InstructionOp::BEQ,
            5 => InstructionOp::BNE,
            6 => InstructionOp::BLEZ,
            7 => InstructionOp::BGTZ,
            8 => InstructionOp::ADDI,
            9 => InstructionOp::ADDIU,
            10 => InstructionOp::SLTI,
            11 => InstructionOp::SLTIU,
            12 => InstructionOp::ANDI,
            13 => InstructionOp::ORI,
            14 => InstructionOp::XORI,
            15 => InstructionOp::LUI,
            16 => InstructionOp::COP0,
            17 => InstructionOp::COP1,
            18 => InstructionOp::COP2,
            19 => InstructionOp::COP3,
            32 => InstructionOp::LB,
            33 => InstructionOp::LH,
            34 => InstructionOp::LWL,
            35 => InstructionOp::LW,
            36 => InstructionOp::LBU,
            37 => InstructionOp::LHU,
            38 => InstructionOp::LWR,
            40 => InstructionOp::SB,
            41 => InstructionOp::SH,
            42 => InstructionOp::SWL,
            43 => InstructionOp::SW,
            46 => InstructionOp::SWR,
            48 => InstructionOp::LWC0,
            49 => InstructionOp::LWC1,
            50 => InstructionOp::LWC2,
            51 => InstructionOp::LWC3,
            56 => InstructionOp::SWC0,
            57 => InstructionOp::SWC1,
            58 => InstructionOp::SWC2,
            59 => InstructionOp::SWC3,
            _ => panic!("Unknown Op Code: {}", value),
        }
    }
}

#[derive(Debug)]
pub enum InstructionFunct {
    SLL = 0,
    SRL = 2,
    SRA = 3,
    SLLV = 4,
    SRLV = 6,
    SRAV = 7,
    JR = 8,
    JALR = 9,
    SYSCALL = 12,
    BREAK = 13,
    MFHI = 16,
    MTHI = 17,
    MFLO = 18,
    MTLO = 19,
    MULT = 24,
    MULTU = 25,
    DIV = 26,
    DIVU = 27,
    ADD = 32,
    ADDU = 33,
    SUB = 34,
    SUBU = 35,
    AND = 36,
    OR = 37,
    XOR = 38,
    NOR = 39,
    SLT = 42,
    SLTU = 43,
}

impl From<u8> for InstructionFunct {
    fn from(value: u8) -> Self {
        match value {
            0 => InstructionFunct::SLL,
            2 => InstructionFunct::SRL,
            3 => InstructionFunct::SRA,
            4 => InstructionFunct::SLLV,
            6 => InstructionFunct::SRLV,
            7 => InstructionFunct::SRAV,
            8 => InstructionFunct::JR,
            9 => InstructionFunct::JALR,
            12 => InstructionFunct::SYSCALL,
            13 => InstructionFunct::BREAK,
            16 => InstructionFunct::MFHI,
            17 => InstructionFunct::MTHI,
            18 => InstructionFunct::MFLO,
            19 => InstructionFunct::MTLO,
            24 => InstructionFunct::MULT,
            25 => InstructionFunct::MULTU,
            26 => InstructionFunct::DIV,
            27 => InstructionFunct::DIVU,
            32 => InstructionFunct::ADD,
            33 => InstructionFunct::ADDU,
            34 => InstructionFunct::SUB,
            35 => InstructionFunct::SUBU,
            36 => InstructionFunct::AND,
            37 => InstructionFunct::OR,
            38 => InstructionFunct::XOR,
            39 => InstructionFunct::NOR,
            42 => InstructionFunct::SLT,
            43 => InstructionFunct::SLTU,
            _ => panic!("Unknown Instruction Funct: {}", value),
        }
    }
}

pub struct Instruction {
    pub bits: u32,
}

impl Instruction {
    pub fn new() -> Self {
        Self { bits: 0x0 }
    }

    pub fn get_op_code(&self) -> InstructionOp {
        return ((self.bits >> 26) as u8).try_into().unwrap();
    }

    pub fn get_rs(&self) -> u8 {
        ((self.bits >> 21) & 0x1F).try_into().unwrap()
    }

    pub fn get_base(&self) -> u8 {
        ((self.bits >> 21) & 0x1F).try_into().unwrap()
    }

    pub fn get_rt(&self) -> u8 {
        ((self.bits >> 16) & 0x1F).try_into().unwrap()
    }

    pub fn get_rd(&self) -> u8 {
        ((self.bits >> 11) & 0x1F).try_into().unwrap()
    }

    pub fn get_shamt(&self) -> u8 {
        ((self.bits >> 6) & 0x1F).try_into().unwrap()
    }

    pub fn get_immediate(&self) -> u16 {
        (self.bits & 0xFFFF).try_into().unwrap()
    }

    pub fn get_offset(&self) -> u16 {
        (self.bits & 0xFFFF).try_into().unwrap()
    }

    pub fn get_funct(&self) -> InstructionFunct {
        return ((self.bits & 0x3F) as u8).try_into().unwrap();
    }
}

impl ToString for Instruction {
    fn to_string(&self) -> String {
        format!(
            "{:04b} {:04b} {:04b} {:04b} {:04b} {:04b} {:04b} {:04b}",
            (self.bits >> 28) & 0xF,
            (self.bits >> 24) & 0xF,
            (self.bits >> 20) & 0xF,
            (self.bits >> 16) & 0xF,
            (self.bits >> 12) & 0xF,
            (self.bits >> 8) & 0xF,
            (self.bits >> 4) & 0xF,
            (self.bits >> 0) & 0xF
        )
    }
}

pub struct Registers {
    zero: u32, // r0
    at: u32,   // r1
    v0: u32,   // r2
    v1: u32,   // r3
    a0: u32,   // r4
    a1: u32,   // r5
    a2: u32,   // r6
    a3: u32,   // r7
    t0: u32,   // r8
    t1: u32,   // r9
    t2: u32,   // r10
    t3: u32,   // r11
    t4: u32,   // r12
    t5: u32,   // r13
    t6: u32,   // r14
    t7: u32,   // r15
    s0: u32,   // r16
    s1: u32,   // r17
    s2: u32,   // r18
    s3: u32,   // r19
    s4: u32,   // r20
    s5: u32,   // r21
    s6: u32,   // r22
    s7: u32,   // r23
    t8: u32,   // r24
    t9: u32,   // r25
    k0: u32,   // r26
    k1: u32,   // r27
    gp: u32,   // r28
    sp: u32,   // r29
    fp: u32,   // r30
    ra: u32,   // r31

    // not accessible to instructions
    hi: u32,      // This holds the high 32 bits of the 64-bit result
    lo: u32,      // This holds the low 32 bits of the 64-bit result
    pub pc: u32, // At execution time: the address of the next instruction to execute (already fetched)
    pub npc: u32, // At execution time: the address of the next instruction to fetch
}

impl Registers {
    pub fn new() -> Self {
        Self {
            zero: 0, // 0
            at: 0,   // 1
            v0: 0,
            v1: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            t0: 0, // 8
            t1: 0,
            t2: 0,
            t3: 0,
            t4: 0, // 12
            t5: 0,
            t6: 0,
            t7: 0,
            s0: 0, // 16
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0, // 20
            s5: 0,
            s6: 0,
            s7: 0,
            t8: 0,
            t9: 0,
            k0: 0,
            k1: 0,
            gp: 0,
            sp: 0,
            fp: 0,
            ra: 0,

            hi: 0,
            lo: 0,
            pc: RESET_VECTOR,
            npc: RESET_VECTOR + 4,
        }
    }

    pub fn read_register(&self, index: u8) -> Result<u32, String> {
        match index {
            0 => Ok(self.zero),
            8 => Ok(self.t0),
            12 => Ok(self.t4),
            16 => Ok(self.s0),
            20 => Ok(self.s4),
            24 => Ok(self.t8),
            _ => Err(format!("Read Error: Index {}", index)),
        }
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), String> {
        match index {
            0 => self.zero = value,
            _ => return Err(format!("Write Error: Index {}", index)),
        }
        Ok(())
    }

    pub fn write_register_upper(&mut self, index: u8, value: u16) -> Result<(), String> {
        match index {
            0 => self.zero = value as u32,
            _ => return Err(format!("Write Error: Index {}", index)),
        }
        Ok(())
    }
}
