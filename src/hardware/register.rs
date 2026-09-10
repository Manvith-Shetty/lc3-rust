use crate::hardware::instruction::ConditionFlag;

// Starting address for PC
const PC_START: u16 = 0x3000;

// In LC-3, A total of 10 Registers (16 bits each). 
// General purpose: Used to perform any program calculations.
// PC is an unsigned integer which is the address of the next instruction in memory to execute. 
// Starts the PC from TODO
// Condition flags tells us about the condition of the previous calculation.

// R0-R7: General purpose registers
// PC: Program counter register 
// COND: Condition flag register
#[derive(Debug, Clone)]
pub struct Registers {
  pub r0: u16,
  pub r1: u16,
  pub r2: u16,
  pub r3: u16,
  pub r4: u16,
  pub r5: u16,
  pub r6: u16, 
  pub r7: u16,
  pub pc: u16,
  pub cond: u16
}

impl Registers {
  pub fn new() -> Self {
    Self {
      r0: 0,
      r1: 0,
      r2: 0,
      r3: 0,
      r4: 0,
      r5: 0,
      r6: 0,
      r7: 0,
      pc: 0,
      cond: 0
    }
  }

pub fn update(&mut self, index: u16, value: u16) {
        match index {
            0 => self.r0 = value,
            1 => self.r1 = value,
            2 => self.r2 = value,
            3 => self.r3 = value,
            4 => self.r4 = value,
            5 => self.r5 = value,
            6 => self.r6 = value,
            7 => self.r7 = value,
            8 => self.pc = value,
            9 => self.cond = value,
            _ => panic!("Index out of bound"),
        }
    }

    pub fn get(&self, index: u16) -> u16 {
        match index {
            0 => self.r0,
            1 => self.r1,
            2 => self.r2,
            3 => self.r3,
            4 => self.r4,
            5 => self.r5,
            6 => self.r6,
            7 => self.r7,
            8 => self.pc,
            9 => self.cond,
            _ => panic!("Index out of bound. "),
        }
    }

    // Update the condition register based on the value that's inside the register `r`.
    pub fn update_r_cond_register(&mut self, r: u16) {
        if self.get(r) == 0 {
            self.update(9, ConditionFlag::ZRO as u16);
        } else if (self.get(r) >> 15) != 0 {
            // a 1 in the left-most bit indicates negative
            self.update(9, ConditionFlag::NEG as u16);
        } else {
            self.update(9, ConditionFlag::POS as u16);
        }
    }
}