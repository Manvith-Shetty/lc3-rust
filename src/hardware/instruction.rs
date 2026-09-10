use super::vm::VM;
use core::panic;

use std::io;
use std::io::Read;
use std::io::Write;
use crate::hardware::vm::read_byte;

/// Instruction set: Instruction tells cpu to perform some fundamental task.
/// Opcode(Kind of task to perform) and a set of params (provide input to the task being performed)
/// Opcode represents one task that the CPU knows "how" to do.
/// In LC-3 Each instruction is 16 bits long, with the left 4 bits storing the opcode eg: for ADD its opcode is 0001.
/// The rest of the bits are used to store the parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionSet {
    BR,   // branch
    ADD,  // add
    LD,   // load
    ST,   // store
    JSR,  // jump register
    AND,  // bitwise and
    LDR,  // load register
    STR,  // store register
    RTI,  // unused
    NOT,  // bitwise not
    LDI,  // load indirect
    STI,  // store indirect
    JMP,  // jump
    RES,  // reserved (unused)
    LEA,  // load effective address
    TRAP, // execute trap
}

/// The RCOND register stores condition flags that represents information about
/// the most recent computation. It's used for checking logical conditions.
/// The LC-3 uses only 3 condition flags which indicate the sign of the previous calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum ConditionFlag {
    // The `<<` operator is a left bit shift operator.
    // It's simpler than it looks: `n << k` means we're moving (or shifting)
    // the bits in the binary representation of the number `n` by `k`.
    // for instance, 1 in binary representation is 0000000000000001
    // 1 << 2 means we're shifting the bits twice to the left, so that
    // `1` at the end will effectively move to left, twice. It ends up being
    // 0000000000000100, which in decimal representation is 4.
    // 1 << 2 == 4.

    // So why are we storing 1, 2, 4 here?
    // In binary, with 3 bits only:
    // 1 == 001
    // 2 == 010
    // 4 == 100
    // So we're playing with the possible conditional flags settings!
    // Because the condition instruction will be `nzp` (neg, zero, pos)
    // and only one can be set at a time, it will either be
    // 001 (positive set `nz1`)
    // 010 (zero set, `n1p`)
    // 100 (negative set, `1zp`)
    // And these three binary values are 1, 2, and 4 in decimal system
    POS = 1 << 0, // Positive
    ZRO = 1 << 1, // Zero
    NEG = 1 << 2, // Negative
}

pub fn execute_instruction(instr: u16, vm: &mut VM) {
    // Extract OPCode from the instruction
    let op_code = get_op_code(&instr);

    // Match OPCode and execute instruction
    match op_code {
        Some(InstructionSet::ADD) => add(instr, vm),
        Some(InstructionSet::AND) => and(instr, vm),
        Some(InstructionSet::NOT) => not(instr, vm),
        Some(InstructionSet::BR) => br(instr, vm),
        Some(InstructionSet::JMP) => jmp(instr, vm),
        Some(InstructionSet::JSR) => jsr(instr, vm),
        Some(InstructionSet::LD) => ld(instr, vm),
        Some(InstructionSet::LDI) => ldi(instr, vm),
        Some(InstructionSet::LDR) => ldr(instr, vm),
        Some(InstructionSet::LEA) => lea(instr, vm),
        Some(InstructionSet::ST) => st(instr, vm),
        Some(InstructionSet::STI) => sti(instr, vm),
        Some(InstructionSet::STR) => str(instr, vm),
        Some(InstructionSet::TRAP) => trap(instr, vm),
        _ => panic!("Invalid OPCode"),
    }
}

