use log::{debug, error, info, warn};
use std::result::Result;

const RESET_VECTOR: u32 = 0xBFC00000;

// R3000A is based on the MIPS III instruction set architecture

// Opcode/Parameter Instruction Encoding
// ============================================================================
//
//  31..26 |25..21|20..16|15..11|10..6 |  5..0  |
//   6bit  | 5bit | 5bit | 5bit | 5bit |  6bit  |
//  -------+------+------+------+------+--------+------------
//  000000 | N/A  | rt   | rd   | imm5 | 0000xx | shift-imm
//  000000 | rs   | rt   | rd   | N/A  | 0001xx | shift-reg
//  000000 | rs   | N/A  | N/A  | N/A  | 001000 | jr
//  000000 | rs   | N/A  | rd   | N/A  | 001001 | jalr
//  000000 | <-----comment20bit------> | 00110x | sys/brk
//  000000 | N/A  | N/A  | rd   | N/A  | 0100x0 | mfhi/mflo
//  000000 | rs   | N/A  | N/A  | N/A  | 0100x1 | mthi/mtlo
//  000000 | rs   | rt   | N/A  | N/A  | 0110xx | mul/div
//  000000 | rs   | rt   | rd   | N/A  | 10xxxx | alu-reg
//  000001 | rs   | 00000| <--immediate16bit--> | bltz
//  000001 | rs   | 00001| <--immediate16bit--> | bgez
//  000001 | rs   | 10000| <--immediate16bit--> | bltzal
//  000001 | rs   | 10001| <--immediate16bit--> | bgezal
//  00001x | <---------immediate26bit---------> | j/jal
//  00010x | rs   | rt   | <--immediate16bit--> | beq/bne
//  00011x | rs   | N/A  | <--immediate16bit--> | blez/bgtz
//  001xxx | rs   | rt   | <--immediate16bit--> | alu-imm
//  001111 | N/A  | rt   | <--immediate16bit--> | lui-imm
//  100xxx | rs   | rt   | <--immediate16bit--> | load rt,[rs+imm]
//  101xxx | rs   | rt   | <--immediate16bit--> | store rt,[rs+imm]
//  x1xxxx | <------coprocessor specific------> | coprocessor (see below)
//
// Coprocessor Opcode/Parameter Instruction Encoding
// ============================================================================
//
//  31..26 |25..21|20..16|15..11|10..6 |  5..0  |
//   6bit  | 5bit | 5bit | 5bit | 5bit |  6bit  |
//  -------+------+------+------+------+--------+------------
//  0100nn |0|0000| rt   | rd   | N/A  | 000000 | MFCn rt,rd_dat  ;rt = dat
//  0100nn |0|0010| rt   | rd   | N/A  | 000000 | CFCn rt,rd_cnt  ;rt = cnt
//  0100nn |0|0100| rt   | rd   | N/A  | 000000 | MTCn rt,rd_dat  ;dat = rt
//  0100nn |0|0110| rt   | rd   | N/A  | 000000 | CTCn rt,rd_cnt  ;cnt = rt
//  0100nn |0|1000|00000 | <--immediate16bit--> | BCnF target ;jump if false
//  0100nn |0|1000|00001 | <--immediate16bit--> | BCnT target ;jump if true
//  0100nn |1| <--------immediate25bit--------> | COPn imm25
//  010000 |1|0000| N/A  | N/A  | N/A  | 000001 | COP0 01h  ;=TLBR, unused on PS1
//  010000 |1|0000| N/A  | N/A  | N/A  | 000010 | COP0 02h  ;=TLBWI, unused on PS1
//  010000 |1|0000| N/A  | N/A  | N/A  | 000110 | COP0 06h  ;=TLBWR, unused on PS1
//  010000 |1|0000| N/A  | N/A  | N/A  | 001000 | COP0 08h  ;=TLBP, unused on PS1
//  010000 |1|0000| N/A  | N/A  | N/A  | 010000 | COP0 10h  ;=RFE
//  1100nn | rs   | rt   | <--immediate16bit--> | LWCn rt_dat,[rs+imm]
//  1110nn | rs   | rt   | <--immediate16bit--> | SWCn rt_dat,[rs+imm]
//
//
// From https://psx-spx.consoledev.net/cpuspecifications/

