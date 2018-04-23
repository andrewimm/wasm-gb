use vm::memmap::MemMap;

pub struct CPU {
  a: u8,
  b: u8,
  c: u8,
  d: u8,
  e: u8,
  h: u8,
  l: u8,
  flags: u8,

  sp: u16,
  pc: u16,

  ime: bool,
}

// flag offsets
const FLAG_Z: u8 = 7;
const FLAG_N: u8 = 6;
const FLAG_H: u8 = 5;
const FLAG_C: u8 = 4;

const FLAG_VAL_Z: u8 = 1 << FLAG_Z;
const FLAG_VAL_N: u8 = 1 << FLAG_N;
const FLAG_VAL_H: u8 = 1 << FLAG_H;
const FLAG_VAL_C: u8 = 1 << FLAG_C;

#[derive(Debug, Copy, Clone)]
pub enum Register8 {
  A,
  B,
  C,
  D,
  E,
  H,
  L,
  Flags,
}

#[derive(Debug, Copy, Clone)]
pub enum Register16 {
  AF,
  BC,
  DE,
  HL,
  SP,
  PC,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RunState {
  Run,
  Crash,
  Halt,
  Stop,
}

pub fn create_cpu() -> CPU {
  return CPU {
    a: 0,
    b: 0,
    c: 0,
    d: 0,
    e: 0,
    h: 0,
    l: 0,
    flags: 0,

    sp: 0,
    pc: 0,

    ime: false,
  };
}

/*
fn u16_of_bytes(h: u8, l: u8) -> u16 {
  return ((h as u16) << 8) + (l as u16);
}
*/

impl CPU {
  pub fn reset(&mut self) {
    self.a = 0;
    self.b = 0;
    self.c = 0;
    self.d = 0;
    self.e = 0;
    self.h = 0;
    self.l = 0;
    self.flags = 0;

    self.sp = 0;
    self.pc = 0;
  }

  pub fn simulate_bootloader(&mut self) {
    self.a = 0x01;
    self.b = 0;
    self.c = 0x13;
    self.d = 0;
    self.e = 0xd8;
    self.h = 0x01;
    self.l = 0x4d;
    self.flags = 0xb0;
    self.sp = 0xfffe;
    self.pc = 0x100;
  }

  pub fn get_register_8(&self, reg: Register8) -> u8 {
    return match reg {
      Register8::A => self.a,
      Register8::B => self.b,
      Register8::C => self.c,
      Register8::D => self.d,
      Register8::E => self.e,
      Register8::H => self.h,
      Register8::L => self.l,
      Register8::Flags => self.flags,
    };
  }

  pub fn set_register_8(&mut self, reg: Register8, value: u8) {
    match reg {
      Register8::A => self.a = value,
      Register8::B => self.b = value,
      Register8::C => self.c = value,
      Register8::D => self.d = value,
      Register8::E => self.e = value,
      Register8::H => self.h = value,
      Register8::L => self.l = value,
      Register8::Flags => self.flags = value,
    };
  }

  pub fn get_register_16(&self, reg: Register16) -> u16 {
    return match reg {
      Register16::AF => ((self.a as u16) << 8) + (self.flags as u16),
      Register16::BC => ((self.b as u16) << 8) + (self.c as u16),
      Register16::DE => ((self.d as u16) << 8) + (self.e as u16),
      Register16::HL => ((self.h as u16) << 8) + (self.l as u16),
      Register16::SP => self.sp,
      Register16::PC => self.pc,
    };
  }