// Each instruction is 16 bits long, with the left 4 bits storing the opcode.
// The rest of the bits are used to store the parameters.
// To extract left 4 bits out of the instruction, we'll use a right bit shift `>>`
// operator and shift to the right the first 4 bits 12 positions.
pub fn get_op_code(instruction: &u16) -> Option<InstructionSet> {
    match instruction >> 12 {
        0 => Some(InstructionSet::BR),
        1 => Some(InstructionSet::ADD),
        2 => Some(InstructionSet::LD),
        3 => Some(InstructionSet::ST),
        4 => Some(InstructionSet::JSR),
        5 => Some(InstructionSet::AND),
        6 => Some(InstructionSet::LDR),
        7 => Some(InstructionSet::STR),
        8 => Some(InstructionSet::RTI),
        9 => Some(InstructionSet::NOT),
        10 => Some(InstructionSet::LDI),
        11 => Some(InstructionSet::STI),
        12 => Some(InstructionSet::JMP),
        13 => Some(InstructionSet::RES),
        14 => Some(InstructionSet::LEA),
        15 => Some(InstructionSet::TRAP),
        _ => None,
    }
}

/// INSTRUCTION: ADD
/// This takes two numbers and adds them together and stores result in a register. There are two modes called register mode (0)
/// and immediate mode (1). Immediate mode makes things easier by having the value to add directly in the instruction encoding
/// rather than in a register. Usually used for increment of the value etc. Can store only value upto 2^5=32.
/// Encoding of ADD is as follows:
/// 0001: Opcode value for ADD
/// DR: Destination register
/// SR1: first number to add
/// SR2: second number to add
/// imm5: Value embedded in instruction itself
///
/// ADD R2 R0 R2;
///
/// Encoding
///      15          12 11       9 8         6 5 4      3 2          0
///       +-------------+----------+----------+-+--------+-----------+
/// ADD   |    0001     |    DR    |   SR1    |0|   00   |    SR2    |
///       +-------------+----------+----------+-+--------+-----------+
///
///      15          12 11       9 8         6 5 4                     0
///       +-------------+----------+----------+-+---------------------+
/// ADD   |    0001     |    DR    |   SR1    |1|       imm5          |
///       +-------------+----------+----------+-+---------------------+
///
pub fn add(instr: u16, vm: &mut VM) {
    // Destination register (DR) is 3 bits (9-11)found by right shifting instruction by 9 bits
    // So 'instr >> 9' will land last bit of DR to the LSB position
    // '&' with '0x7' (111) will give only the three bits
    let r0 = (instr >> 9) & 0x7;

    // First operand (SR1) is the same case (6-8)
    let r1 = (instr >> 6) & 0x7;

    // Check which mode we operating in (register mode or immediate mode)
    // '&' with 0x1 as its only 1 bit
    let imm_flag: bool = ((instr >> 5) & 0x1) != 0;

    // first value to add
    let a = vm.registers.get(r1);
    // second value to add
    let b = if imm_flag {
        sign_extend(instr & 0x1F, 5)
    } else {
        let r2 = instr & 0x7;
        vm.registers.get(r2)
    };

    // wrapping_add because in LC-3 0xFFFF + 1 is legal thus if 'a+b' and overflow will panic in debug mode
    vm.registers.update(r0, a.wrapping_add(b));
    vm.registers.update_r_cond_register(r0);
}

/// Instruction: Load Indirect (LDI)
/// Current PC has the address for LDI instruction. Now the PCoffset9 is 9 bits only so we can't use that as absolute address
/// for representing address like 0x9000 which is more than 9 bits. Now we do 'PC+PCoffset' which is again an address for the
/// data (first memory read). Now that address contains actual data (second memory read).
///
/// Why do all this?
/// Because LD inst has only 9-bit signed value offset available which can reach only
/// PC - 256 .. PC + 256
/// So if, PC = 0x9000 we cannot say LD R0, 0x9000
/// We use LDI to solve this
///          instruction
///       PC = 0x3000
///            |
///            | + small 9-bit offset
///            v
///          0x3020: 0x9000    <-- nearby pointer
///                     |
///                     | points to
///                     v
///                0x9000: 42     <-- actual data
///
/// LDI R0, #0x20
///
/// Encoding of LDi is as follows:
/// 1010: Opcode value for LDI
/// DR: Destination register
/// PCoffset9: Value embedded in instruction itself
///
/// Encoding
///      15          12 11       9 8                              0
///       +-------------+----------+------------------------------+
/// LDI   |    1010     |    DR    |            PCoffset9         |
///       +-------------+----------+------------------------------+
///
pub fn ldi(instr: u16, vm: &mut VM) {
    // Destination register (DR) is 3 bits (9-11)found by right shifting instruction by 9 bits
    // So 'instr >> 9' will land last bit of DR to the LSB position
    // '&' with '0x7' (111) will give only the three bits
    let r0 = (instr >> 9) & 0x7;

    // add pc_offset to the current PC, look at that memory location to get the final address
    let pc_offset = sign_extend(instr & 0x1FF, 9);

    // The offset is added to the PC to form the *address* we read, not to the value read.
    let first_read = vm.read_memory(vm.registers.pc.wrapping_add(pc_offset));

    // Read the resulting address and update the r0
    let second_read = vm.read_memory(first_read);
    vm.registers.update(r0, second_read);
    vm.registers.update_r_cond_register(r0);
}

