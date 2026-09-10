pub mod hardware;

use std::env;
use std::fs::File;
use std::io::Read;

use crate::hardware::vm::{VM, execute_program};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: lc3-vm <image.obj>");
        std::process::exit(1);
    }
    // Initialize vm with memory, registers and flag (empty right now)
    let mut vm = VM::new();

    let obj_path = &args[1];
    println!("Loading: {}", obj_path);

    let mut f = File::open(obj_path).expect("Error when opening the file");

    // Read first 2 bytes
    let mut base_address_bytes = [0u8; 2];
    f.read_exact(&mut base_address_bytes)
        .expect("Error reading origin");

    // LC-3 object files are big-endian
    let base_address = u16::from_be_bytes(base_address_bytes);
    // Initialize the pc with base address (0x3000) in this case
    vm.registers.pc = base_address;

    println!("Base address: {:#06X}", base_address); 

    // Here we're loading the program in memory
    let mut address = base_address;
    loop {
       let mut instr_bytes = [0u8; 2];

       // Read next two bytes
       match f.read_exact(&mut instr_bytes) {
            Ok(_) => {
                // In LC-3 words are big-endian
                let instruction = u16::from_be_bytes(instr_bytes);
                vm.write_memory(address, instruction);
                address = address.wrapping_add(1);
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    println!("OK")
                } else {
                    println!("failed: {}", e);
                }
                break;
            }
       }
    }

    // Now start executing the program
    execute_program(&mut vm);
}