// Exception Handling
// ============================================================================
//
// cop0r13 - CAUSE - (Read-only, except, Bit8-9 are R/W)
//
// Describes the most recently recognised exception
//
//  0-1   -      Not used (zero)
//  2-6   Excode Describes what kind of exception occured:
//                 00 INT     Interrupt
//                 01 MOD     Tlb modification (none such in PSX)
//                 02 TLBL    Tlb load         (none such in PSX)
//                 03 TLBS    Tlb store        (none such in PSX)
//                 04 AdEL    Address error, Data load or Instruction fetch
//                 05 AdES    Address error, Data store
//                            The address errors occur when attempting to read
//                            outside of KUseg in user mode and when the address
//                            is misaligned. (See also: BadVaddr register)
//                 06 IBE     Bus error on Instruction fetch
//                 07 DBE     Bus error on Data load/store
//                 08 Syscall Generated unconditionally by syscall instruction
//                 09 BP      Breakpoint - break instruction
//                 0A RI      Reserved instruction
//                 0B CpU     Coprocessor unusable
//                 0C Ov      Arithmetic overflow
//                 0D-1F      Not used
//  7     -      Not used (zero)
//  8-15  Ip     Interrupt pending field. Bit 8 and 9 are R/W, and
//               contain the last value written to them. As long
//               as any of the bits are set they will cause an
//               interrupt if the corresponding bit is set in IM.
//  16-27 -      Not used (zero)
//  28-29 CE     Contains the coprocessor number if the exception
//               occurred because of a coprocessor instuction for
//               a coprocessor which wasn't enabled in SR.
//  30    -      Not used (zero)
//  31    BD     Is set when last exception points to the
//               branch instuction instead of the instruction
//               in the branch delay slot, where the exception
//               occurred.
//
// cop0r12 - SR - System status register (R/W)
//
//  0     IEc Current Interrupt Enable  (0=Disable, 1=Enable) ;rfe pops IUp here
//  1     KUc Current Kernel/User Mode  (0=Kernel, 1=User)    ;rfe pops KUp here
//  2     IEp Previous Interrupt Disable                      ;rfe pops IUo here
//  3     KUp Previous Kernel/User Mode                       ;rfe pops KUo here
//  4     IEo Old Interrupt Disable                       ;left unchanged by rfe
//  5     KUo Old Kernel/User Mode                        ;left unchanged by rfe
//  6-7   -   Not used (zero)
//  8-15  Im  8 bit interrupt mask fields. When set the corresponding
//            interrupts are allowed to cause an exception.
//  16    Isc Isolate Cache (0=No, 1=Isolate)
//              When isolated, all load and store operations are targetted
//              to the Data cache, and never the main memory.
//              (Used by PSX Kernel, in combination with Port FFFE0130h)
//  17    Swc Swapped cache mode (0=Normal, 1=Swapped)
//              Instruction cache will act as Data cache and vice versa.
//              Use only with Isc to access & invalidate Instr. cache entries.
//              (Not used by PSX Kernel)
//  18    PZ  When set cache parity bits are written as 0.
//  19    CM  Shows the result of the last load operation with the D-cache
//            isolated. It gets set if the cache really contained data
//            for the addressed memory location.
//  20    PE  Cache parity error (Does not cause exception)
//  21    TS  TLB shutdown. Gets set if a programm address simultaneously
//            matches 2 TLB entries.
//            (initial value on reset allows to detect extended CPU version?)
//  22    BEV Boot exception vectors in RAM/ROM (0=RAM/KSEG0, 1=ROM/KSEG1)
//  23-24 -   Not used (zero)
//  25    RE  Reverse endianness   (0=Normal endianness, 1=Reverse endianness)
//              Reverses the byte order in which data is stored in
//              memory. (lo-hi -> hi-lo)
//              (Affects only user mode, not kernel mode) (?)
//              (The bit doesn't exist in PSX ?)
//  26-27 -   Not used (zero)
//  28    CU0 COP0 Enable (0=Enable only in Kernel Mode, 1=Kernel and User Mode)
//  29    CU1 COP1 Enable (0=Disable, 1=Enable) (none in PSX)
//  30    CU2 COP2 Enable (0=Disable, 1=Enable) (GTE in PSX)
//  31    CU3 COP3 Enable (0=Disable, 1=Enable) (none in PSX)
//
//
// From https://psx-spx.consoledev.net/cpuspecifications/#cop0-exception-handling

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

