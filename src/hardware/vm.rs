// Little Computer 3 or LC3
// Memory has 2^16 which is maximum addressed by 16bit int. Each stores a 16-bit value.
// 2^16 memory locations * 16 bits per location
// 65,536 * 2 bytes
// 131,072 bytes
// ~ 128 kB (1 KB = 1,024 bytes) is the memory of this LC-3 VM

use crate::hardware::instruction::execute_instruction;
use crate::hardware::register::Registers;
use libc::{FD_SET, FD_ZERO, c_void, c_int, fd_set, read, select, ssize_t, timeval};
use std::io::Read;
use std::os::unix::io::RawFd;

// FD to directly interact with standard input
const STDIN_FD: RawFd = 0;

/// Number of addressable 16-bit words. Every `u16` address is valid,
/// so `address as usize` can never be out of bounds.
const MEMORY_SIZE: usize = 1 << 16;

pub const KBSR: u16 = 0xFE00; // Keyboard Status Register: bit 15 = 1 when a key is available
pub const KBDR: u16 = 0xFE02; // Keyboard Data Register: contains the ASCII value of the key pressed
pub const DSR: u16 = 0xFE04; // Display Status Register: bit 15 = 1 when the display is ready to accept a character
pub const DDR: u16 = 0xFE06; // Display Data Register: write an ASCII character here to display it
pub const MCR: u16 = 0xFFFE; // Machine Control Register: controls whether the LC-3 is running or halted

#[derive(Debug, Clone)]
pub struct VM {
    // The full 128 kB address space, one `u16` per LC-3 word.
    pub memory: Box<[u16; MEMORY_SIZE]>,
    // R0–R7, the program counter, and the condition flag.
    pub registers: Registers,
    // Is vm running? False when HALT trap code triggered
    pub running: bool,
}

impl VM {
    pub fn new() -> Self {
        Self {
            memory: vec![0u16; MEMORY_SIZE]
                .into_boxed_slice()
                .try_into()
                .expect("allocation is exactly MEMORY_SIZE words"),
            registers: Registers::new(),
            running: true,
        }
    }

    pub fn read_memory(&mut self, address: u16) -> u16 {
        match address {
            KBSR => {
                self.handle_keyboard();
                self.memory[KBSR as usize]
            }
            KBDR => {
                // Reading the data register consumes the keypress.
                self.memory[KBSR as usize] = 0;
                self.memory[KBDR as usize]
            }
            _ => self.memory[address as usize],
        }
    }

    pub fn write_memory(&mut self, address: u16, value: u16) {
        // Todo: Implement DSR
        if address == MCR && value & (1 << 15) == 0 {
            self.running = false;
        }
        self.memory[address as usize] = value;
    }

    pub fn handle_keyboard(&mut self) {
        // A key is already waiting and hasn't been consumed — don't overwrite it.
        if self.memory[KBSR as usize] & (1 << 15) != 0 {
            return;
        }
        match poll_key() {
            Some(c) => {
                self.memory[KBSR as usize] = 1 << 15;
                self.memory[KBDR as usize] = c as u16;
            }
            None => {
                self.memory[KBSR as usize] = 0;
            }
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

/// Read one byte straight from the kernel — no BufReader anywhere, so the
/// kernel's queue is the only queue. Blocks if nothing is waiting (fd 0 is a
/// blocking fd, and VMIN=1 means "return as soon as 1 byte arrives").
/// None means EOF or error.
pub fn read_byte() -> Option<u8> {
    let mut b = 0u8;

    // unsafe because read is c style api for calling read() sys call
    // so rust can't verify completely what this does
    let n: ssize_t = unsafe {
        read(
            STDIN_FD,
            // takes mut ref to b -> convert rust ref to raw mut pointer -> convert it to a generic c pointer
            // for more than 1 byte, we can do
            // b.as_mut_ptr() as *mut c_void
            &mut b as *mut u8 as *mut c_void,
            1,
        )
    };

    // Returns -1 if error or returns how many bytes it has read. In our case 1
    if n == 1 { Some(b) } else { None }
}

/// Ask the kernel whether a byte is available *right now*. Zero timeout, so
/// this always returns immediately. If true, then read sys call or else proceed
/// select takes fd_set which has list of fd I am interested in monitoring like stdin, socket etc.
/// so we need to make it 1 for stdin only
///
fn key_available() -> bool {
    unsafe {
        let mut set: fd_set = std::mem::zeroed();
        // Empty this complete set
        FD_ZERO(&mut set);
        // Add this FD to the set which makes fd 0 as 1 and others still remain zero
        FD_SET(STDIN_FD, &mut set);
        let mut now = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let status: c_int = select(
            // range query: look at fd from 0 to nfds - 1
            STDIN_FD + 1,
            // fds im interested in checking for readability
            &mut set,
            // fds are ready for writing (Not needed for my keyboard)
            // eg: when my program wants to send data to socket, for writability 
            std::ptr::null_mut(),
            // for checking error conditions on fds
            //
            std::ptr::null_mut(),
            &mut now
        );
        
        // status > 0 no of fd ready, 0 nothing is ready or timeout, < 0 error
        status == 1
    }
}

/// Non-blocking: select is the gate, read_byte only runs on a guaranteed hit.
fn poll_key() -> Option<u8> {
    if key_available() {
        read_byte()
    } else {
        None
    }
}

pub fn execute_program(vm: &mut VM) {
    //initialize Registers
    while vm.running {
        //read instruction
        let instruction = vm.read_memory(vm.registers.pc);

        //increment program counter
        vm.registers.pc = vm.registers.pc.wrapping_add(1);

        //extract op_code and execute operation...
        execute_instruction(instruction, vm)
    }
}
