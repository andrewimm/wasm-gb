#[derive(Debug, PartialEq)]
pub enum MBC {
  NoMBC,
  MBC1,
  MBC2,
  MBC3,
  MBC5,
}

#[derive(Debug, PartialEq)]
enum SelectMode {
  ROM,
  RAM,
}

pub struct Cart {
  pub mbc: MBC,
  pub rom_size: u8,
  pub ram_size: u8,
  pub rom_bank: u8,
  pub ram_bank: u8,
  pub has_battery: bool,
  pub has_timer: bool,
  pub has_rumble: bool,

  select_mode: SelectMode, // Only used by MBC1
  ram_enabled: bool,
  pub raw_rom: Box<[u8; 2 * 1024 * 1024]>,
  pub raw_ram: Box<[u8; 32 * 1024]>,
}

impl Cart {
  pub fn get_rom_byte(&self, addr: u16) -> u8 {
    if addr < 0x4000 {
      return self.raw_rom[addr as usize];
    }
    if addr < 0x8000 {
      let offset = addr - 0x4000;
      let bank = if self.mbc == MBC::MBC1 {
        match self.rom_bank {
          0x20 => 0x21,
          0x40 => 0x41,
          0x60 => 0x61,
          _ => self.rom_bank,
        }
      } else {
        self.rom_bank
      };
      let raw_addr = (bank as u32) * 0x4000 + (offset as u32);
      return self.raw_rom[raw_addr as usize];
    }
    return 0xff;
  }

  pub fn get_ram_byte(&self, addr: u16) -> u8 {
    if self.ram_size == 0 { // No RAM
      return 0xff;
    }
    if addr < 0x2000 {
      let raw_addr = (self.ram_bank as u32) * 0x2000 + (addr as u32);
      return self.raw_ram[raw_addr as usize];
    }
    return 0xff;
  }

  pub fn set_ram_byte(&mut self, addr: u16, value: u8) {
    if !self.ram_enabled {
      return;
    }
    if self.ram_size == 0 {
      return;
    }
    if addr < 0x2000 {
      let raw_addr = (self.ram_bank as u32) * 0x2000 + (addr as u32);
      self.raw_ram[raw_addr as usize] = value;
    }
  }

  pub fn write_rom_addr(&mut self, addr: u16, value: u8) {
    if addr < 0x2000 {
      if value & 0xa > 0 {
        self.enable_ram();
      } else {
        self.disable_ram();
      }
      return;
    }
    if addr < 0x4000 {
      match self.mbc {
        MBC::MBC1 => {
          // set lower 5 bits of bank
          let bank_high = self.rom_bank & 0x60;
          let bank_low = value & 0x1f;
          self.set_rom_bank(bank_high | bank_low);
        },
        MBC::MBC3 => {
          self.set_rom_bank(value);
        },
        _ => (),
      };
      return;
    }
    if addr < 0x6000 {
      match self.mbc {
        MBC::MBC1 => {
          if self.select_mode == SelectMode::ROM {
            let bank_high = (value & 0x3) << 5;
            let bank_low = self.rom_bank & 0x1f;
            self.set_rom_bank(bank_high | bank_low);
          } else {
            self.set_ram_bank(value);
          }
        },
        MBC::MBC3 => {
          if value < 0x4 {
            self.set_ram_bank(value);
          } else if value >= 0x8 && value <= 0xc {
            // RTC register
            // Not yet implemented
          }
        },
        _ => (),
      };
      return;
    }
    // 0x6000-0x7fff
    match self.mbc {
      MBC::MBC1 => {
        self.select_mode = if value == 0 { SelectMode::ROM } else { SelectMode::RAM };
      },
      MBC::MBC3 => {
        // Latch RTC register values
        // Not yet implemented
      },
      _ => (),
    }
  }

  pub fn set_rom_bank(&mut self, bank: u8) {
    let mut b = bank & 0x7f;
    if b == 0 {
      b = 1;
    }
    self.rom_bank = b;
  }

  pub fn set_ram_bank(&mut self, bank: u8) {
    self.ram_bank = bank & 0x3
  }

  pub fn enable_ram(&mut self) {
    self.ram_enabled = true;
  }

  pub fn disable_ram(&mut self) {
    self.ram_enabled = false;
  }