#[derive(Debug)]
pub enum CopCommonInstruction {
    MFCN = 0b0000, // Move From Coprocessor N
    CFCN = 0b0010, // Coprocessor Register to General Purpose Register Move
    MTCN = 0b0100, // Move To Coprocessor N
    CTCN = 0b0110, // Coprocessor To Coprocessor Register Transfer
}

impl From<u8> for CopCommonInstruction {
    fn from(value: u8) -> Self {
        match value {
            0 => CopCommonInstruction::MFCN,
            2 => CopCommonInstruction::CFCN,
            4 => CopCommonInstruction::MTCN,
            6 => CopCommonInstruction::CTCN,
            _ => panic!("Unknown Cop Common Instruction: {}", value),
        }
    }
}

#[derive(Debug)]
pub enum Cop0Instruction {
    TLBR = 0x01,  // Translation Lookaside Buffer Read
    TLBWI = 0x02, // Translation Lookaside Buffer Write Indexed
    TLBWR = 0x04, // Translation Lookaside Buffer Write
    TLBP = 0x08,  // Translation Lookaside Buffer Probe
    RFE = 0x10,   // Return From Exception
}

impl From<u8> for Cop0Instruction {
    fn from(value: u8) -> Self {
        match value {
            1 => Cop0Instruction::TLBR,
            2 => Cop0Instruction::TLBWI,
            4 => Cop0Instruction::TLBWR,
            8 => Cop0Instruction::TLBP,
            16 => Cop0Instruction::RFE,
            _ => panic!("Unknown Cop0 Instruction: {}", value),
        }
    }
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

    pub fn get_target(&self) -> u32 {
        (self.bits & 0x3FFFFFF).try_into().unwrap()
    }

    pub fn get_funct(&self) -> InstructionFunct {
        ((self.bits & 0x3F) as u8).try_into().unwrap()
    }

    pub fn is_cop_common_instruction(&self) -> bool {
        ((self.bits >> 25) & 0x1) == 0
    }

    pub fn get_cop_number(&self) -> u8 {
        ((self.bits >> 26) & 0x3).try_into().unwrap()
    }

    pub fn get_cop_common_op(&self) -> CopCommonInstruction {
        (((self.bits >> 21) & 0xF) as u8).try_into().unwrap()
    }

    pub fn get_cop0_op(&self) -> Cop0Instruction {
        ((self.bits & 0x3F) as u8).try_into().unwrap()
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
    // General-Purpose Registers (GPRs):
    //
    //  Name       Alias    Common Usage
    //  R0         zero     Constant (always 0)
    //  R1         at       Assembler temporary (destroyed by some assembler pseudoinstructions!)
    //  R2-R3      v0-v1    Subroutine return values, may be changed by subroutines
    //  R4-R7      a0-a3    Subroutine arguments, may be changed by subroutines
    //  R8-R15     t0-t7    Temporaries, may be changed by subroutines
    //  R16-R23    s0-s7    Static variables, must be saved by subs
    //  R24-R25    t8-t9    Temporaries, may be changed by subroutines
    //  R26-R27    k0-k1    Reserved for kernel (destroyed by some IRQ handlers!)
    //  R28        gp       Global pointer (rarely used)
    //  R29        sp       Stack pointer
    //  R30        fp(s8)   Frame Pointer, or 9th Static variable, must be saved
    //  R31        ra       Return address (used so by JAL,BLTZAL,BGEZAL opcodes)
    gprs: [u32; 32],

    // Not accessible to instructions
    pub hi: u32,  // This holds the high 32 bits of the 64-bit result
    pub lo: u32,  // This holds the low 32 bits of the 64-bit result
    pub pc: u32, // At execution time: the address of the next instruction to execute (already fetched)
    pub npc: u32, // At execution time: the address of the next instruction to fetch
}

impl Registers {
    pub fn new() -> Self {
        Self {
            gprs: [0; 32],

            hi: 0,
            lo: 0,
            pc: RESET_VECTOR,
            npc: RESET_VECTOR + 4,
        }
    }