/// Instruction: Bitwise and (If both bits are 1 then 1 or else 0). Two operation modes, immediate or passing a register.
///
///
/// 15           12 │11        9│8         6│ 5 │4     3│2         0
/// ┌───────────────┼───────────┼───────────┼───┼───────┼───────────┐
/// │      0101     │     DR    │  SR1      │ 0 │  00   │    SR2    │
/// └───────────────┴───────────┴───────────┴───┴───────┴───────────┘
///
///  15           12│11        9│8         6│ 5 │4                 0
/// ┌───────────────┼───────────┼───────────┼───┼───────────────────┐
/// │      0101     │     DR    │  SR1      │ 1 │       IMM5        │
/// └───────────────┴───────────┴───────────┴───┴───────────────────┘
///
pub fn and(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let r1 = (instr >> 6) & 0x7;
    let imm_flag = ((instr >> 5) & 0x1) != 0;

    let a = vm.registers.get(r1);

    let b = if imm_flag {
        sign_extend(instr & 0x1F, 5)
    } else {
        let r2 = instr & 0x7;
        vm.registers.get(r2)
    };

    // `a` and `b` are register *contents*, not register numbers.
    let result = a & b;

    vm.registers.update(r0, result);
    vm.registers.update_r_cond_register(r0);
}

/// Instruction: Bitwise not (1 -> 0 or 0 -> 1)
///
/// 15           12 │11        9│8         6│ 5 │4                 0
/// ┌───────────────┼───────────┼───────────┼───┼───────────────────┐
/// │      1001     │     DR    │     SR    │ 1 │       1111        │
/// └───────────────┴───────────┴───────────┴───┴───────────────────┘
///
pub fn not(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let r1 = (instr >> 6) & 0x7;

    vm.registers.update(r0, !vm.registers.get(r1));

    vm.registers.update_r_cond_register(r0);
}

/// Instruction: Branch
/// The branching operation; means to go somewhere else in the assembly code
/// depending on whether some conditions are met. LC-3 if + goto instruction
/// The condition codes specified by the state of bits [11:9] are tested.
/// If bit [11] is set, N is tested; if bit [11] is clear, N is not tested.
/// If bit [10] is set, Z is tested. If any of the condition codes tested is set,
/// the program branches to the location specified by
/// adding the sign-extended PCOffset9 field to the incremented PC.
///
/// 15           12 │11 │10 │ 9 │8                                 0
/// ┌───────────────┼───┼───┼───┼───────────────────────────────────┐
/// │      0000     │ N │ Z │ P │             PCOffset9             │
/// └───────────────┴───┴───┴───┴───────────────────────────────────┘
///
/// eg: BRz #5 - branch if last result was zero
///     BRn #1 - branch if last result was negative
///     BRp #5 - branch if last result was positive
///
/// So if above condition is true the next instruction to be executed will be 'PC+PCOffset' one
///
pub fn br(instr: u16, vm: &mut VM) {
    let pc_offset = sign_extend(instr & 0x1FF, 9);

    let cond_flag = (instr >> 9) & 0x7;

    // We are doing is and operation on cond flag in instr '001', or '010' or '100'
    // with prev cond flag in register. And if it is true, then increment PC with offset
    if cond_flag & vm.registers.cond != 0 {
        let offset = vm.registers.pc.wrapping_add(pc_offset);
        vm.registers.pc = offset;
    }

    // If the branch isn't taken (no condition met), PC isn't changed and PC will increment to next
    // instruction to be executed
}