  pub fn rom_ptr(&mut self) -> *mut u8 {
    let ptr = &mut self.raw_rom[0] as *mut u8;
    return ptr;
  }

  pub fn ram_ptr(&mut self) -> *mut u8 {
    let ptr = &mut self.raw_ram[0] as *mut u8;
    return ptr;
  }

  fn reset(&mut self, mbc: MBC, rom_size: u8, ram_size: u8, has_battery: bool, has_timer: bool, has_rumble: bool) {
    self.mbc = mbc;
    self.rom_size = rom_size;
    self.ram_size = ram_size;
    self.rom_bank = 0x01;
    self.ram_bank = 0;
    self.has_battery = has_battery;
    self.has_timer = has_timer;
    self.has_rumble = has_rumble;

    self.select_mode = SelectMode::ROM;
    self.ram_enabled = false;
  }

  pub fn set_mbc(&mut self, mbc: u8) {
    match mbc {
      0x01 => // MBC1
        self.reset(MBC::MBC1, 0x80, 0, false, false, false),
      0x02 => // MBC1 + RAM
        self.reset(MBC::MBC1, 0x80, 4, false, false, false),
      0x03 => // MBC1 + RAM + Battery
        self.reset(MBC::MBC1, 0x80, 4, true, false, false),
      0x05 => // MBC2
        self.reset(MBC::MBC2, 0x10, 1, false, false, false),
      0x06 => // MBC2 + Battery
        self.reset(MBC::MBC2, 0x10, 1, true, false, false),
      0x08 => // ROM + RAM
        self.reset(MBC::NoMBC, 1, 1, false, false, false),
      0x09 => // ROM + RAM + Battery
        self.reset(MBC::NoMBC, 1, 1, true, false, false),
      0x0f => // MBC3 + Timer + Battery
        self.reset(MBC::MBC3, 0x80, 0, true, true, false),
      0x10 => // MBC3 + Timer + RAM + Battery
        self.reset(MBC::MBC3, 0x80, 4, true, true, false),
      0x11 => // MBC3
        self.reset(MBC::MBC3, 0x80, 0, false, false, false),
      0x12 => // MBC3 + RAM
        self.reset(MBC::MBC3, 0x80, 4, false, false, false),
      0x13 => // MBC3 + RAM + Battery
        self.reset(MBC::MBC3, 0x80, 4, true, false, false),

      _ => // Default to no MBC
        self.reset(MBC::NoMBC, 1, 0, false, false, false),
    }
  }
}

fn create(mbc: MBC, rom_size: u8, ram_size: u8, has_battery: bool, has_timer: bool, has_rumble: bool) -> Cart {
  return Cart {
    mbc: mbc,
    rom_size: rom_size,
    ram_size: ram_size,
    rom_bank: 0x01,
    ram_bank: 0,
    has_battery: has_battery,
    has_timer: has_timer,
    has_rumble: has_rumble,

    select_mode: SelectMode::ROM,
    ram_enabled: false,
    raw_rom: box [0; 2 * 1024 * 1024],
    raw_ram: box [0; 32 * 1024],
  };
}

pub fn create_cart(mbc: u8) -> Cart {
  return match mbc {
    0x01 => // MBC1
      create(MBC::MBC1, 0x80, 0, false, false, false),
    0x02 => // MBC1 + RAM
      create(MBC::MBC1, 0x80, 4, false, false, false),
    0x03 => // MBC1 + RAM + Battery
      create(MBC::MBC1, 0x80, 4, true, false, false),
    0x05 => // MBC2
      create(MBC::MBC2, 0x10, 1, false, false, false),
    0x06 => // MBC2 + Battery
      create(MBC::MBC2, 0x10, 1, true, false, false),
    0x08 => // ROM + RAM
      create(MBC::NoMBC, 1, 1, false, false, false),
    0x09 => // ROM + RAM + Battery
      create(MBC::NoMBC, 1, 1, true, false, false),
    0x0f => // MBC3 + Timer + Battery
      create(MBC::MBC3, 0x80, 0, true, true, false),
    0x10 => // MBC3 + Timer + RAM + Battery
      create(MBC::MBC3, 0x80, 4, true, true, false),
    0x11 => // MBC3
      create(MBC::MBC3, 0x80, 0, false, false, false),
    0x12 => // MBC3 + RAM
      create(MBC::MBC3, 0x80, 4, false, false, false),
    0x13 => // MBC3 + RAM + Battery
      create(MBC::MBC3, 0x80, 4, true, false, false),

    _ => // Default to no MBC
      create(MBC::NoMBC, 1, 0, false, false, false),
  }
}