    pub fn read_register(&self, index: u8) -> u32 {
        self.gprs[index as usize]
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> () {
        self.gprs[index as usize] = value;
    }

    pub fn write_register_upper(&mut self, index: u8, value: u16) -> Result<(), String> {
        Ok(self.gprs[index as usize] = (value as u32) << 16)
    }

    pub fn dump(&self) -> () {
        // ze at v0 v1 a0 a1 a2 a3
        // t0 t1 t2 t3 t4 t5 t6 t7
        // s0 s1 s2 s3 s4 s5 s6 s7
        // t8 t9 k0 k1 gp sp fp ra
        let r = self.gprs;
        println!(
            "{:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x}",
            r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7]
        );
        println!(
            "{:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x}",
            r[8], r[9], r[10], r[11], r[12], r[13], r[14], r[15]
        );
        println!(
            "{:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x}",
            r[16], r[17], r[18], r[19], r[20], r[21], r[22], r[23]
        );
        println!(
            "{:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x}",
            r[24], r[25], r[26], r[27], r[28], r[29], r[30], r[31]
        );
        println!("LO: {:8x}", self.lo);
        println!("HI: {:8x}", self.hi);
    }
}

#[derive(Debug)]
pub enum Cop0Reg {
    BPC = 3,
    BDA = 5,
    JUMPDEST = 6,
    DCIC = 7,
    BadVaddr = 8,
    BDAM = 9,
    BPCM = 11,
    SR = 12,
    CAUSE = 13,
    EPC = 14,
    PRID = 15,
}

impl From<u8> for Cop0Reg {
    fn from(value: u8) -> Self {
        match value {
            3 => Cop0Reg::BPC,
            5 => Cop0Reg::BDA,
            6 => Cop0Reg::JUMPDEST,
            7 => Cop0Reg::DCIC,
            8 => Cop0Reg::BadVaddr,
            9 => Cop0Reg::BDAM,
            11 => Cop0Reg::BPCM,
            12 => Cop0Reg::SR,
            13 => Cop0Reg::CAUSE,
            14 => Cop0Reg::EPC,
            15 => Cop0Reg::PRID,
            _ => panic!("Unknown Cop0 Reg: {}", value),
        }
    }
}

#[derive(Debug)]
pub enum Exception {
    INT = 0x00,     // interrupt
    MOD = 0x01,     // tlb modification
    TLBL = 0x02,    // tlb load
    TLBS = 0x03,    // tlb store
    AdEL = 0x04,    // address error, data load/instruction fetch
    AdES = 0x05,    // address error, data store
    IBE = 0x06,     // bus error on instruction fetch
    DBE = 0x07,     // bus error on data load/store
    Syscall = 0x08, // system call instruction
    BP = 0x09,      // break instruction
    RI = 0x0A,      // reserved instruction
    CpU = 0x0B,     // coprocessor unusable
    Ov = 0x0C,      // arithmetic overflow
}

impl Exception {
    pub fn get_excode(&self) -> u8 {
        match self {
            Exception::INT => 0x00,
            Exception::MOD => 0x01,
            Exception::TLBL => 0x02,
            Exception::TLBS => 0x03,
            Exception::AdEL => 0x04,
            Exception::AdES => 0x05,
            Exception::IBE => 0x06,
            Exception::DBE => 0x07,
            Exception::Syscall => 0x08,
            Exception::BP => 0x09,
            Exception::RI => 0x0A,
            Exception::CpU => 0x0B,
            Exception::Ov => 0x0C,
        }
    }
}

pub struct Cop0SystemStatusRegister {
    bits: u32,
}

impl Cop0SystemStatusRegister {
    pub fn get_mode_bits(&self) -> u8 {
        (self.bits & 0x3F) as u8
    }