/// Instruction: Jump
/// The program unconditionally jumps to the location specified by the contents of the base register.
/// Bits [8:6] identify the base register. `RET` is listed as a separate instruction
/// in the specification, since it is a different keyword in assembly.
/// However, it is actually a special case of JMP. RET happens whenever R1 is 7.
///
///  15           12│11        9│8         6│ 5                    0
/// ┌───────────────┼───────────┼───────────┼───────────────────────┐
/// │      1100     │    000    │   BaseR   │       00000           │
/// └───────────────┴───────────┴───────────┴───────────────────────┘
///  15           12│11        9│8         6│ 5                    0
/// ┌───────────────┼───────────┼───────────┼───────────────────────┐
/// │      1100     │    000    │    111    │       00000           │
/// └───────────────┴───────────┴───────────┴───────────────────────┘
///
pub fn jmp(instr: u16, vm: &mut VM) {
    // base_reg will either be an arbitrary register or the register 7 (`111`) which in this
    // case it would be the `RET` operation.
    let base_reg = (instr >> 6) & 0x7;
    vm.registers.pc = vm.registers.get(base_reg);
}

/// Instruction: Jump register
/// First, the incremented PC is saved in R7.
/// This is the linkage back to the calling routine.
/// Then the PC is loaded with the address of the first instruction of the subroutine,
/// causing an unconditional jump to that address.
/// The address of the subroutine is obtained from the base register (if bit [11] is 0),
/// or the address is computed by sign-extending bits [10:0] and adding this value to the incremented PC (if bit [11] is 1).

///  15           12│11 │10
/// ┌───────────────┼───┼───────────────────────────────────────────┐
/// │      0100     │ 1 │                PCOffset11                 │
/// └───────────────┴───┴───────────────────────────────────────────┘
///  15           12│11 │10    9│8     6│5                         0
/// ┌───────────────┼───┼───────┼───────┼───────────────────────────┐
/// │      0100     │ 0 │   00  │ BaseR │           00000           │
/// └───────────────┴───┴───────┴───────┴───────────────────────────┘
///
pub fn jsr(instr: u16, vm: &mut VM) {
    let long_flag = (instr >> 11) & 0x1 != 0;
    vm.registers.r7 = vm.registers.pc;
    if long_flag {
        let long_pc_offset = sign_extend(instr & 0x7FF, 11);
        vm.registers.pc = vm.registers.pc.wrapping_add(long_pc_offset);
    } else {
        let r1 = (instr >> 6) & 0x7;
        vm.registers.pc = vm.registers.get(r1);
    }
}

/// Instruction: Load
/// An address is computed by sign-extending bits [8:0] to 16 bits and
/// adding this value to the incremented PC.
/// The contents of memory at this address are loaded into DR.
/// The condition codes are set, based on whether the value loaded is negative, zero, or positive.
///
///  15           12│11        9│8                                 0
/// ┌───────────────┼───────────┼───────────────────────────────────┐
/// │      0010     │     DR    │            PCOffset9              │
/// └───────────────┴───────────┴───────────────────────────────────┘
///
pub fn ld(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let pc_offset = sign_extend(instr & 0x1FF, 9);

    // Read the value from the memory above was computed
    let value = vm.read_memory(vm.registers.pc.wrapping_add(pc_offset));

    // Save that value to the dest. register and update the condition register
    vm.registers.update(r0, value);
    vm.registers.update_r_cond_register(r0);
}