#[cfg(test)]
mod tests {
  use vm::cart::create_cart;

  #[test]
  fn no_mbc() {
    let mut cart = create_cart(0);
    cart.raw_rom[0x204] = 12;
    assert_eq!(cart.get_rom_byte(0x204), 12);
    cart.raw_rom[0x4060] = 14;
    assert_eq!(cart.get_rom_byte(0x4060), 14);
    cart.set_ram_byte(0x12, 45);
    assert_eq!(cart.get_ram_byte(0x12), 0xff);
  }

  #[test]
  fn mbc1_rom() {
    let mut cart = create_cart(1);
    cart.raw_rom[0x204] = 12;
    assert_eq!(cart.get_rom_byte(0x204), 12);
    cart.raw_rom[0x4060] = 14;
    cart.raw_rom[0x8060] = 41;
    cart.raw_rom[0x50060] = 20;
    cart.raw_rom[0xa8060] = 42;
    assert_eq!(cart.get_rom_byte(0x4060), 14);
    cart.set_rom_bank(2);
    assert_eq!(cart.get_rom_byte(0x4060), 41);
    cart.write_rom_addr(0x6000, 0);
    cart.write_rom_addr(0x2000, 0);
    assert_eq!(cart.get_rom_byte(0x4060), 14);
    cart.write_rom_addr(0x2000, 20);
    assert_eq!(cart.get_rom_byte(0x4060), 20);
    assert_eq!(cart.rom_bank, 20);
    cart.write_rom_addr(0x4000, 1);
    cart.write_rom_addr(0x2000, 10);
    assert_eq!(cart.rom_bank, 42);
    assert_eq!(cart.get_rom_byte(0x4060), 42);
  }

  #[test]
  fn mbc1_no_ram() {
    let mut cart = create_cart(1);
    assert_eq!(cart.get_ram_byte(0x02), 0xff);
  }

  #[test]
  fn mbc1_ram() {
    let mut cart = create_cart(2);
    cart.write_rom_addr(0x1000, 0xa); // enable ram
    cart.set_ram_byte(0x02, 22);
    assert_eq!(cart.get_ram_byte(0x02), 22);
    cart.write_rom_addr(0x1000, 0);
    cart.set_ram_byte(0x02, 23);
    assert_eq!(cart.get_ram_byte(0x02), 22);
    cart.write_rom_addr(0x6000, 1);
    cart.write_rom_addr(0x4000, 2);
    cart.write_rom_addr(0x1000, 0xa);
    cart.set_ram_byte(0x304, 0xab);
    assert_eq!(cart.get_ram_byte(0x304), 0xab);
    assert_eq!(cart.raw_ram[0x4304], 0xab);
    cart.write_rom_addr(0x4000, 1);
    assert_eq!(cart.get_ram_byte(0x304), 0);
  }

  #[test]
  fn mbc3_rom() {
    let mut cart = create_cart(0x11);
    cart.raw_rom[0x204] = 12;
    assert_eq!(cart.get_rom_byte(0x204), 12);
    cart.raw_rom[0x4060] = 14;
    cart.raw_rom[0x8060] = 41;
    cart.raw_rom[0x50060] = 20;
    cart.raw_rom[0xa8060] = 42;
    assert_eq!(cart.get_rom_byte(0x4060), 14);
    cart.set_rom_bank(2);
    assert_eq!(cart.get_rom_byte(0x4060), 41);
    cart.write_rom_addr(0x2000, 0);
    assert_eq!(cart.get_rom_byte(0x4060), 14);
    cart.write_rom_addr(0x2000, 20);
    assert_eq!(cart.get_rom_byte(0x4060), 20);
    assert_eq!(cart.rom_bank, 20);
    cart.write_rom_addr(0x2000, 42);
    assert_eq!(cart.rom_bank, 42);
    assert_eq!(cart.get_rom_byte(0x4060), 42);
  }
}