    pub fn set_mode_bits(&mut self, mode_bits: u8) -> () {
        let masked_bits = self.bits & 0xFFFFFFC0;
        let masked_mode_bits = (mode_bits as u32) & 0x3F;
        self.bits = masked_bits | masked_mode_bits;
    }

    pub fn get_isc(&self) -> bool {
        // If cache isolated (disabled),
        // no writes to memory occur
        (self.bits >> 16) == 0x1
    }

    pub fn get_bev(&self) -> bool {
        (self.bits << 22) == 0x1
    }
}

pub struct Cop0CauseRegister {
    bits: u32,
}

impl Cop0CauseRegister {
    pub fn set_excode(&mut self, value: u8) -> () {
        // 5 bits: 2-6
        assert!(value < 0x0D);
        let masked_bits = self.bits & !0x7C;
        let masked_value = (value & 0x1F) as u32;
        self.bits = masked_bits | (masked_value << 2);
    }

    pub fn get_interrupt_pending(&self) -> u8 {
        // 8 bits: 8-15
        ((self.bits >> 8) & 0xFF) as u8
    }

    pub fn set_interrupt_pending(&mut self, value: u8) -> () {
        // 8 bits: 8-15
        let masked_bits = self.bits & !0xFF00;
        self.bits = masked_bits | ((value as u32) << 8);
    }
}

pub struct Cop0Registers {
    bpc: u32,                         // 3: Breakpoint on execute
    bda: u32,                         // 5: Breakpoint on data access
    tar: u32,                         // 6: Randomly memorized jump address
    dcic: u32,                        // 7: Data cache invalidate by index
    bad_vaddr: u32,                   // 8: Bad virtual address value
    bdam: u32,                        // 9: Data breakpoint mask
    bpcm: u32,                        // 11: Execute breakpoint mask
    pub sr: Cop0SystemStatusRegister, // 12: System status register
    pub cause: Cop0CauseRegister,     // 13: Exception cause
    pub epc: u32,                     // 14: Return address from trap (Exception Program Counter)
    prid: u32,                        // 15: Processor ID
}

impl Cop0Registers {
    pub fn new() -> Self {
        Self {
            bpc: 0x0,
            bda: 0x0,
            tar: 0x0,
            bad_vaddr: 0x0,
            bdam: 0x0,
            bpcm: 0x0,
            epc: 0x0,
            prid: 0x0,
            sr: Cop0SystemStatusRegister { bits: 0 },
            dcic: 0x0,
            cause: Cop0CauseRegister { bits: 0 },
        }
    }

    pub fn read_register(&self, reg: Cop0Reg) -> u32 {
        match reg {
            Cop0Reg::BPC => self.bpc,
            Cop0Reg::CAUSE => self.cause.bits,
            Cop0Reg::EPC => self.epc,
            Cop0Reg::SR => self.sr.bits,
            _ => panic!("Cop0 Register Read Error: {:?}", reg),
        }
    }

    pub fn write_register(&mut self, reg: Cop0Reg, value: u32) -> () {
        match reg {
            Cop0Reg::BDA => self.bda = value,
            Cop0Reg::BDAM => self.bdam = value,
            Cop0Reg::BPC => self.bpc = value,
            Cop0Reg::BPCM => self.bpcm = value,
            Cop0Reg::CAUSE => self.cause.bits = value,
            Cop0Reg::DCIC => self.dcic = value,
            Cop0Reg::JUMPDEST => self.tar = value,
            Cop0Reg::SR => self.sr.bits = value,
            _ => panic!("Cop0 Register Write Error: {:?}", reg),
        }
    }

    pub fn dump(&self) -> () {
        // bpc bda  tar   bad_vaddr bdam bpcm epc prid
        // sr  dcic cause
        println!(
            "{:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x} {:8x}",
            self.bpc,
            self.bda,
            self.tar,
            self.dcic,
            self.bad_vaddr,
            self.bdam,
            self.bpcm,
            self.sr.bits
        );
        println!("{:8x} {:8x} {:8x}", self.cause.bits, self.epc, self.prid);
    }
}
