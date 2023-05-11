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
            _ => panic!("Unknown Op Code {}", value),
        }
    }
}

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
    BREAK_ = 13,
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
    AND_ = 36,
    OR_ = 37,
    XOR_ = 38,
    NOR = 39,
    SLT = 42,
    SLTU = 43,
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
}