/// Instruction: Load register
/// Load base + offset
/// An address is computed by sign-extending bits [5:0] to 16 bits
/// and adding this value to the contents of the register specified by bits [8:6].
/// The contents of memory at this address are loaded into DR.
/// The condition codes are set, based on whether the value loaded is negative, zero, or positive.
///
///  15           12│11        9│8             6│5                 0
/// ┌───────────────┼───────────┼───────────────┼───────────────────┐
/// │      1010     │     DR    │     BaseR     │     PCOffset6     │
/// └───────────────┴───────────┴───────────────┴───────────────────┘
///
pub fn ldr(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let r1 = (instr >> 6) & 0x7;
    let pc_offset = sign_extend(instr & 0x3F, 6);

    let value = vm.read_memory(vm.registers.get(r1).wrapping_add(pc_offset));

    vm.registers.update(r0, value);
    vm.registers.update_r_cond_register(r0);
}

/// Instruction: Load effective address
/// An address is computed by sign-extending bits [8:0] to 16 bits and adding
/// this value to the incremented PC.
/// This address is loaded into DR. The condition codes are set, based on whether the
/// value loaded is negative, zero, or positive.
///
///  15           12│11        9│8                                 0
/// ┌───────────────┼───────────┼───────────────────────────────────┐
/// │      1110     │     DR    │            PCOffset9              │
/// └───────────────┴───────────┴───────────────────────────────────┘
///
pub fn lea(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let pc_offset = sign_extend(instr & 0x1FF, 9);

    let value = vm.registers.pc.wrapping_add(pc_offset);

    vm.registers.update(r0, value);
    vm.registers.update_r_cond_register(r0);
}

/// Instruction: Store
/// The contents of the register specified by SR are stored in the memory location
/// whose address is computed by sign-extending bits [8:0] to 16 bits and adding
/// this value to the incremented PC.
///
///  15           12│11        9│8                                 0
/// ┌───────────────┼───────────┼───────────────────────────────────┐
/// │      0011     │     SR    │            PCOffset9              │
/// └───────────────┴───────────┴───────────────────────────────────┘
///
pub fn st(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let pc_offset = sign_extend(instr & 0x1FF, 9);

    // Store the *contents* of SR, not the register number.
    let value = vm.registers.get(r0);
    vm.write_memory(vm.registers.pc.wrapping_add(pc_offset), value);
}

/// Instruction: Store indirect
/// The contents of the register specified by SR are stored in the memory location
/// whose address is obtained as follows: Bits [8:0] are sign-extended to 16 bits and added to the incremented PC.
/// What is in memory at this address is the address of the location to which the data in SR is stored.
///
///  15           12│11        9│8                                 0
/// ┌───────────────┼───────────┼───────────────────────────────────┐
/// │      1011     │     SR    │            PCOffset9              │
/// └───────────────┴───────────┴───────────────────────────────────┘
///
pub fn sti(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let pc_offset = sign_extend(instr & 0x1FF, 9);

    let addr = vm.read_memory(vm.registers.pc.wrapping_add(pc_offset));

    vm.write_memory(addr, vm.registers.get(r0));
}

/// Instruction: Store register
/// The contents of the register specified by SR are stored in the memory location
/// whose address is computed by sign-extending bits [5:0] to 16 bits
/// and adding this value to the contents of the register specified by bits [8:6].
///
///  15           12│11        9│8         6│                      0
/// ┌───────────────┼───────────┼───────────┼───────────────────────┐
/// │      0111     │     SR    │   BaseR   │        PCOffset6      │
/// └───────────────┴───────────┴───────────┴───────────────────────┘
///
pub fn str(instr: u16, vm: &mut VM) {
    let r0 = (instr >> 9) & 0x7;
    let r1 = (instr >> 6) & 0x7;

    let pc_offset = sign_extend(instr & 0x3F, 6);

    // SR holds the value to store; it is not an address to load from.
    let value = vm.registers.get(r0);
    let addr = vm.registers.get(r1).wrapping_add(pc_offset);

    vm.write_memory(addr, value);
}