  pub fn set_register_16(&mut self, reg: Register16, value: u16) {
    match reg {
      Register16::AF => {
        self.a = (value >> 8) as u8;
        self.flags = (value & 0xf0) as u8;
      },
      Register16::BC => {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xff) as u8;
      },
      Register16::DE => {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xff) as u8;
      },
      Register16::HL => {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xff) as u8;
      },
      Register16::SP => self.sp = value,
      Register16::PC => self.pc = value,
    };
  }

  pub fn set_flag(&mut self, flag: u8) {
    self.flags = self.flags | (1 << flag);
  }

  pub fn clear_flag(&mut self, flag: u8) {
    self.flags = self.flags & !(1 << flag);
  }

  fn flag_test(&mut self, val: u8, sub: bool, carry: bool) {
    self.flags = if val == 0 {
      FLAG_VAL_Z
    } else {
      0
    };
    if sub {
      self.flags |= FLAG_VAL_N;
    }
    if carry {
      self.flags |= FLAG_VAL_C;
    }
  }

  fn half_carry_test(&mut self, a: u8, b: u8) {
    if (a & 0xf) + (b & 0xf) > 0xf {
      self.flags |= FLAG_VAL_H;
    } else {
      self.flags &= !FLAG_VAL_H;
    }
  }

  fn half_carry_sub_test(&mut self, a: u8, b: u8) {
    if (a & 0xf).wrapping_sub(b & 0xf) > 0x7f {
      self.flags |= FLAG_VAL_H;
    } else {
      self.flags &= !FLAG_VAL_H;
    }
  }

  fn flag_test_zero(&mut self, result: u8) {
    if result == 0 {
      self.set_flag(FLAG_Z);
    } else {
      self.clear_flag(FLAG_Z);
    }
  }

  fn flag_test_compare(&mut self, a: u8, b: u8) {
    if a == b {
      self.set_flag(FLAG_Z);
    } else {
      self.clear_flag(FLAG_Z);
    }
    self.set_flag(FLAG_N);
    if (a & 0xf) < (b & 0xf) {
      self.set_flag(FLAG_H);
    } else {
      self.clear_flag(FLAG_H);
    }
    if a < b {
      self.set_flag(FLAG_C);
    } else {
      self.clear_flag(FLAG_C);
    }
  }

  pub fn push(&mut self, mem: &mut MemMap, val: u16) {
    let addr = self.get_register_16(Register16::SP) - 2;
    mem.set_word(addr, val);
    self.set_register_16(Register16::SP, addr);
  }

  pub fn pop(&mut self, mem: &mut MemMap) -> u16 {
    let addr = self.get_register_16(Register16::SP);
    let value = mem.get_word(addr);
    self.set_register_16(Register16::SP, addr + 2);
    return value;
  }

  pub fn rotate_register_left(&mut self, reg: Register8, use_carry: bool, clear_zero: bool) {
    let orig = self.get_register_8(reg);
    let carry_in = if use_carry {
      if 0x80 & orig > 0 {
        1
      } else {
        0
      }
    } else {
      if self.flags & 0x10 > 0 {
        1
      } else {
        0
      }
    };
    let carry_out = orig & 0x80 > 0;
    let value = (orig << 1).wrapping_add(carry_in);
    self.set_register_8(reg, value);
    if clear_zero {
      self.flag_test(1, false, carry_out);
    } else {
      self.flag_test(value, false, carry_out);
    }
    self.clear_flag(FLAG_H);
  }

  pub fn rotate_register_right(&mut self, reg: Register8, use_carry: bool, clear_zero: bool) {
    let orig = self.get_register_8(reg);
    let carry_in = if use_carry {
      if 1 & orig > 0 {
        0x80
      } else {
        0
      }
    } else {
      if self.flags & 0x10 > 0 {
        0x80
      } else {
        0
      }
    };
    let carry_out = orig & 1 > 0;
    let value = (orig >> 1).wrapping_add(carry_in);
    self.set_register_8(reg, value);
    if clear_zero {
      self.flag_test(1, false, carry_out);
    } else {
      self.flag_test(value, false, carry_out);
    }
    self.clear_flag(FLAG_H);
  }

  pub fn shift_register_right(&mut self, reg: Register8, use_carry: bool) {
    let orig = self.get_register_8(reg);
    let carry_in = if use_carry {
      0x80 & orig
    } else {
      0
    };
    let carry_out = orig & 1 > 0;
    let value = (orig >> 1).wrapping_add(carry_in);
    self.set_register_8(reg, value);
    self.flag_test(value, false, false);
    if carry_out {
      self.set_flag(FLAG_C);
    }
  }

  pub fn shift_register_left(&mut self, reg: Register8) {
    let orig = self.get_register_8(reg);
    let carry_out = orig & 0x80 > 0;
    let value = orig << 1;
    self.set_register_8(reg, value);
    self.flag_test(value, false, false);
    if carry_out {
      self.set_flag(FLAG_C);
    }
  }

  pub fn inc_8(&mut self, reg: Register8) {
    let orig = self.get_register_8(reg);
    let value = if orig == 0xff {
      0
    } else {
      orig + 1
    };
    self.flag_test_zero(value);
    self.clear_flag(FLAG_N);
    self.half_carry_test(orig, 1);
    self.set_register_8(reg, value);
  }

  pub fn inc_16(&mut self, reg: Register16) {
    let orig = self.get_register_16(reg);
    let value = if orig == 0xffff {
      0
    } else {
      orig + 1
    };
    self.set_register_16(reg, value);
  }

  pub fn dec_8(&mut self, reg: Register8) {
    let orig = self.get_register_8(reg);
    let value = if orig == 0 {
      0xff
    } else {
      orig - 1
    };
    self.flag_test_zero(value);
    self.set_flag(FLAG_N);
    self.half_carry_sub_test(orig, 1);
    self.set_register_8(reg, value);
  }

  pub fn dec_16(&mut self, reg: Register16) {
    let orig = self.get_register_16(reg);
    let value = if orig == 0 {
      0xffff
    } else {
      orig - 1
    };
    self.set_register_16(reg, value);
  }

  pub fn swap(&mut self, reg: Register8) {
    let orig = self.get_register_8(reg);
    let low = orig & 0xf;
    let high = (orig & 0xf0) >> 4;
    let value = (low << 4) | high;
    self.set_register_8(reg, value);
    self.flag_test_zero(value);
    self.clear_flag(FLAG_N);
    self.clear_flag(FLAG_H);
    self.clear_flag(FLAG_C);
  }

  pub fn enable_interrupts(&mut self) {
    self.ime = true;
  }

  pub fn disable_interrupts(&mut self) {
    self.ime = false;
  }

  pub fn interrupt_enabled(&mut self) -> bool {
    self.ime
  }

  pub fn int_vblank(&mut self, mem: &mut MemMap) {
    self.disable_interrupts();
    let pc = self.pc;
    self.push(mem, pc);
    self.pc = 0x40;
  }

  pub fn int_stat(&mut self, mem: &mut MemMap) {
    self.disable_interrupts();
    let pc = self.pc;
    self.push(mem, pc);
    self.pc = 0x48;
  }

  pub fn int_timer(&mut self, mem: &mut MemMap) {
    self.disable_interrupts();
    let pc = self.pc;
    self.push(mem, pc);
    self.pc = 0x50;
  }

  pub fn int_joypad(&mut self, mem: &mut MemMap) {
    self.disable_interrupts();
    let pc = self.pc;
    self.push(mem, pc);
    self.pc = 0x60;
  }

  pub fn step(&mut self, mem: &mut MemMap) -> (RunState, u8) {
    let mut state = RunState::Run;
    let index = self.pc;
    let (byte_len, cycles) = match mem.get_byte(index) {
      0x00 => (1, 4), // NOP
      0x01 => { // LD BC,nn
          let value = mem.get_word(index + 1);
          self.set_register_16(Register16::BC, value);
          (3, 12)
      },
      0x02 => { // LD (BC),A
          let value = self.get_register_8(Register8::A);
          let addr = self.get_register_16(Register16::BC);
          mem.set_byte(addr, value);
          (1, 8)
      },
      0x03 => { // INC BC
          self.inc_16(Register16::BC);
          (1, 8)
      },
      0x04 => { // INC B
          self.inc_8(Register8::B);
          (1, 4)
      },
      0x05 => { // DEC B
          self.dec_8(Register8::B);
          (1, 4)
      },
      0x06 => { // LD B,n
          let value = mem.get_byte(index + 1);
          self.set_register_8(Register8::B, value);
          (2, 8)
      },
      0x07 => { // RLCA
          self.rotate_register_left(Register8::A, true, true);
          (1, 4)
      },
      0x08 => { // LD (nn),SP
          let value = self.get_register_16(Register16::SP);
          let addr = mem.get_word(index + 1);
          mem.set_word(addr, value);
          (3, 20)
      },
      0x09 => { // ADD HL,BC
          let bc = self.get_register_16(Register16::BC);
          let hl = self.get_register_16(Register16::HL);
          let (value, overflow) = bc.overflowing_add(hl);
          self.set_register_16(Register16::HL, value);
          self.clear_flag(FLAG_N);
          if overflow {
            self.set_flag(FLAG_C);
          } else {
            self.clear_flag(FLAG_C);
          }
          if (bc & 0xfff) + (hl & 0xfff) > 0xfff {
            self.set_flag(FLAG_H);
          } else {
            self.clear_flag(FLAG_H);
          }
          (1, 8)
      },
      0x0a => { // LD A,(BC)
          let value = mem.get_byte(self.get_register_16(Register16::BC));
          self.set_register_8(Register8::A, value);
          (1, 8)
      },
      0x0b => { // DEC BC
          self.dec_16(Register16::BC);
          (1, 8)
      },
      0x0c => { // INC C
          self.inc_8(Register8::C);
          (1, 4)
      },
      0x0d => { // DEC C
          self.dec_8(Register8::C);
          (1, 4)
      },
      0x0e => { // LD C,n
          let value = mem.get_byte(index + 1);
          self.set_register_8(Register8::C, value);
          (2, 8)
      },
      0x0f => { // RRCA
          self.rotate_register_right(Register8::A, true, true);
          (1, 4)
      },
      0x10 => { // STOP
          state = RunState::Stop;
          (1, 4)
      },
      0x11 => { // LD DE,nn
          let value = mem.get_word(index + 1);
          self.set_register_16(Register16::DE, value);
          (3, 12)
      },
      0x12 => { // LD (DE),A
          let value = self.get_register_8(Register8::A);
          let addr = self.get_register_16(Register16::DE);
          mem.set_byte(addr, value);
          (1, 8)
      },
      0x13 => { // INC DE
          self.inc_16(Register16::DE);
          (1, 8)
      },
      0x14 => { // INC D
          self.inc_8(Register8::D);
          (1, 4)
      },
      0x15 => { // DEC D
          self.dec_8(Register8::D);
          (1, 4)
      },
      0x16 => { // LD D,n
          let value = mem.get_byte(index + 1);
          self.set_register_8(Register8::D, value);
          (2, 8)
      },
      0x17 => { // RLA
          self.rotate_register_left(Register8::A, false, true);
          (1, 4)
      },
      0x18 => { // JR n
          let offset = mem.get_byte(index + 1);
          if offset & 0b10000000 > 0 {
            self.pc -= (!offset + 1) as u16;
          } else {
            self.pc += offset as u16;
          }
          (2, 8)
      },
      0x19 => { // ADD HL,DE
          let de = self.get_register_16(Register16::DE);
          let hl = self.get_register_16(Register16::HL);
          let (value, overflow) = de.overflowing_add(hl);
          self.set_register_16(Register16::HL, value);
          self.clear_flag(FLAG_N);
          if overflow {
            self.set_flag(FLAG_C);
          } else {
            self.clear_flag(FLAG_C);
          }
          if (de & 0xfff) + (hl & 0xfff) > 0xfff {
            self.set_flag(FLAG_H);
          } else {
            self.clear_flag(FLAG_H);
          }
          (1, 8)
      },
      0x1a => { // LD A,(DE)
          let value = mem.get_byte(self.get_register_16(Register16::DE));
          self.set_register_8(Register8::A, value);
          (1, 8)
      },
      0x1b => { // DEC DE
          self.dec_16(Register16::DE);
          (1, 8)
      },
      0x1c => { // INC E
          self.inc_8(Register8::E);
          (1, 4)
      },
      0x1d => { // DEC E
          self.dec_8(Register8::E);
          (1, 4)
      },
      0x1e => { // LD E,n
          let value = mem.get_byte(index + 1);
          self.set_register_8(Register8::E, value);
          (2, 8)
      },
      0x1f => { // RRA
          self.rotate_register_right(Register8::A, false, true);
          (1, 4)
      },
      0x20 => { // JR NZ,n
          let offset = mem.get_byte(index + 1);
          if self.flags & (1 << FLAG_Z) == 0 {
            // Zero flag is not set
            if offset & 0b10000000 > 0 {
              self.pc -= (!offset + 1) as u16;
            } else {
              self.pc += offset as u16;
            }
          }
          (2, 8)
      },
      0x21 => { // LD HL,nn
        let value = mem.get_word(index + 1);
        self.set_register_16(Register16::HL, value);
        (3, 12)
      },
      0x22 => { // LDI (HL),A
        let value = self.get_register_8(Register8::A);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        self.set_register_16(Register16::HL, addr.wrapping_add(1));
        (1, 8)
      },
      0x23 => { // INC HL
        self.inc_16(Register16::HL);
        (1, 8)
      },
      0x24 => { // INC H
        self.inc_8(Register8::H);
        (1, 4)
      },
      0x25 => { // DEC H
        self.dec_8(Register8::H);
        (1, 4)
      },
      0x26 => { // LD H,n
        let value = mem.get_byte(index + 1);
        self.set_register_8(Register8::H, value);
        (2, 8)
      },
      0x27 => { // DAA
        let orig = self.get_register_8(Register8::A);
        let mut value = orig;
        let mut carry = false;
        if self.flags & FLAG_VAL_N == 0 {
          let low = orig & 0xf;
          if low > 9 || (self.flags & FLAG_VAL_H > 0) {
            let (v, c) = value.overflowing_add(6);
            value = v;
            carry = carry || c;
          }
          if value > 0x9f || (self.flags & FLAG_VAL_C > 0) {
            let (v, c) = value.overflowing_add(0x60);
            value = v;
            carry = carry || c;
          }
        } else {
          if self.flags & FLAG_VAL_H > 0 {
            let (v, _c) = value.overflowing_sub(6);
            value = v;
          }
          if self.flags & FLAG_VAL_C > 0 {
            let (v, c) = value.overflowing_sub(0x60);
            value = v;
            carry = carry || c;
          }
        }
        self.set_register_8(Register8::A, value);
        self.clear_flag(FLAG_H);
        if value == 0 {
          self.set_flag(FLAG_Z);
        } else {
          self.clear_flag(FLAG_Z);
        }
        if carry {
          self.set_flag(FLAG_C);
        } else {
          self.clear_flag(FLAG_C);
        }
        (1, 4)
      },
      0x28 => { // JR Z,n
        let offset = mem.get_byte(index + 1);
        if self.flags & (1 << FLAG_Z) != 0 {
          // Zero flag is set
          if offset & 0b10000000 > 0 {
            self.pc -= (!offset + 1) as u16;
          } else {
            self.pc += offset as u16;
          }
        }
        (2, 8)
      },
      0x29 => { // ADD HL,HL
        let hl = self.get_register_16(Register16::HL);
        let (value, overflow) = hl.overflowing_add(hl);
        self.set_register_16(Register16::HL, value);
        self.clear_flag(FLAG_N);
        if overflow {
          self.set_flag(FLAG_C);
        } else {
          self.clear_flag(FLAG_C);
        }
        if (hl & 0xfff) + (hl & 0xfff) > 0xfff {
          self.set_flag(FLAG_H);
        } else {
          self.clear_flag(FLAG_H);
        }
        (1, 8)
      },
      0x2a => { // LDI A,(HL)
        let addr = self.get_register_16(Register16::HL);
        let value = mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        self.set_register_16(Register16::HL, addr.wrapping_add(1));
        (1, 8)
      },
      0x2b => { // DEC HL
        self.dec_16(Register16::HL);
        (1, 8)
      },
      0x2c => { // INC L
        self.inc_8(Register8::L);
        (1, 4)
      },
      0x2d => { // DEC L
        self.dec_8(Register8::L);
        (1, 4)
      },
      0x2e => { // LD L,n
        let value = mem.get_byte(index + 1);
        self.set_register_8(Register8::L, value);
        (2, 8)
      },
      0x2f => { // CPL A
        let value = !self.get_register_8(Register8::A);
        self.set_register_8(Register8::A, value);
        self.set_flag(FLAG_N);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0x30 => { // JR NC,n
        let offset = mem.get_byte(index + 1);
        if self.flags & (1 << FLAG_C) == 0 {
          // Carry flag is not set
          if offset & 0b10000000 > 0 {
            self.pc -= (!offset + 1) as u16;
          } else {
            self.pc += offset as u16;
          }
        }
        (2, 8)
      },
      0x31 => { // LD SP,nn
        let value = mem.get_word(index + 1);
        self.set_register_16(Register16::SP, value);
        (3, 12)
      },
      0x32 => { // LDD (HL),A
        let value = self.get_register_8(Register8::A);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        self.set_register_16(Register16::HL, addr.wrapping_sub(1));
        (1, 8)
      },
      0x33 => { // INC SP
        self.inc_16(Register16::SP);
        (1, 8)
      },
      0x34 => { // INC (HL)
        let addr = self.get_register_16(Register16::HL);
        let orig = mem.get_byte(addr);
        let value = orig.wrapping_add(1);
        mem.set_byte(addr, value);

        self.flag_test_zero(value);
        self.clear_flag(FLAG_N);
        self.half_carry_test(orig, 1);
        (1, 12)
      },
      0x35 => { // DEC (HL)
        let addr = self.get_register_16(Register16::HL);
        let orig = mem.get_byte(addr);
        let value = orig.wrapping_sub(1);
        mem.set_byte(addr, value);

        self.flag_test_zero(value);
        self.set_flag(FLAG_N);
        self.half_carry_sub_test(orig, 1);
        (1, 12)
      },
      0x36 => { // LD (HL),n
        let addr = self.get_register_16(Register16::HL);
        let value = mem.get_byte(index + 1);
        mem.set_byte(addr, value);
        (2, 12)
      },
      0x37 => { // SCF
        self.clear_flag(FLAG_H);
        self.clear_flag(FLAG_N);
        self.set_flag(FLAG_C);
        (1, 4)
      },
      0x38 => { // JR C,n
        let offset = mem.get_byte(index + 1);
        if self.flags & (1 << FLAG_C) != 0 {
          // Carry flag is set
          if offset & 0b10000000 != 0 {
            self.pc -= (!offset + 1) as u16;
          } else {
            self.pc += offset as u16;
          }
        }
        (2, 8)
      },
      0x39 => { // ADD HL,SP
        let orig = self.get_register_16(Register16::HL);
        let sp = self.get_register_16(Register16::SP);
        let (value, overflow) = orig.overflowing_add(sp);
        self.set_register_16(Register16::HL, value);
        self.clear_flag(FLAG_N);
        if overflow {
          self.set_flag(FLAG_C);
        } else {
          self.clear_flag(FLAG_C);
        }
        if (orig & 0xfff) + (sp & 0xfff) > 0xfff {
          self.set_flag(FLAG_H);
        } else {
          self.clear_flag(FLAG_H);
        }
        (1, 8)
      },
      0x3a => { // LDD A,(HL)
        let addr = self.get_register_16(Register16::HL);
        let value = mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        self.set_register_16(Register16::HL, addr.wrapping_sub(1));
        (1, 8)
      },
      0x3b => { // DEC SP
        self.dec_16(Register16::SP);
        (1, 8)
      },
      0x3c => { // INC A
        self.inc_8(Register8::A);
        (1, 4)
      },
      0x3d => { // DEC A
        self.dec_8(Register8::A);
        (1, 4)
      },
      0x3e => { // LD A,n
        let value = mem.get_byte(index + 1);
        self.set_register_8(Register8::A, value);
        (2, 8)
      },
      0x3f => { // CCF
        self.clear_flag(FLAG_H);
        self.clear_flag(FLAG_N);
        if self.flags & FLAG_VAL_C == 0 {
          self.set_flag(FLAG_C);
        } else {
          self.clear_flag(FLAG_C);
        }
        (1, 4)
      },
      0x40 => { // LD B,B
        let value = self.get_register_8(Register8::B);
        self.set_register_8(Register8::B, value);
        (1, 4)
      },
      0x41 => { // LD B,C
        let value = self.get_register_8(Register8::C);
        self.set_register_8(Register8::B, value);
        (1, 4)
      },
      0x42 => { // LD B,D
        let value = self.get_register_8(Register8::D);
        self.set_register_8(Register8::B, value);
        (1, 4)
      },
      0x43 => { // LD B,E
        let value = self.get_register_8(Register8::E);
        self.set_register_8(Register8::B, value);
        (1, 4)
      },
      0x44 => { // LD B,H
        let value = self.get_register_8(Register8::H);
        self.set_register_8(Register8::B, value);
        (1, 4)
      },
      0x45 => { // LD B,L
        let value = self.get_register_8(Register8::L);
        self.set_register_8(Register8::B, value);
        (1, 4)
      },
      0x46 => { // LD B,(HL)
        let value = mem.get_byte(self.get_register_16(Register16::HL));
        self.set_register_8(Register8::B, value);
        (1, 8)
      },
      0x47 => { // LD B,A
        let value = self.get_register_8(Register8::A);
        self.set_register_8(Register8::B, value);
        (1, 4)
      },
      0x48 => { // LD C,B
        let value = self.get_register_8(Register8::B);
        self.set_register_8(Register8::C, value);
        (1, 4)
      },
      0x49 => { // LD C,C
        let value = self.get_register_8(Register8::C);
        self.set_register_8(Register8::C, value);
        (1, 4)
      },
      0x4a => { // LD C,D
        let value = self.get_register_8(Register8::D);
        self.set_register_8(Register8::C, value);
        (1, 4)
      },
      0x4b => { // LD C,E
        let value = self.get_register_8(Register8::E);
        self.set_register_8(Register8::C, value);
        (1, 4)
      },
      0x4c => { // LD C,H
        let value = self.get_register_8(Register8::H);
        self.set_register_8(Register8::C, value);
        (1, 4)
      },
      0x4d => { // LD C,L
        let value = self.get_register_8(Register8::L);
        self.set_register_8(Register8::C, value);
        (1, 4)
      },
      0x4e => { // LD C,(HL)
        let value = mem.get_byte(self.get_register_16(Register16::HL));
        self.set_register_8(Register8::C, value);
        (1, 8)
      },
      0x4f => { // LD C,A
        let value = self.get_register_8(Register8::A);
        self.set_register_8(Register8::C, value);
        (1, 4)
      },
      0x50 => { // LD D,B
        let value = self.get_register_8(Register8::B);
        self.set_register_8(Register8::D, value);
        (1, 4)
      },
      0x51 => { // LD D,C
        let value = self.get_register_8(Register8::C);
        self.set_register_8(Register8::D, value);
        (1, 4)
      },
      0x52 => { // LD D,D
        let value = self.get_register_8(Register8::D);
        self.set_register_8(Register8::D, value);
        (1, 4)
      },
      0x53 => { // LD D,E
        let value = self.get_register_8(Register8::E);
        self.set_register_8(Register8::D, value);
        (1, 4)
      },
      0x54 => { // LD D,H
        let value = self.get_register_8(Register8::H);
        self.set_register_8(Register8::D, value);
        (1, 4)
      },
      0x55 => { // LD D,L
        let value = self.get_register_8(Register8::L);
        self.set_register_8(Register8::D, value);
        (1, 4)
      },
      0x56 => { // LD D,(HL)
        let value = mem.get_byte(self.get_register_16(Register16::HL));
        self.set_register_8(Register8::D, value);
        (1, 8)
      },
      0x57 => { // LD D,A
        let value = self.get_register_8(Register8::A);
        self.set_register_8(Register8::D, value);
        (1, 4)
      },
      0x58 => { // LD E,B
        let value = self.get_register_8(Register8::B);
        self.set_register_8(Register8::E, value);
        (1, 4)
      },
      0x59 => { // LD E,C
        let value = self.get_register_8(Register8::C);
        self.set_register_8(Register8::E, value);
        (1, 4)
      },
      0x5a => { // LD E,D
        let value = self.get_register_8(Register8::D);
        self.set_register_8(Register8::E, value);
        (1, 4)
      },
      0x5b => { // LD E,E
        let value = self.get_register_8(Register8::E);
        self.set_register_8(Register8::E, value);
        (1, 4)
      },
      0x5c => { // LD E,H
        let value = self.get_register_8(Register8::H);
        self.set_register_8(Register8::E, value);
        (1, 4)
      },
      0x5d => { // LD E,L
        let value = self.get_register_8(Register8::L);
        self.set_register_8(Register8::E, value);
        (1, 4)
      },
      0x5e => { // LD E,(HL)
        let value = mem.get_byte(self.get_register_16(Register16::HL));
        self.set_register_8(Register8::E, value);
        (1, 8)
      },
      0x5f => { // LD E,A
        let value = self.get_register_8(Register8::A);
        self.set_register_8(Register8::E, value);
        (1, 4)
      },
      0x60 => { // LD H,B
        let value = self.get_register_8(Register8::B);
        self.set_register_8(Register8::H, value);
        (1, 4)
      },
      0x61 => { // LD H,C
        let value = self.get_register_8(Register8::C);
        self.set_register_8(Register8::H, value);
        (1, 4)
      },
      0x62 => { // LD H,D
        let value = self.get_register_8(Register8::D);
        self.set_register_8(Register8::H, value);
        (1, 4)
      },
      0x63 => { // LD H,E
        let value = self.get_register_8(Register8::E);
        self.set_register_8(Register8::H, value);
        (1, 4)
      },
      0x64 => { // LD H,H
        let value = self.get_register_8(Register8::H);
        self.set_register_8(Register8::H, value);
        (1, 4)
      },
      0x65 => { // LD H,L
        let value = self.get_register_8(Register8::L);
        self.set_register_8(Register8::H, value);
        (1, 4)
      },
      0x66 => { // LD H,(HL)
        let value = mem.get_byte(self.get_register_16(Register16::HL));
        self.set_register_8(Register8::H, value);
        (1, 8)
      },
      0x67 => { // LD H,A
        let value = self.get_register_8(Register8::A);
        self.set_register_8(Register8::H, value);
        (1, 4)
      },
      0x68 => { // LD L,B
        let value = self.get_register_8(Register8::B);
        self.set_register_8(Register8::L, value);
        (1, 4)
      },
      0x69 => { // LD L,C
        let value = self.get_register_8(Register8::C);
        self.set_register_8(Register8::L, value);
        (1, 4)
      },
      0x6a => { // LD L,D
        let value = self.get_register_8(Register8::D);
        self.set_register_8(Register8::L, value);
        (1, 4)
      },
      0x6b => { // LD L,E
        let value = self.get_register_8(Register8::E);
        self.set_register_8(Register8::L, value);
        (1, 4)
      },
      0x6c => { // LD L,H
        let value = self.get_register_8(Register8::H);
        self.set_register_8(Register8::L, value);
        (1, 4)
      },
      0x6d => { // LD L,L
        let value = self.get_register_8(Register8::L);
        self.set_register_8(Register8::L, value);
        (1, 4)
      },
      0x6e => { // LD L,(HL)
        let value = mem.get_byte(self.get_register_16(Register16::HL));
        self.set_register_8(Register8::L, value);
        (1, 8)
      },
      0x6f => { // LD L,A
        let value = self.get_register_8(Register8::A);
        self.set_register_8(Register8::L, value);
        (1, 4)
      },
      0x70 => { // LD (HL),B
        let value = self.get_register_8(Register8::B);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0x71 => { // LD (HL),C
        let value = self.get_register_8(Register8::C);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0x72 => { // LD (HL),D
        let value = self.get_register_8(Register8::D);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0x73 => { // LD (HL),E
        let value = self.get_register_8(Register8::E);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0x74 => { // LD (HL),H
        let value = self.get_register_8(Register8::H);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0x75 => { // LD (HL),L
        let value = self.get_register_8(Register8::L);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0x76 => { // HALT
        state = RunState::Halt;
        (1, 4)
      },
      0x77 => { // LD (HL),A
        let value = self.get_register_8(Register8::A);
        let addr = self.get_register_16(Register16::HL);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0x78 => { // LD A,B
        let value = self.get_register_8(Register8::B);
        self.set_register_8(Register8::A, value);
        (1, 4)
      },
      0x79 => { // LD A,C
        let value = self.get_register_8(Register8::C);
        self.set_register_8(Register8::A, value);
        (1, 4)
      },
      0x7a => { // LD A,D
        let value = self.get_register_8(Register8::D);
        self.set_register_8(Register8::A, value);
        (1, 4)
      },
      0x7b => { // LD A,E
        let value = self.get_register_8(Register8::E);
        self.set_register_8(Register8::A, value);
        (1, 4)
      },
      0x7c => { // LD A,H
        let value = self.get_register_8(Register8::H);
        self.set_register_8(Register8::A, value);
        (1, 4)
      },
      0x7d => { // LD A,L
        let value = self.get_register_8(Register8::L);
        self.set_register_8(Register8::A, value);
        (1, 4)
      },
      0x7e => { // LD A,(HL)
        let value = mem.get_byte(self.get_register_16(Register16::HL));
        self.set_register_8(Register8::A, value);
        (1, 8)
      },
      0x7f => { // LD A,A
        let value = self.get_register_8(Register8::A);
        self.set_register_8(Register8::A, value);
        (1, 4)
      },
      0x80 => { // ADD A,B
        let a = self.get_register_8(Register8::A);
        let b = self.get_register_8(Register8::B);
        let (value, overflow) = a.overflowing_add(b);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, b);
        (1, 4)
      },
      0x81 => { // ADD A,C
        let a = self.get_register_8(Register8::A);
        let c = self.get_register_8(Register8::C);
        let (value, overflow) = a.overflowing_add(c);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, c);
        (1, 4)
      },
      0x82 => { // ADD A,D
        let a = self.get_register_8(Register8::A);
        let d = self.get_register_8(Register8::D);
        let (value, overflow) = a.overflowing_add(d);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, d);
        (1, 4)
      },
      0x83 => { // ADD A,E
        let a = self.get_register_8(Register8::A);
        let e = self.get_register_8(Register8::E);
        let (value, overflow) = a.overflowing_add(e);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, e);
        (1, 4)
      },
      0x84 => { // ADD A,H
        let a = self.get_register_8(Register8::A);
        let h = self.get_register_8(Register8::H);
        let (value, overflow) = a.overflowing_add(h);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, h);
        (1, 4)
      },
      0x85 => { // ADD A,L
        let a = self.get_register_8(Register8::A);
        let l = self.get_register_8(Register8::L);
        let (value, overflow) = a.overflowing_add(l);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, l);
        (1, 4)
      },
      0x86 => { // ADD A,(HL)
        let addr = self.get_register_16(Register16::HL);
        let a = self.get_register_8(Register8::A);
        let n = mem.get_byte(addr);
        let (value, overflow) = a.overflowing_add(n);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, n);
        (1, 8)
      },
      0x87 => { // ADD A,A
        let a = self.get_register_8(Register8::A);
        let (value, overflow) = a.overflowing_add(a);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        self.half_carry_test(a, a);
        (1, 4)
      },
      0x88 => { // ADC A,B
        let a = self.get_register_8(Register8::A);
        let b = self.get_register_8(Register8::B);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(b);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (b & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x89 => { // ADC A,C
        let a = self.get_register_8(Register8::A);
        let c = self.get_register_8(Register8::C);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(c);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (c & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x8a => { // ADC A,D
        let a = self.get_register_8(Register8::A);
        let d = self.get_register_8(Register8::D);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(d);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (d & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x8b => { // ADC A,E
        let a = self.get_register_8(Register8::A);
        let e = self.get_register_8(Register8::E);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(e);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (e & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x8c => { // ADC A,H
        let a = self.get_register_8(Register8::A);
        let h = self.get_register_8(Register8::H);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(h);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (h & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x8d => { // ADC A,L
        let a = self.get_register_8(Register8::A);
        let l = self.get_register_8(Register8::L);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(l);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (l & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x8e => { // ADC A,(HL)
        let a = self.get_register_8(Register8::A);
        let addr = self.get_register_16(Register16::HL);
        let orig = mem.get_byte(addr);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(orig);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (orig & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 8)
      },
      0x8f => { // ADC A,A
        let a = self.get_register_8(Register8::A);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_add(a);
        let (carry_value, carry_overflow) = value.overflowing_add(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        if (a & 0xf) + (a & 0xf) + carry > 0xf {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x90 => { // SUB A,B
        let a = self.get_register_8(Register8::A);
        let b = self.get_register_8(Register8::B);
        let (value, overflow) = a.overflowing_sub(b);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, b);
        (1, 4)
      },
      0x91 => { // SUB A,C
        let a = self.get_register_8(Register8::A);
        let c = self.get_register_8(Register8::C);
        let (value, overflow) = a.overflowing_sub(c);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, c);
        (1, 4)
      },
      0x92 => { // SUB A,D
        let a = self.get_register_8(Register8::A);
        let d = self.get_register_8(Register8::D);
        let (value, overflow) = a.overflowing_sub(d);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, d);
        (1, 4)
      },
      0x93 => { // SUB A,E
        let a = self.get_register_8(Register8::A);
        let e = self.get_register_8(Register8::E);
        let (value, overflow) = a.overflowing_sub(e);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, e);
        (1, 4)
      },
      0x94 => { // SUB A,H
        let a = self.get_register_8(Register8::A);
        let h = self.get_register_8(Register8::H);
        let (value, overflow) = a.overflowing_sub(h);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, h);
        (1, 4)
      },
      0x95 => { // SUB A,L
        let a = self.get_register_8(Register8::A);
        let l = self.get_register_8(Register8::L);
        let (value, overflow) = a.overflowing_sub(l);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, l);
        (1, 4)
      },
      0x96 => { // SUB A,(HL)
        let addr = self.get_register_16(Register16::HL);
        let a = self.get_register_8(Register8::A);
        let n = mem.get_byte(addr);
        let (value, overflow) = a.overflowing_sub(n);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, n);
        (1, 8)
      },
      0x97 => { // SUB A,A
        let a = self.get_register_8(Register8::A);
        let (value, overflow) = a.overflowing_sub(a);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        self.half_carry_sub_test(a, a);
        (1, 4)
      },
      0x98 => { // SBC A,B
        let a = self.get_register_8(Register8::A);
        let b = self.get_register_8(Register8::B);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(b);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(b & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x99 => { // SBC A,C
        let a = self.get_register_8(Register8::A);
        let c = self.get_register_8(Register8::C);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(c);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(c & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x9a => { // SBC A,D
        let a = self.get_register_8(Register8::A);
        let d = self.get_register_8(Register8::D);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(d);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(d & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x9b => { // SBC A,E
        let a = self.get_register_8(Register8::A);
        let e = self.get_register_8(Register8::E);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(e);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(e & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x9c => { // SBC A,H
        let a = self.get_register_8(Register8::A);
        let h = self.get_register_8(Register8::H);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(h);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(h & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x9d => { // SBC A,L
        let a = self.get_register_8(Register8::A);
        let l = self.get_register_8(Register8::L);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(l);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(l & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0x9e => { // SBC A,(HL)
        let a = self.get_register_8(Register8::A);
        let addr = self.get_register_16(Register16::HL);
        let n = mem.get_byte(addr);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(n);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(n & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 8)
      },
      0x9f => { // SBC A,A
        let a = self.get_register_8(Register8::A);
        let carry = if self.flags & FLAG_VAL_C != 0 { 1 } else { 0 };
        let (mut value, mut overflow) = a.overflowing_sub(a);
        let (carry_value, carry_overflow) = value.overflowing_sub(carry);
        value = carry_value;
        overflow = overflow || carry_overflow;
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        if (a & 0xf).wrapping_sub(a & 0xf).wrapping_sub(carry) > 0x7f {
          self.set_flag(FLAG_H);
        }
        (1, 4)
      },
      0xa0 => { // AND B
        let value = self.get_register_8(Register8::A) & self.get_register_8(Register8::B);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0xa1 => { // AND C
        let value = self.get_register_8(Register8::A) & self.get_register_8(Register8::C);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0xa2 => { // AND D
        let value = self.get_register_8(Register8::A) & self.get_register_8(Register8::D);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0xa3 => { // AND E
        let value = self.get_register_8(Register8::A) & self.get_register_8(Register8::E);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0xa4 => { // AND H
        let value = self.get_register_8(Register8::A) & self.get_register_8(Register8::H);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0xa5 => { // AND L
        let value = self.get_register_8(Register8::A) & self.get_register_8(Register8::L);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0xa6 => { // AND (HL)
        let addr = self.get_register_16(Register16::HL);
        let value = self.get_register_8(Register8::A) & mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 8)
      },
      0xa7 => { // AND A
        let value = self.get_register_8(Register8::A) & self.get_register_8(Register8::A);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        self.set_flag(FLAG_H);
        (1, 4)
      },
      0xa8 => { // XOR B
        let value = self.get_register_8(Register8::A) ^ self.get_register_8(Register8::B);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xa9 => { // XOR C
        let value = self.get_register_8(Register8::A) ^ self.get_register_8(Register8::C);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xaa => { // XOR D
        let value = self.get_register_8(Register8::A) ^ self.get_register_8(Register8::D);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xab => { // XOR E
        let value = self.get_register_8(Register8::A) ^ self.get_register_8(Register8::E);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xac => { // XOR H
        let value = self.get_register_8(Register8::A) ^ self.get_register_8(Register8::H);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xad => { // XOR L
        let value = self.get_register_8(Register8::A) ^ self.get_register_8(Register8::L);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xae => { // XOR (HL)
        let addr = self.get_register_16(Register16::HL);
        let value = self.get_register_8(Register8::A) ^ mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 8)
      },
      0xaf => { // XOR A
        let value = self.get_register_8(Register8::A) ^ self.get_register_8(Register8::A);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb0 => { // OR B
        let value = self.get_register_8(Register8::A) | self.get_register_8(Register8::B);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb1 => { // OR C
        let value = self.get_register_8(Register8::A) | self.get_register_8(Register8::C);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb2 => { // OR D
        let value = self.get_register_8(Register8::A) | self.get_register_8(Register8::D);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb3 => { // OR E
        let value = self.get_register_8(Register8::A) | self.get_register_8(Register8::E);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb4 => { // OR H
        let value = self.get_register_8(Register8::A) | self.get_register_8(Register8::H);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb5 => { // OR L
        let value = self.get_register_8(Register8::A) | self.get_register_8(Register8::L);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb6 => { // OR (HL)
        let addr = self.get_register_16(Register16::HL);
        let value = self.get_register_8(Register8::A) | mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 8)
      },
      0xb7 => { // OR A
        let value = self.get_register_8(Register8::A) | self.get_register_8(Register8::A);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (1, 4)
      },
      0xb8 => { // CP B
        let a = self.get_register_8(Register8::A);
        let b = self.get_register_8(Register8::B);
        self.flag_test_compare(a, b);
        (1, 4)
      },
      0xb9 => { // CP C
        let a = self.get_register_8(Register8::A);
        let c = self.get_register_8(Register8::C);
        self.flag_test_compare(a, c);
        (1, 4)
      },
      0xba => { // CP D
        let a = self.get_register_8(Register8::A);
        let d = self.get_register_8(Register8::D);
        self.flag_test_compare(a, d);
        (1, 4)
      },
      0xbb => { // CP E
        let a = self.get_register_8(Register8::A);
        let e = self.get_register_8(Register8::E);
        self.flag_test_compare(a, e);
        (1, 4)
      },
      0xbc => { // CP H
        let a = self.get_register_8(Register8::A);
        let h = self.get_register_8(Register8::H);
        self.flag_test_compare(a, h);
        (1, 4)
      },
      0xbd => { // CP L
        let a = self.get_register_8(Register8::A);
        let l = self.get_register_8(Register8::L);
        self.flag_test_compare(a, l);
        (1, 4)
      },
      0xbe => { // CP (HL)
        let a = self.get_register_8(Register8::A);
        let addr = self.get_register_16(Register16::HL);
        let cp = mem.get_byte(addr);
        self.flag_test_compare(a, cp);
        (1, 8)
      },
      0xbf => { // CP A
        self.flag_test(0, true, false);
        (1, 4)
      },
      0xc0 => { // RET NZ
        if self.flags & (1 << FLAG_Z) == 0 {
          // Zero flag is not set;
          let value = self.pop(mem);
          self.pc = value;
          (0, 8)
        } else {
          (1, 8)
        }
      },
      0xc1 => { // POP BC
        let value = self.pop(mem);
        self.set_register_16(Register16::BC, value);
        (1, 12)
      },
      0xc2 => { // JP NZ,nn
        if self.flags & (1 << FLAG_Z) == 0 {
          // Zero flag is not set
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xc3 => { // JP nn
        let dest = mem.get_word(index + 1);
        self.pc = dest;
        (0, 12)
      },
      0xc4 => { // CALL NZ,nn
        if self.flags & (1 << FLAG_Z) == 0 {
          // Zero flag is not set
          let next = self.pc + 3;
          self.push(mem, next);
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xc5 => { // PUSH BC
        let value = self.get_register_16(Register16::BC);
        self.push(mem, value);
        (1, 16)
      },
      0xc6 => { // ADD A,n
        let (value, overflow) = self.get_register_8(Register8::A).overflowing_add(mem.get_byte(index + 1));
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        (2, 8)
      },
      0xc7 => { // RST 0
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 0;
        (0, 32)
      },
      0xc8 => { // RET Z
        if self.flags & (1 << FLAG_Z) != 0 {
          // Zero flag is set;
          let value = self.pop(mem);
          self.pc = value;
          (0, 8)
        } else {
          (1, 8)
        }
      },
      0xc9 => { // RET
        let value = self.pop(mem);
        self.pc = value;
        (0, 8)
      },
      0xca => { // JP Z,nn
        if self.flags & (1 << FLAG_Z) != 0 {
          // Zero flag is set
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xcb => { // 2-byte instruction code
        let (by, cy) = match mem.get_byte(index + 1) {
          0x00 => { // RLC B
            self.rotate_register_left(Register8::B, true, false);
            (1, 8)
          },
          0x01 => { // RLC C
            self.rotate_register_left(Register8::C, true, false);
            (1, 8)
          },
          0x02 => { // RLC D
            self.rotate_register_left(Register8::D, true, false);
            (1, 8)
          },
          0x03 => { // RLC E
            self.rotate_register_left(Register8::E, true, false);
            (1, 8)
          },
          0x04 => { // RLC H
            self.rotate_register_left(Register8::H, true, false);
            (1, 8)
          },
          0x05 => { // RLC L
            self.rotate_register_left(Register8::L, true, false);
            (1, 8)
          },
          0x06 => { // RLC (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);
            let carry_in = if 0x80 & orig > 0 {
              1
            } else {
              0
            };
            let carry_out = orig & 0x80 > 0;
            let value = (orig << 1).wrapping_add(carry_in);
            mem.set_byte(addr, value);
            self.flag_test(value, false, carry_out);
            (1, 16)
          },
          0x07 => { // RLC A
            self.rotate_register_left(Register8::A, true, false);
            (1, 8)
          },
          0x08 => { // RRC B
            self.rotate_register_right(Register8::B, true, false);
            (1, 8)
          },
          0x09 => { // RRC C
            self.rotate_register_right(Register8::C, true, false);
            (1, 8)
          },
          0x0a => { // RRC D
            self.rotate_register_right(Register8::D, true, false);
            (1, 8)
          },
          0x0b => { // RRC E
            self.rotate_register_right(Register8::E, true, false);
            (1, 8)
          },
          0x0c => { // RRC H
            self.rotate_register_right(Register8::H, true, false);
            (1, 8)
          },
          0x0d => { // RRC L
            self.rotate_register_right(Register8::L, true, false);
            (1, 8)
          },
          0x0e => { // RRC (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);
            let carry_in = if 1 & orig > 0 {
              0x80
            } else {
              0
            };
            let carry_out = orig & 1 > 0;
            let value = (orig >> 1).wrapping_add(carry_in);
            mem.set_byte(addr, value);
            self.flag_test(value, false, carry_out);
            (1, 16)
          },
          0x0f => { // RRC A
            self.rotate_register_right(Register8::A, true, false);
            (1, 8)
          },
          0x10 => { // RL B
            self.rotate_register_left(Register8::B, false, false);
            (1, 8)
          },
          0x11 => { // RL C
            self.rotate_register_left(Register8::C, false, false);
            (1, 8)
          },
          0x12 => { // RL D
            self.rotate_register_left(Register8::D, false, false);
            (1, 8)
          },
          0x13 => { // RL E
            self.rotate_register_left(Register8::E, false, false);
            (1, 8)
          },
          0x14 => { // RL H
            self.rotate_register_left(Register8::H, false, false);
            (1, 8)
          },
          0x15 => { // RL L
            self.rotate_register_left(Register8::L, false, false);
            (1, 8)
          },
          0x16 => { // RL (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);
            let carry_in = if self.flags & 0x10 > 0 {
              1
            } else {
              0
            };
            let carry_out = orig & 0x80 > 0;
            let value = (orig << 1).wrapping_add(carry_in);
            mem.set_byte(addr, value);
            self.flag_test(value, false, carry_out);
            (1, 16)
          },
          0x17 => { // RL A
            self.rotate_register_left(Register8::A, false, false);
            (1, 8)
          },
          0x18 => { // RR B
            self.rotate_register_right(Register8::B, false, false);
            (1, 8)
          },
          0x19 => { // RR C
            self.rotate_register_right(Register8::C, false, false);
            (1, 8)
          },
          0x1a => { // RR D
            self.rotate_register_right(Register8::D, false, false);
            (1, 8)
          },
          0x1b => { // RR E
            self.rotate_register_right(Register8::E, false, false);
            (1, 8)
          },
          0x1c => { // RR H
            self.rotate_register_right(Register8::H, false, false);
            (1, 8)
          },
          0x1d => { // RR L
            self.rotate_register_right(Register8::L, false, false);
            (1, 8)
          },
          0x1e => { // RR (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);

            let carry_in = if self.flags & 0x10 > 0 {
              0x80
            } else {
              0
            };
            let carry_out = orig & 1 > 0;
            let value = (orig >> 1).wrapping_add(carry_in);
            mem.set_byte(addr, value);
            self.flag_test(value, false, carry_out);
            (1, 16)
          },
          0x1f => { // RR A
            self.rotate_register_right(Register8::A, false, false);
            (1, 8)
          },
          0x20 => { // SLA B
            self.shift_register_left(Register8::B);
            (1, 8)
          },
          0x21 => { // SLA C
            self.shift_register_left(Register8::C);
            (1, 8)
          },
          0x22 => { // SLA D
            self.shift_register_left(Register8::D);
            (1, 8)
          },
          0x23 => { // SLA E
            self.shift_register_left(Register8::E);
            (1, 8)
          },
          0x24 => { // SLA H
            self.shift_register_left(Register8::H);
            (1, 8)
          },
          0x25 => { // SLA L
            self.shift_register_left(Register8::L);
            (1, 8)
          },
          0x26 => { // SLA (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);
            let carry_out = orig & 0x80 > 0;
            let value = orig << 1;
            mem.set_byte(addr, value);
            self.flag_test(value, false, false);
            if carry_out {
              self.set_flag(FLAG_C);
            }
            (1, 16)
          },
          0x27 => { // SLA A
            self.shift_register_left(Register8::A);
            (1, 8)
          },
          0x28 => { // SRA B
            self.shift_register_right(Register8::B, true);
            (1, 8)
          },
          0x29 => { // SRA C
            self.shift_register_right(Register8::C, true);
            (1, 8)
          },
          0x2a => { // SRA D
            self.shift_register_right(Register8::D, true);
            (1, 8)
          },
          0x2b => { // SRA E
            self.shift_register_right(Register8::E, true);
            (1, 8)
          },
          0x2c => { // SRA H
            self.shift_register_right(Register8::H, true);
            (1, 8)
          },
          0x2d => { // SRA L
            self.shift_register_right(Register8::L, true);
            (1, 8)
          },
          0x2e => { // SRA (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);
            let carry_out = orig & 1 > 0;
            let value = (orig >> 1).wrapping_add(0x80 & orig);
            mem.set_byte(addr, value);
            self.flag_test(value, false, false);
            if carry_out {
              self.set_flag(FLAG_C);
            }
            (1, 16)
          },
          0x2f => { // SRA A
            self.shift_register_right(Register8::A, true);
            (1, 8)
          },
          0x30 => { // SWAP B
            self.swap(Register8::B);
            (1, 8)
          },
          0x31 => { // SWAP C
            self.swap(Register8::C);
            (1, 8)
          },
          0x32 => { // SWAP D
            self.swap(Register8::D);
            (1, 8)
          },
          0x33 => { // SWAP E
            self.swap(Register8::E);
            (1, 8)
          },
          0x34 => { // SWAP H
            self.swap(Register8::H);
            (1, 8)
          },
          0x35 => { // SWAP L
            self.swap(Register8::L);
            (1, 8)
          },
          0x36 => { // SWAP (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);
            let low = orig & 0xf;
            let high = (orig & 0xf0) >> 4;
            let value = (low << 4) | high;
            mem.set_byte(addr, value);
            self.flag_test_zero(value);
            self.clear_flag(FLAG_N);
            self.clear_flag(FLAG_H);
            self.clear_flag(FLAG_C);
            (1, 16)
          },
          0x37 => { // SWAP A
            self.swap(Register8::A);
            (1, 8)
          },
          0x38 => { // SRL B
            self.shift_register_right(Register8::B, false);
            (1, 8)
          },
          0x39 => { // SRL C
            self.shift_register_right(Register8::C, false);
            (1, 8)
          },
          0x3a => { // SRL D
            self.shift_register_right(Register8::D, false);
            (1, 8)
          },
          0x3b => { // SRL E
            self.shift_register_right(Register8::E, false);
            (1, 8)
          },
          0x3c => { // SRL H
            self.shift_register_right(Register8::H, false);
            (1, 8)
          },
          0x3d => { // SRL L
            self.shift_register_right(Register8::L, false);
            (1, 8)
          },
          0x3e => { // SRL (HL)
            let addr = self.get_register_16(Register16::HL);
            let orig = mem.get_byte(addr);
            let carry_out = orig & 1 > 0;
            let value = orig >> 1;
            mem.set_byte(addr, value);
            self.flag_test(value, false, false);
            if carry_out {
              self.set_flag(FLAG_C);
            }
            (1, 16)
          },
          0x3f => { // SRL A
            self.shift_register_right(Register8::A, false);
            (1, 8)
          },
          0x40 => { // BIT 0,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x41 => { // BIT 0,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x42 => { // BIT 0,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x43 => { // BIT 0,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x44 => { // BIT 0,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x45 => { // BIT 0,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x46 => { // BIT 0,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x47 => { // BIT 0,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x1);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x48 => { // BIT 1,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x49 => { // BIT 1,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x4a => { // BIT 1,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x4b => { // BIT 1,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x4c => { // BIT 1,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x4d => { // BIT 1,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x4e => { // BIT 1,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x4f => { // BIT 1,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x2);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x50 => { // BIT 2,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x51 => { // BIT 2,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x52 => { // BIT 2,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x53 => { // BIT 2,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x54 => { // BIT 2,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x55 => { // BIT 2,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x56 => { // BIT 2,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x57 => { // BIT 2,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x4);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x58 => { // BIT 3,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x59 => { // BIT 3,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x5a => { // BIT 3,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x5b => { // BIT 3,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x5c => { // BIT 3,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x5d => { // BIT 3,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x5e => { // BIT 3,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x5f => { // BIT 3,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x8);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x60 => { // BIT 4,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x61 => { // BIT 4,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x62 => { // BIT 4,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x63 => { // BIT 4,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x64 => { // BIT 4,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x65 => { // BIT 4,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x66 => { // BIT 4,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x67 => { // BIT 4,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x10);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x68 => { // BIT 5,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x69 => { // BIT 5,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x6a => { // BIT 5,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x6b => { // BIT 5,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x6c => { // BIT 5,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x6d => { // BIT 5,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x6e => { // BIT 5,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x6f => { // BIT 5,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x20);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x70 => { // BIT 6,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x71 => { // BIT 6,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x72 => { // BIT 6,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x73 => { // BIT 6,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x74 => { // BIT 6,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x75 => { // BIT 6,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x76 => { // BIT 6,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x77 => { // BIT 6,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x40);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x78 => { // BIT 7,B
            let value = self.get_register_8(Register8::B);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x79 => { // BIT 7,C
            let value = self.get_register_8(Register8::C);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x7a => { // BIT 7,D
            let value = self.get_register_8(Register8::D);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x7b => { // BIT 7,E
            let value = self.get_register_8(Register8::E);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x7c => { // BIT 7,H
            let value = self.get_register_8(Register8::H);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x7d => { // BIT 7,L
            let value = self.get_register_8(Register8::L);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x7e => { // BIT 7,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 16)
          },
          0x7f => { // BIT 7,A
            let value = self.get_register_8(Register8::A);
            self.flag_test_zero(value & 0x80);
            self.clear_flag(FLAG_N);
            self.set_flag(FLAG_H);
            (1, 8)
          },
          0x80 => { // RES 0,B
            let value = self.get_register_8(Register8::B) & 0xfe;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0x81 => { // RES 0,C
            let value = self.get_register_8(Register8::C) & 0xfe;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0x82 => { // RES 0,D
            let value = self.get_register_8(Register8::D) & 0xfe;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0x83 => { // RES 0,E
            let value = self.get_register_8(Register8::E) & 0xfe;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0x84 => { // RES 0,H
            let value = self.get_register_8(Register8::H) & 0xfe;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0x85 => { // RES 0,L
            let value = self.get_register_8(Register8::L) & 0xfe;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0x86 => { // RES 0,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0xfe;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0x87 => { // RES 0,A
            let value = self.get_register_8(Register8::A) & 0xfe;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0x88 => { // RES 1,B
            let value = self.get_register_8(Register8::B) & 0xfd;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0x89 => { // RES 1,C
            let value = self.get_register_8(Register8::C) & 0xfd;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0x8a => { // RES 1,D
            let value = self.get_register_8(Register8::D) & 0xfd;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0x8b => { // RES 1,E
            let value = self.get_register_8(Register8::E) & 0xfd;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0x8c => { // RES 1,H
            let value = self.get_register_8(Register8::H) & 0xfd;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0x8d => { // RES 1,L
            let value = self.get_register_8(Register8::L) & 0xfd;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0x8e => { // RES 1,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0xfd;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0x8f => { // RES 1,A
            let value = self.get_register_8(Register8::A) & 0xfd;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0x90 => { // RES 2,B
            let value = self.get_register_8(Register8::B) & 0xfb;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0x91 => { // RES 2,C
            let value = self.get_register_8(Register8::C) & 0xfb;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0x92 => { // RES 2,D
            let value = self.get_register_8(Register8::D) & 0xfb;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0x93 => { // RES 2,E
            let value = self.get_register_8(Register8::E) & 0xfb;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0x94 => { // RES 2,H
            let value = self.get_register_8(Register8::H) & 0xfb;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0x95 => { // RES 2,L
            let value = self.get_register_8(Register8::L) & 0xfb;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0x96 => { // RES 2,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0xfb;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0x97 => { // RES 2,A
            let value = self.get_register_8(Register8::A) & 0xfb;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0x98 => { // RES 3,B
            let value = self.get_register_8(Register8::B) & 0xf7;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0x99 => { // RES 3,C
            let value = self.get_register_8(Register8::C) & 0xf7;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0x9a => { // RES 3,D
            let value = self.get_register_8(Register8::D) & 0xf7;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0x9b => { // RES 3,E
            let value = self.get_register_8(Register8::E) & 0xf7;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0x9c => { // RES 3,H
            let value = self.get_register_8(Register8::H) & 0xf7;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0x9d => { // RES 3,L
            let value = self.get_register_8(Register8::L) & 0xf7;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0x9e => { // RES 3,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0xf7;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0x9f => { // RES 3,A
            let value = self.get_register_8(Register8::A) & 0xf7;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xa0 => { // RES 4,B
            let value = self.get_register_8(Register8::B) & 0xef;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xa1 => { // RES 4,C
            let value = self.get_register_8(Register8::C) & 0xef;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xa2 => { // RES 4,D
            let value = self.get_register_8(Register8::D) & 0xef;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xa3 => { // RES 4,E
            let value = self.get_register_8(Register8::E) & 0xef;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xa4 => { // RES 4,H
            let value = self.get_register_8(Register8::H) & 0xef;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xa5 => { // RES 4,L
            let value = self.get_register_8(Register8::L) & 0xef;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xa6 => { // RES 4,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0xef;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xa7 => { // RES 4,A
            let value = self.get_register_8(Register8::A) & 0xef;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xa8 => { // RES 5,B
            let value = self.get_register_8(Register8::B) & 0xdf;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xa9 => { // RES 5,C
            let value = self.get_register_8(Register8::C) & 0xdf;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xaa => { // RES 5,D
            let value = self.get_register_8(Register8::D) & 0xdf;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xab => { // RES 5,E
            let value = self.get_register_8(Register8::E) & 0xdf;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xac => { // RES 5,H
            let value = self.get_register_8(Register8::H) & 0xdf;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xad => { // RES 5,L
            let value = self.get_register_8(Register8::L) & 0xdf;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xae => { // RES 5,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0xdf;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xaf => { // RES 5,A
            let value = self.get_register_8(Register8::A) & 0xdf;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },

          0xb0 => { // RES 6,B
            let value = self.get_register_8(Register8::B) & 0xbf;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xb1 => { // RES 6,C
            let value = self.get_register_8(Register8::C) & 0xbf;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xb2 => { // RES 6,D
            let value = self.get_register_8(Register8::D) & 0xbf;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xb3 => { // RES 6,E
            let value = self.get_register_8(Register8::E) & 0xbf;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xb4 => { // RES 6,H
            let value = self.get_register_8(Register8::H) & 0xbf;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xb5 => { // RES 6,L
            let value = self.get_register_8(Register8::L) & 0xbf;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xb6 => { // RES 6,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0xbf;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xb7 => { // RES 6,A
            let value = self.get_register_8(Register8::A) & 0xbf;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xb8 => { // RES 7,B
            let value = self.get_register_8(Register8::B) & 0x7f;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xb9 => { // RES 7,C
            let value = self.get_register_8(Register8::C) & 0x7f;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xba => { // RES 7,D
            let value = self.get_register_8(Register8::D) & 0x7f;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xbb => { // RES 7,E
            let value = self.get_register_8(Register8::E) & 0x7f;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xbc => { // RES 7,H
            let value = self.get_register_8(Register8::H) & 0x7f;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xbd => { // RES 7,L
            let value = self.get_register_8(Register8::L) & 0x7f;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xbe => { // RES 7,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) & 0x7f;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xbf => { // RES 7,A
            let value = self.get_register_8(Register8::A) & 0x7f;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xc0 => { // SET 0,B
            let value = self.get_register_8(Register8::B) | 0x01;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xc1 => { // SET 0,C
            let value = self.get_register_8(Register8::C) | 0x01;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xc2 => { // SET 0,D
            let value = self.get_register_8(Register8::D) | 0x01;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xc3 => { // SET 0,E
            let value = self.get_register_8(Register8::E) | 0x01;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xc4 => { // SET 0,H
            let value = self.get_register_8(Register8::H) | 0x01;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xc5 => { // SET 0,L
            let value = self.get_register_8(Register8::L) | 0x01;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xc6 => { // SET 0,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x01;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xc7 => { // SET 0,A
            let value = self.get_register_8(Register8::A) | 0x01;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xc8 => { // SET 1,B
            let value = self.get_register_8(Register8::B) | 0x02;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xc9 => { // SET 1,C
            let value = self.get_register_8(Register8::C) | 0x02;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xca => { // SET 1,D
            let value = self.get_register_8(Register8::D) | 0x02;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xcb => { // SET 1,E
            let value = self.get_register_8(Register8::E) | 0x02;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xcc => { // SET 1,H
            let value = self.get_register_8(Register8::H) | 0x02;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xcd => { // SET 1,L
            let value = self.get_register_8(Register8::L) | 0x02;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xce => { // SET 1,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x02;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xcf => { // SET 1,A
            let value = self.get_register_8(Register8::A) | 0x02;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xd0 => { // SET 2,B
            let value = self.get_register_8(Register8::B) | 0x04;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xd1 => { // SET 2,C
            let value = self.get_register_8(Register8::C) | 0x04;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xd2 => { // SET 2,D
            let value = self.get_register_8(Register8::D) | 0x04;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xd3 => { // SET 2,E
            let value = self.get_register_8(Register8::E) | 0x04;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xd4 => { // SET 2,H
            let value = self.get_register_8(Register8::H) | 0x04;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xd5 => { // SET 2,L
            let value = self.get_register_8(Register8::L) | 0x04;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xd6 => { // SET 2,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x04;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xd7 => { // SET 2,A
            let value = self.get_register_8(Register8::A) | 0x04;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xd8 => { // SET 3,B
            let value = self.get_register_8(Register8::B) | 0x08;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xd9 => { // SET 3,C
            let value = self.get_register_8(Register8::C) | 0x08;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xda => { // SET 3,D
            let value = self.get_register_8(Register8::D) | 0x08;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xdb => { // SET 3,E
            let value = self.get_register_8(Register8::E) | 0x08;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xdc => { // SET 3,H
            let value = self.get_register_8(Register8::H) | 0x08;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xdd => { // SET 3,L
            let value = self.get_register_8(Register8::L) | 0x08;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xde => { // SET 3,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x08;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xdf => { // SET 3,A
            let value = self.get_register_8(Register8::A) | 0x08;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xe0 => { // SET 4,B
            let value = self.get_register_8(Register8::B) | 0x10;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xe1 => { // SET 4,C
            let value = self.get_register_8(Register8::C) | 0x10;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xe2 => { // SET 4,D
            let value = self.get_register_8(Register8::D) | 0x10;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xe3 => { // SET 4,E
            let value = self.get_register_8(Register8::E) | 0x10;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xe4 => { // SET 4,H
            let value = self.get_register_8(Register8::H) | 0x10;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xe5 => { // SET 4,L
            let value = self.get_register_8(Register8::L) | 0x10;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xe6 => { // SET 4,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x10;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xe7 => { // SET 4,A
            let value = self.get_register_8(Register8::A) | 0x10;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xe8 => { // SET 5,B
            let value = self.get_register_8(Register8::B) | 0x20;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xe9 => { // SET 5,C
            let value = self.get_register_8(Register8::C) | 0x20;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xea => { // SET 5,D
            let value = self.get_register_8(Register8::D) | 0x20;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xeb => { // SET 5,E
            let value = self.get_register_8(Register8::E) | 0x20;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xec => { // SET 5,H
            let value = self.get_register_8(Register8::H) | 0x20;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xed => { // SET 5,L
            let value = self.get_register_8(Register8::L) | 0x20;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xee => { // SET 5,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x20;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xef => { // SET 5,A
            let value = self.get_register_8(Register8::A) | 0x20;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xf0 => { // SET 6,B
            let value = self.get_register_8(Register8::B) | 0x40;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xf1 => { // SET 6,C
            let value = self.get_register_8(Register8::C) | 0x40;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xf2 => { // SET 6,D
            let value = self.get_register_8(Register8::D) | 0x40;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xf3 => { // SET 6,E
            let value = self.get_register_8(Register8::E) | 0x40;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xf4 => { // SET 6,H
            let value = self.get_register_8(Register8::H) | 0x40;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xf5 => { // SET 6,L
            let value = self.get_register_8(Register8::L) | 0x40;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xf6 => { // SET 6,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x40;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xf7 => { // SET 6,A
            let value = self.get_register_8(Register8::A) | 0x40;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          0xf8 => { // SET 7,B
            let value = self.get_register_8(Register8::B) | 0x80;
            self.set_register_8(Register8::B, value);
            (1, 8)
          },
          0xf9 => { // SET 7,C
            let value = self.get_register_8(Register8::C) | 0x80;
            self.set_register_8(Register8::C, value);
            (1, 8)
          },
          0xfa => { // SET 7,D
            let value = self.get_register_8(Register8::D) | 0x80;
            self.set_register_8(Register8::D, value);
            (1, 8)
          },
          0xfb => { // SET 7,E
            let value = self.get_register_8(Register8::E) | 0x80;
            self.set_register_8(Register8::E, value);
            (1, 8)
          },
          0xfc => { // SET 7,H
            let value = self.get_register_8(Register8::H) | 0x80;
            self.set_register_8(Register8::H, value);
            (1, 8)
          },
          0xfd => { // SET 7,L
            let value = self.get_register_8(Register8::L) | 0x80;
            self.set_register_8(Register8::L, value);
            (1, 8)
          },
          0xfe => { // SET 7,(HL)
            let addr = self.get_register_16(Register16::HL);
            let value = mem.get_byte(addr) | 0x80;
            mem.set_byte(addr, value);
            (1, 16)
          },
          0xff => { // SET 7,A
            let value = self.get_register_8(Register8::A) | 0x80;
            self.set_register_8(Register8::A, value);
            (1, 8)
          },
          _ => {
            state = RunState::Crash;
            (1, 4)
          },
        };
        (by + 1, cy)
      },
      0xcc => { // CALL Z,nn
        if self.flags & (1 << FLAG_Z) != 0 {
          // Zero flag is set
          let next = self.pc + 3;
          self.push(mem, next);
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xcd => { // CALL nn
        let next = self.pc + 3;
        self.push(mem, next);
        self.pc = mem.get_word(index + 1);
        (0, 12)
      },
      0xce => { // ADC A,n
        let (mut value, mut overflow) = self.get_register_8(Register8::A).overflowing_add(mem.get_byte(index + 1));
        if self.flags & (1 << FLAG_C) != 0 {
          let (carry_value, carry_overflow) = value.overflowing_add(1);
          value = carry_value;
          overflow = carry_overflow;
        }
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, overflow);
        (1, 8)
      },
      0xcf => { // RST 08
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 8;
        (0, 32)
      },
      0xd0 => { // RET NC
        if self.flags & (1 << FLAG_C) == 0 {
          // Carry flag is not set
          let value = self.pop(mem);
          self.pc = value;
          (0, 8)
        } else {
          (1, 8)
        }
      },
      0xd1 => { // POP DE
        let value = self.pop(mem);
        self.set_register_16(Register16::DE, value);
        (1, 12)
      },
      0xd2 => { // JP NC,nn
        if self.flags & (1 << FLAG_C) == 0 {
          // Carry flag is not set
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xd3 => (1, 4),
      0xd4 => { // CALL NC,nn
        if self.flags & (1 << FLAG_C) == 0 {
          // Carry flag is not set
          let next = self.pc + 3;
          self.push(mem, next);
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xd5 => { // PUSH DE
        let value = self.get_register_16(Register16::DE);
        self.push(mem, value);
        (1, 16)
      },
      0xd6 => { // SUB A,n
        let (value, overflow) = self.get_register_8(Register8::A).overflowing_sub(mem.get_byte(index + 1));
        self.set_register_8(Register8::A, value);
        self.flag_test(value, true, overflow);
        (2, 8)
      },
      0xd7 => { // RST 10
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 0x10;
        (0, 32)
      },
      0xd8 => { // RET C
        if self.flags & (1 << FLAG_C) != 0 {
          // Carry flag is set
          let value = self.pop(mem);
          self.pc = value;
          (0, 8)
        } else {
          (1, 8)
        }
      },
      0xd9 => { // RETI
        self.enable_interrupts();
        let value = self.pop(mem);
        self.pc = value;
        (0, 8)
      },
      0xda => { // JP C,nn
        if self.flags & (1 << FLAG_C) != 0 {
          // Carry flag is set
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xdb => (1, 4),
      0xdc => { // CALL C,nn
        if self.flags & (1 << FLAG_C) != 0 {
          // Carry flag is set
          let next = self.pc + 3;
          self.push(mem, next);
          self.pc = mem.get_word(index + 1);
          (0, 12)
        } else {
          (3, 12)
        }
      },
      0xdd => (1, 4),
      0xde => { // SBC A,n
        let mut value = self.get_register_8(Register8::A).wrapping_sub(mem.get_byte(index + 1));
        if self.flags & (1 << FLAG_C) != 0 {
          value = value.wrapping_sub(1);
        }
        self.clear_flag(FLAG_N);
        self.set_register_8(Register8::A, value);
        (1, 8)
      },
      0xdf => { // RST 18
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 0x18;
        (0, 32)
      },
      0xe0 => { // LDH (n),A
        let value = self.get_register_8(Register8::A);
        let addr = 0xff00 + (mem.get_byte(index + 1) as u16);
        mem.set_byte(addr, value);
        (2, 12)
      },
      0xe1 => { // POP HL
        let value = self.pop(mem);
        self.set_register_16(Register16::HL, value);
        (1, 12)
      },
      0xe2 => { // LDH (C),A
        let value = self.get_register_8(Register8::A);
        let addr = 0xff00 + (self.get_register_8(Register8::C) as u16);
        mem.set_byte(addr, value);
        (1, 8)
      },
      0xe3 => (1, 4),
      0xe4 => (1, 4),
      0xe5 => { // PUSH HL
        let value = self.get_register_16(Register16::HL);
        self.push(mem, value);
        (1, 16)
      },
      0xe6 => { // AND n
        let value = self.get_register_8(Register8::A) & mem.get_byte(index + 1);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (2, 8)
      },
      0xe7 => { // RST 20
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 0x20;
        (0, 32)
      },
      0xe8 => { // ADD SP,d
        let orig = self.get_register_16(Register16::SP);
        let d = mem.get_byte(index + 1);
        let sub = d & 0x80 > 0;
        let delta = (if sub { !d + 1 } else { d }) as u16;
        let value = if sub {
          // offset is negative
          orig.wrapping_sub(delta)
        } else {
          orig.wrapping_add(delta)
        };
        self.set_register_16(Register16::SP, value);
        if (orig & 0xf) + ((d as u16) & 0xf) > 0xf {
          self.set_flag(FLAG_H);
        } else {
          self.clear_flag(FLAG_H);
        }
        if (orig & 0xff) + ((d as u16) & 0xff) > 0xff {
          self.set_flag(FLAG_C);
        } else {
          self.clear_flag(FLAG_C);
        }
        self.clear_flag(FLAG_Z);
        self.clear_flag(FLAG_N);
        (2, 16)
      },
      0xe9 => { // JP (HL)
        let addr = self.get_register_16(Register16::HL);
        self.pc = addr;
        (0, 4)
      },
      0xea => { // LD (nn),A
        let addr = mem.get_word(index + 1);
        let value = self.get_register_8(Register8::A);
        mem.set_byte(addr, value);
        (3, 16)
      },
      0xeb => (1, 4),
      0xec => (1, 4),
      0xed => (1, 4),
      0xee => { // XOR n
        let value = self.get_register_8(Register8::A) ^ mem.get_byte(index + 1);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (2, 8)
      },
      0xef => { // RST 28
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 0x28;
        (0, 32)
      },
      0xf0 => { // LDH A,(n)
        let addr = 0xff00 + (mem.get_byte(index + 1) as u16);
        let value = mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        (2, 12)
      },
      0xf1 => { // POP AF
        let value = self.pop(mem);
        self.set_register_16(Register16::AF, value);
        (1, 12)
      },
      0xf2 => { // LD A,(C)
        let addr = 0xff00 + (self.get_register_8(Register8::C) as u16);
        let value = mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        (1, 8)
      },
      0xf3 => { // DI
        self.disable_interrupts();
        (1, 4)
      },
      0xf4 => (1, 4),
      0xf5 => { // PUSH AF
        let value = self.get_register_16(Register16::AF);
        self.push(mem, value);
        (1, 16)
      },
      0xf6 => { // OR n
        let value = self.get_register_8(Register8::A) | mem.get_byte(index + 1);
        self.set_register_8(Register8::A, value);
        self.flag_test(value, false, false);
        (2, 8)
      },
      0xf7 => { // RST 30
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 0x30;
        (0, 32)
      },
      0xf8 => { // LDHL SP,d
        let orig = self.get_register_16(Register16::SP);
        let d = mem.get_byte(index + 1);
        let sub = d & 0x80 > 0;
        let delta = (if sub { !d + 1 } else { d }) as u16;
        let value = if sub {
          // offset is negative
          orig.wrapping_sub(delta)
        } else {
          orig.wrapping_add(delta)
        };
        self.set_register_16(Register16::HL, value);
        if (orig & 0xf) + ((d as u16) & 0xf) > 0xf {
          self.set_flag(FLAG_H);
        } else {
          self.clear_flag(FLAG_H);
        }
        if (orig & 0xff) + ((d as u16) & 0xff) > 0xff {
          self.set_flag(FLAG_C);
        } else {
          self.clear_flag(FLAG_C);
        }
        self.clear_flag(FLAG_Z);
        self.clear_flag(FLAG_N);
        (2, 12)
      },
      0xf9 => { // LD SP,HL
        let value = self.get_register_16(Register16::HL);
        self.set_register_16(Register16::SP, value);
        (1, 8)
      },
      0xfa => { // LD A,(nn)
        let addr = mem.get_word(index + 1);
        let value = mem.get_byte(addr);
        self.set_register_8(Register8::A, value);
        (3, 16)
      },
      0xfb => { // EI
        self.enable_interrupts();
        (1, 4)
      },
      0xfc => (1, 4),
      0xfd => (1, 4),
      0xfe => { // CP n
        let a = self.get_register_8(Register8::A);
        let n = mem.get_byte(index + 1);
        let (value, overflow) = a.overflowing_sub(n);
        self.flag_test(value, true, overflow);
        (2, 8)
      },
      0xff => { // RST 38
        let next = self.pc + 1;
        self.push(mem, next);
        self.pc = 0x38;
        (0, 32)
      },
      _ => {
        state = RunState::Crash; 
        (0, 4)
      },
    };
    self.pc += byte_len;

    return (state, cycles);
  }
}

#[cfg(test)]
mod tests {
  use vm::cpu::create_cpu;
  use vm::cpu::Register8;
  use vm::cpu::Register16;
  use vm::memmap::create_memmap;

  #[test]
  fn get_register_8() {
    let mut cpu = create_cpu();
    cpu.a = 5;
    assert_eq!(cpu.get_register_8(Register8::A), 5);
  }

  #[test]
  fn increment() {
    let mut cpu = create_cpu();
    cpu.set_register_8(Register8::B, 5);
    cpu.inc_8(Register8::B);
    assert_eq!(cpu.get_register_8(Register8::B), 6);
    assert_eq!(cpu.get_register_8(Register8::Flags), 0);

    cpu.set_register_8(Register8::B, 0x2f);
    cpu.inc_8(Register8::B);
    assert_eq!(cpu.get_register_8(Register8::B), 0x30);
    assert!(cpu.flags & (1 << 5) > 0);

    cpu.set_register_8(Register8::B, 0xff);
    cpu.inc_8(Register8::B);
    assert_eq!(cpu.get_register_8(Register8::B), 0);
    assert!(cpu.flags & (1 << 7) > 0);
    assert!(cpu.flags & (1 << 5) > 0);
  }

  #[test]
  fn instruction_0x01() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    mem.set_byte(0xc010, 0x01);
    mem.set_byte(0xc011, 0x3d);
    assert_eq!(mem.get_byte(0xc010), 0x01);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_16(Register16::BC), 0x3d);
  }

  #[test]
  fn instruction_0x02() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.a = 36;
    cpu.set_register_16(Register16::BC, 0x8040);
    mem.set_byte(0xc010, 0x02);
    cpu.step(&mut mem);
    assert_eq!(mem.get_byte(0x8040), 36);
  }

  #[test]
  fn instruction_0x03() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_register_16(Register16::BC, 4);
    mem.set_byte(0xc010, 0x03);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_16(Register16::BC), 5);
  }

  #[test]
  fn instruction_0x04() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_register_8(Register8::B, 40);
    mem.set_byte(0xc010, 0x04);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::B), 41);
  }

  #[test]
  fn instruction_0x05() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_register_8(Register8::B, 1);
    mem.set_byte(0xc010, 0x05);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::B), 0);
    assert!(cpu.get_register_8(Register8::Flags) & (1 << 7) > 0);
  }

  #[test]
  fn instruction_0x06() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    mem.set_byte(0xc010, 0x06);
    mem.set_byte(0xc011, 60);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::B), 60);
  }

  #[test]
  fn instruction_0x08() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.sp = 0xfffc;
    mem.set_byte(0xc010, 0x08);
    mem.set_word(0xc011, 0x8088);
    cpu.step(&mut mem);
    assert_eq!(mem.get_word(0x8088), 0xfffc);
  }

  #[test]
  fn instruction_0x09() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_register_16(Register16::HL, 0x4040);
    cpu.set_register_16(Register16::BC, 0x102);
    mem.set_byte(0xc010, 0x09);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_16(Register16::HL), 0x4142);
    // test wrap
    cpu.set_register_16(Register16::HL, 0xfff7);
    cpu.set_register_16(Register16::BC, 0xb);
    mem.set_byte(0xc011, 0x09);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_16(Register16::HL), 2);
  }

  #[test]
  fn instruction_0x0a() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_register_16(Register16::BC, 0x8123);
    mem.set_byte(0x8123, 45);
    mem.set_byte(0xc010, 0x0a);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::A), 45);
  }

  #[test]
  fn instruction_0x0e() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.c = 5;
    mem.set_byte(0xc010, 0x0e);
    mem.set_byte(0xc011, 0xf0);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::C), 0xf0);
  }

  #[test]
  fn instruction_0x11() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    mem.set_byte(0xc010, 0x11);
    mem.set_word(0xc011, 0x1234);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_16(Register16::DE), 0x1234);
  }

  #[test]
  fn instruction_0x12() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.a = 99;
    cpu.set_register_16(Register16::DE, 0x8003);
    mem.set_byte(0xc010, 0x12);
    cpu.step(&mut mem);
    assert_eq!(mem.get_byte(0x8003), 99);
  }

  #[test]
  fn instruction_0x16() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    mem.set_byte(0xc010, 0x16);
    mem.set_byte(0xc011, 12);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::D), 12);
  }

  #[test]
  fn instruction_0x18() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    mem.set_byte(0xc010, 0x18);
    mem.set_byte(0xc011, 0x14);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0xc026);
    mem.set_byte(0xc026, 0x18);
    mem.set_byte(0xc027, 0xfc); // -4
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0xc024);
  }

  #[test]
  fn instruction_0x19() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_register_16(Register16::HL, 0x0030);
    cpu.set_register_16(Register16::DE, 0x0120);
    mem.set_byte(0xc010, 0x19);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_16(Register16::HL), 0x150);
  }

  #[test]
  fn instruction_0x1a() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_register_16(Register16::DE, 0x8003);
    mem.set_byte(0xc010, 0x1a);
    mem.set_byte(0x8003, 99);
    cpu.step(&mut mem);
    assert_eq!(cpu.a, 99);
  }

  #[test]
  fn instruction_0x1e() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    mem.set_byte(0xc010, 0x1e);
    mem.set_byte(0xc011, 88);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::E), 88);
  }

  #[test]
  fn instruction_0x20() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_flag(7);
    mem.set_byte(0xc010, 0x20);
    mem.set_byte(0xc011, 0x12);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0xc012);
    cpu.pc = 0xc010;
    cpu.clear_flag(7);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0xc024);
  }

  #[test]
  fn instruction_0x22() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.a = 4;
    cpu.set_register_16(Register16::HL, 0x8089);
    mem.set_byte(0xc010, 0x22);
    cpu.step(&mut mem);
    assert_eq!(mem.get_byte(0x8089), 4);
    assert_eq!(cpu.get_register_16(Register16::HL), 0x808a);
  }

  #[test]
  fn instruction_0x2f() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.a = 0x2f;
    mem.set_byte(0xc010, 0x2f);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::A), 0xd0);
  }

  #[test]
  fn instruction_0x88() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.flags = 1 << 4;
    cpu.a = 0x38;
    cpu.b = 0x05;
    mem.set_byte(0xc010, 0x88);
    mem.set_byte(0xc011, 0x88);
    mem.set_byte(0xc012, 0x88);
    mem.set_byte(0xc013, 0x88);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::A), 0x3e);
    assert_eq!(cpu.flags, 0x00);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::A), 0x43);
    assert_eq!(cpu.flags, 0x20);
    cpu.b = 0xbd;
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::A), 0x00);
    assert_eq!(cpu.flags, 0xb0);
    cpu.a = 0x43;
    cpu.b = 0xbc;
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_8(Register8::A), 0x00);
    assert_eq!(cpu.flags, 0xb0);
  }

  #[test]
  fn instruction_0xc0() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.sp = 0xfffe;
    cpu.push(&mut mem, 43);
    mem.set_byte(0xc010, 0xc0);
    cpu.set_flag(7);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0xc011);
    cpu.pc = 0xc010;
    cpu.clear_flag(7);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 43);
  }

  #[test]
  fn instruction_0xc1() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.sp = 0xfffe;
    cpu.push(&mut mem, 43);
    mem.set_byte(0xc010, 0xc1);
    cpu.step(&mut mem);
    assert_eq!(cpu.get_register_16(Register16::BC), 43);
  }

  #[test]
  fn instruction_0xc2() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.set_flag(7);
    mem.set_byte(0xc010, 0xc2);
    mem.set_word(0xc011, 0x2345);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0xc013);
    cpu.pc = 0xc010;
    cpu.clear_flag(7);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0x2345);
  }

  #[test]
  fn instruction_0xc3() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    mem.set_byte(0xc010, 0xc3);
    mem.set_word(0xc011, 0x2345);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0x2345);
  }

  #[test]
  fn instruction_0xc4() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.sp = 0xfffe;
    cpu.set_flag(7);
    mem.set_byte(0xc010, 0xc4);
    mem.set_word(0xc011, 200);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 0xc013);
    cpu.pc = 0xc010;
    cpu.clear_flag(7);
    cpu.step(&mut mem);
    assert_eq!(cpu.pc, 200);
    assert_eq!(mem.get_byte(0xfffc), 0xc013);
  }

  #[test]
  fn instruction_0xc5() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.sp = 0xfffe;
    cpu.set_register_16(Register16::BC, 0x331);
    mem.set_byte(0xc010, 0xc5);
    cpu.step(&mut mem);
    assert_eq!(mem.get_word(0xfffc), 0x331);
  }

  #[test]
  fn instruction_0xe0() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.a = 5;
    mem.set_byte(0xc010, 0xe0);
    mem.set_byte(0xc011, 0x40);
    cpu.step(&mut mem);
    assert_eq!(mem.get_byte(0xff40), 5);
  }

  #[test]
  fn instruction_0xe2() {
    let mut cpu = create_cpu();
    let mut mem = create_memmap(0);
    cpu.pc = 0xc010;
    cpu.a = 5;
    cpu.c = 0x81;
    mem.set_byte(0xc010, 0xe2);
    cpu.step(&mut mem);
    assert_eq!(mem.get_byte(0xff81), 5);
  }
}