use std::fs::File;
use std::io::Read;

#[rustfmt::skip]
static OPCODE_NAMES: [&str; 64] = [
    "ALU_Op", "REGIMM", "J"   , "JAL"  , "BEQ" , "BNE" , "BLEZ", "BGTZ",
    "ADDI"  , "ADDIU" , "SLTI", "SLTIU", "ANDI", "ORI" , "XORI", "LUI" ,
    "COP0"  , "NULL"  , "COP2", "NULL" , "NULL", "NULL", "NULL", "NULL",
    "NULL"  , "NULL"  , "NULL", "NULL" , "NULL", "NULL", "NULL", "NULL",
    "LB"    , "LH"    , "LWL",  "LW"   , "LBU" , "LHU" , "LWR" , "NULL",
    "SB"    , "SH"    , "SWL",  "SW"   , "NULL", "NULL", "SWR" , "NULL",
    "NULL"  , "NULL"  , "LWC2", "NULL" , "NULL", "NULL", "NULL", "NULL",
    "NULL"  , "NULL"  , "SWC2", "HLE"  , "NULL", "NULL", "NULL", "NULL",
];

fn read_rom_file(filename: &str) -> Result<Vec<u32>, std::io::Error> {
    let mut bytes = Vec::new();
    let mut file = File::open(filename)?;
    file.read_to_end(&mut bytes)?;
    let mut program = Vec::new();
    for chunk in bytes.chunks(4) {
        program.push(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(program)
}

pub fn read() {
    let program = match read_rom_file("roms/tekken.bin") {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading ROM file: {}", e);
            std::process::exit(1);
        }
    };
    println!("Read {} instructions from ROM", program.len());

    // Parse and print instructions (MIPS R3000A)
    for i in 0..program.len() {
        let instruction = program[i];
        let opcode = instruction >> 26 as u8;
        let progress = i as f32 / program.len() as f32 * 100.0;
        println!(
            "{:02.2}% - Opcode: 0x{:02x} ({:02}) ({})",
            progress, opcode, opcode, OPCODE_NAMES[opcode as usize]
        );
    }
}