// Sign extending means adding the bit mask of 1 in front of our number to make it 16 bits.
// In case of ADD imm val which is 5 bits to add the value with u16 in register we need to make it
// u16.
pub fn sign_extend(x: u16, bit_count: u16) -> u16 {
    let mut val: u16 = x;
    // Checks for negative value by checking if the MSB is set or not. eg: 0000 1010 and bit_count=4 (to consider)
    if (x >> (bit_count - 1)) & 1 != 0 {
        // does masking of the bits.
        // 0000 0000 0000 1010
        // 1111 1111 1111 0000
        // -------------------
        // 1111 1111 1111 1010
        val |= 0xFFFF << bit_count;
    }
    val
}

/// Trap routines are few predefined routines for performing common tasks and interacting with I/O devices
/// Each trap routine has trap code which identifies it (similar to opcode)
///  Because of these, programs start at address 0x3000 instead of 0x0 as lower address are left empty for trap routine code.
/// When trap code is called, PC moved to that code's address. CPU executes the instructions and after done PC resets to
/// location following the initial call
///
///  15           12│11        8│7                                  0
/// ┌───────────────┼───────────┼─────────────────────────────────┐
/// │      1111     │   0000    │           trapvect8             │
/// └───────────────┴───────────┴─────────────────────────────────┘
///
///

#[derive(Debug, Clone, PartialEq, Eq)]
enum Trap {
    Getc,  // get character from keyboard, not echoed onto the terminal
    Out,   // output a character
    Puts,  // output a word string
    In,    // get character from keyboard, echoed onto the terminal
    Putsp, // output a byte string
    Halt,  // halt the program
}

pub fn trap(instr: u16, vm: &mut VM) {
    let trap_vector = instr & 0xFF;
    vm.registers.r7 = vm.registers.pc;

    match trap_vector {
        0x20 => trap_getc(vm),
        0x21 => trap_out(vm),
        0x22 => trap_puts(vm),
        0x23 => trap_in(vm),
        0x24 => trap_putsp(vm),
        0x25 => trap_halt(vm),
        _ => panic!("Unknown trap"),
    }
}

pub fn trap_getc(vm: &mut VM) {
    match read_byte() {
        Some(b) => vm.registers.update(0, b as u16),
        None => vm.running = false,
    }
}

pub fn trap_out(vm: &mut VM) {
    let c = vm.registers.r0 as u8;
    // Without newline as println macro prints it with new line
    print!("{}", c as char);
    // stdout is line-buffered, so a prompt that doesn't end in `\n` would
    // otherwise sit invisible until the next newline.
    io::stdout().flush().expect("failed to flush");
}

pub fn trap_puts(vm: &mut VM) {
    let mut index = vm.registers.r0;

    loop {
        let c = vm.read_memory(index);

        if c == 0x0 {
            break;
        }

        print!("{}", (c as u8) as char);
        index += 1;
    }

    io::stdout().flush().expect("failed to flush");
}

// Todo: same reason as getc
// In, Print a prompt on the screen and read a single character from the keyboard.
pub fn trap_in(vm: &mut VM) {
    print!("Enter a  character : ");
    io::stdout().flush().expect("failed to flush");

    match read_byte() {
        Some(b) => {
            print!("{}", b as char);
            io::stdout().flush().expect("failed to flush");
            vm.registers.update(0, b as u16);
        }
        None => vm.running = false,
    }
}

pub fn trap_putsp(vm: &mut VM) {
    let mut index = vm.registers.r0;

    loop {
        let c = vm.read_memory(index);

        if c == 0x0 {
            break;
        }

        let low = (c & 0xFF) as u8;
        let high = (c >> 8) as u8;

        print!("{}", low as char);

        if high != 0u8 {
            print!("{}", high as char);
        }

        index += 1;
    }

    io::stdout().flush().expect("failed to flush");
}

pub fn trap_halt(vm: &mut VM) {
    println!("HALT detected");
    io::stdout().flush().expect("failed to flush");
    vm.running = false;
}
