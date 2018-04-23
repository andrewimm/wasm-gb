use vm::audio;
use vm::cart;

#[cfg(not(test))]
extern "C" {
  fn set_master_gain(left: u8, right: u8);
  fn audio_enabled(flag: u8);
}

#[cfg(test)]
fn set_master_gain(left: u8, right: u8) {}
#[cfg(test)]
fn audio_enabled(flag: u8) {}

#[derive(Debug, PartialEq)]
enum KeySelect {
  Buttons,
  Directions,
}

pub struct MemMap {
  pub boot: [u8; 0x100],
  pub video_ram: [u8; 0x2000],
  pub work_ram: [u8; 0x2000],
  pub sprite_table: [u8; 0xa0],
  pub zero_page: [u8; 0x100],

  pub cart: cart::Cart,

  keys_buttons: u8,
  keys_directions: u8,
  key_select: KeySelect,

  timer: u16,

  pub audio: audio::Audio,

  pub cart_ram_dirty: bool,
  pub tile_map_0_dirty: bool,
  pub tile_map_1_dirty: bool,
  pub tile_data_dirty: bool,
}

pub fn create_memmap(mbc: u8) -> MemMap {
  return MemMap {
    boot: [0; 0x100],
    video_ram: [0; 0x2000],
    work_ram: [0; 0x2000],
    sprite_table: [0; 0xa0],
    zero_page: [0; 0x100],

    cart: cart::create_cart(mbc),

    keys_buttons: 0x0f,
    keys_directions: 0x0f,
    key_select: KeySelect::Buttons,

    timer: 0,

    audio: audio::create_audio(),

    cart_ram_dirty: false,
    tile_map_0_dirty: true,
    tile_map_1_dirty: true,
    tile_data_dirty: true,
  };
}

impl MemMap {
  pub fn simulate_bootloader(&mut self) {
    // Modify zero-page RAM to simulate the state after the bootloader has run
    self.set_byte(0xff05, 0x00);
    self.set_byte(0xff06, 0x00);
    self.set_byte(0xff07, 0x00);
    self.set_byte(0xff10, 0x80);
    self.set_byte(0xff11, 0x80);
    self.set_byte(0xff12, 0xf3);
    self.set_byte(0xff13, 0xc1);
    self.set_byte(0xff14, 0x87);
    self.set_byte(0xff16, 0x3f);
    self.set_byte(0xff17, 0x00);
    self.set_byte(0xff19, 0xbf);
    self.set_byte(0xff1a, 0x7f);
    self.set_byte(0xff1b, 0xff);
    self.set_byte(0xff1c, 0x9f);
    self.set_byte(0xff1e, 0xbf);
    self.set_byte(0xff20, 0xff);
    self.set_byte(0xff21, 0x00);
    self.set_byte(0xff22, 0x00);
    self.set_byte(0xff23, 0xbf);
    self.set_byte(0xff24, 0x77);
    self.set_byte(0xff25, 0xf3);
    self.set_byte(0xff26, 0x80);
    self.set_byte(0xff40, 0x91);
    self.set_byte(0xff42, 0x00);
    self.set_byte(0xff43, 0x00);
    self.set_byte(0xff44, 0x8f);
    self.set_byte(0xff45, 0x00);
    self.set_byte(0xff47, 0xfc);
    self.set_byte(0xff48, 0xff);
    self.set_byte(0xff49, 0xff);
    self.set_byte(0xff4a, 0x00);
    self.set_byte(0xff4b, 0x00);
    self.set_byte(0xff50, 0x01);
    self.set_byte(0xfffb, 0x01);
    self.set_byte(0xfffc, 0x2e);
    self.set_byte(0xfffd, 0x00);
    self.set_byte(0xffff, 0x00);
  }

  pub fn get_byte(&self, addr: u16) -> u8 {
    if addr < 0x100 {
      if self.zero_page[0x50] == 0 {
        return self.boot[addr as usize];
      } else {
        return self.cart.get_rom_byte(addr);
      }
    }
    if addr < 0x8000 {
      return self.cart.get_rom_byte(addr);
    }
    if addr < 0xa000 {
      return self.video_ram[(addr - 0x8000) as usize];
    }
    if addr < 0xc000 {
      return self.cart.get_ram_byte(addr - 0xa000);
    }
    if addr < 0xe000 {
      return self.work_ram[(addr - 0xc000) as usize];
    }
    if addr < 0xfe00 {
      return self.work_ram[(addr - 0xe000) as usize];
    }
    if addr < 0xfea0 {
      return self.sprite_table[(addr - 0xfe00) as usize];
    }
    if addr < 0xff00 {
      // Inaccessible
      return 0xff;
    }
    if addr >= 0xff40 {
      return self.zero_page[(addr - 0xff00) as usize];
    }
    if addr == 0xff00 {
      if self.key_select == KeySelect::Buttons {
        return self.keys_buttons & 0xf;
      } else {
        return self.keys_directions & 0xf;
      }
    }
    return self.zero_page[(addr - 0xff00) as usize];
  }

  pub fn set_byte(&mut self, addr: u16, value: u8) {
    if addr < 0x8000 {
      self.cart.write_rom_addr(addr, value);
      return;
    }
    if addr < 0xa000 {
      self.video_ram[(addr - 0x8000) as usize] = value;
      if addr < 0x9800 {
        self.tile_data_dirty = true;
      } else if addr < 0x9c00 {
        self.tile_map_0_dirty = true;
      } else {
        self.tile_map_1_dirty = true;
      }
      return;
    }
    if addr < 0xc000 {
      self.cart.set_ram_byte(addr - 0xa000, value);
      self.cart_ram_dirty = true;
      return;
    }
    if addr < 0xe000 {
      self.work_ram[(addr - 0xc000) as usize] = value;
      return;
    }
    if addr < 0xfe00 {
      self.work_ram[(addr - 0xe000) as usize] = value;
    }
    if addr < 0xfea0 {
      // Sprite table
      self.sprite_table[(addr - 0xfe00) as usize] = value;
      return;
    }
    if addr < 0xff00 {
      // Inaccessible
      return;
    }
    if addr >= 0xff80 {
      self.zero_page[(addr - 0xff00) as usize] = value;
      return;
    }
    if addr >= 0xff40 {
      // Graphics
      if addr == 0xff41 {
        // Can't write the lower three bits
        let cur_mode = self.zero_page[0x41] & 0x7;
        self.zero_page[0x41] = (value & 0xf8) | cur_mode;
      } else if addr == 0xff44 {
        self.zero_page[0x44] = value;
        let lyc = self.zero_page[0x45];
        let coincidence = if lyc == value {
          0x4
        } else {
          0x0
        };
        self.zero_page[0x41] = (self.zero_page[0x41] & 0xfb) | coincidence;
      } else if addr == 0xff46 {
        // DMA transfer
        let src = (value as u16) << 8;
        for i in 0..0x9f {
          self.sprite_table[i as usize] = self.get_byte(src + i);
        }
      } else {
        self.zero_page[(addr - 0xff00) as usize] = value;
      }
      return;
    }
    if addr == 0xff00 {
      if value & 0b100000 == 0 {
        self.key_select = KeySelect::Buttons;
      } else if value & 0b10000 == 0 {
        self.key_select = KeySelect::Directions;
      }
      return;
    }
    if addr == 0xff04 {
      self.zero_page[0x04] = 0;
      return;
    }
    if addr == 0xff05 {
      self.zero_page[0x06] = value;
      return;
    }
    if addr == 0xff06 {
      self.zero_page[0x06] = value;
      return;
    }
    if addr == 0xff07 {
      self.zero_page[0x07] = value & 0x7;
      return;
    }

    if addr == 0xff10 {
      self.zero_page[0x10] = value;
      return;
    }

    if addr == 0xff14 {
      if value & 0x80 > 0 {
        // channel 1 enable
        let sweep_time = (value & 0x70) >> 4;
        let sweep_dir = (value & 0x8) >> 3;
        let sweep_shift = value & 0x7;

        let len = self.zero_page[0x11] & 0x3f;
        let freq = (((value & 0x7) as u32) << 8) + (self.zero_page[0x13] as u32);
        let nr12 = self.zero_page[0x12];
        let vol = (nr12 & 0xf0) >> 4;
        let vol_dir = (nr12 & 0x4) >> 3;
        let vol_len = nr12 & 0x7;
        self.audio.channel_1.sweep(sweep_time, sweep_dir, sweep_shift);
        self.audio.channel_1.reset(len, freq, vol, vol_dir, vol_len);
      }
      self.zero_page[0x14] = value;
      return;
    }

    if addr == 0xff19 {
      if value & 0x80 > 0 {
        // channel 2 enable
        let len = self.zero_page[0x16] & 0x3f;
        let freq = (((value & 0x7) as u32) << 8) + (self.zero_page[0x18] as u32);
        let nr22 = self.zero_page[0x17];
        let vol = (nr22 & 0xf0) >> 4;
        let vol_dir = (nr22 & 0x4) >> 3;
        let vol_len = nr22 & 0x7;
        self.audio.channel_2.reset(len, freq, vol, vol_dir, vol_len);
      }
      self.zero_page[0x19] = value;
      return;
    }

    if addr == 0xff20 {
      // channel 4 length
      self.zero_page[0x20] = value & 0x3f;
      return;
    }
    if addr == 0xff21 {
      self.zero_page[0x21] = value;
      return;
    }
    if addr == 0xff22 {
      // channel 4 polynomial counter
      return;
    }
    if addr == 0xff23 {
      if value & 0x80 > 0 {
        // channel 4 enable
        let len = self.zero_page[0x20];
        let nr42 = self.zero_page[0x21];
        let vol = (nr42 & 0xf0) >> 4;
        let vol_dir = (nr42 & 0x4) >> 3;
        let vol_len = nr42 & 0x7;
        self.audio.channel_4.reset(len, vol, vol_dir, vol_len);
      }
      self.zero_page[0x23] = value & 0xc0;
      return;
    }
    if addr == 0xff24 {
      // master volume
      let left = (value & 0x70) >> 4;
      let right = value & 0x7;
      unsafe { set_master_gain(left, right); }
      return;
    }
    if addr == 0xff25 {
      // sound to terminal
      //unsafe { set_sound_to_terminals(value); }
      self.zero_page[0x25] = value;
      return;
    }
    if addr == 0xff26 {
      // sound on/off
      let on_off = value & 0x80;
      if on_off > 0 {
        // enable sound
        self.zero_page[0x26] &= 0x80;
        unsafe { audio_enabled(1); }
      } else {
        self.zero_page[0x26] = 0;
        unsafe { audio_enabled(0); }
      }
      return;
    }

    self.zero_page[(addr - 0xff00) as usize] = value;
  }

  pub fn get_word(&self, addr: u16) -> u16 {
    let low = self.get_byte(addr) as u16;
    let high = self.get_byte(addr + 1) as u16;
    return (high << 8) + low;
  }

  pub fn set_word(&mut self, addr: u16, value: u16) {
    let low = (value & 0xff) as u8;
    let high = (value >> 8) as u8;
    self.set_byte(addr, low);
    self.set_byte(addr + 1, high);
  }

  pub fn boot_ptr(&mut self) -> *mut u8 {
    let ptr = &mut self.boot[0] as *mut u8;
    return ptr;
  }

  pub fn rom_ptr(&mut self) -> *mut u8 {
    return self.cart.rom_ptr();
  }

  pub fn vram_ptr(&mut self) -> *mut u8 {
    let ptr = &mut self.video_ram[0] as *mut u8;
    return ptr;
  }

  pub fn wram_ptr(&mut self) -> *mut u8 {
    let ptr = &mut self.work_ram[0] as *mut u8;
    return ptr;
  }

  pub fn external_ram_ptr(&mut self) -> *mut u8 {
    return self.cart.ram_ptr();
  }

  pub fn sprite_table_ptr(&mut self) -> *mut u8 {
    let ptr = &mut self.sprite_table[0] as *mut u8;
    return ptr;
  }

  pub fn zero_page_ptr(&mut self) -> *mut u8 {
    let ptr = &mut self.zero_page[0] as *mut u8;
    return ptr;
  }

  pub fn key_down_button(&mut self, mask: u8) {
    self.keys_buttons = self.keys_buttons & mask;
    self.zero_page[0x0f] = self.zero_page[0x0f] | 0x10;
  }

  pub fn key_up_button(&mut self, mask: u8) {
    self.keys_buttons = self.keys_buttons | mask;
    self.zero_page[0x0f] = self.zero_page[0x0f] | 0x10;
  }

  pub fn key_down_direction(&mut self, mask: u8) {
    self.keys_directions = self.keys_directions & mask;
    self.zero_page[0x0f] = self.zero_page[0x0f] | 0x10;
  }

  pub fn key_up_direction(&mut self, mask: u8) {
    self.keys_directions = self.keys_directions | mask;
    self.zero_page[0x0f] = self.zero_page[0x0f] | 0x10;
  }

  pub fn set_buttons(&mut self, buttons: u8) {
    self.keys_buttons = buttons;
    self.zero_page[0x0f] = self.zero_page[0x0f] | 0x10;
  }

  pub fn set_directions(&mut self, directions: u8) {
    self.keys_directions = directions;
    self.zero_page[0x0f] = self.zero_page[0x0f] | 0x10;
  }

  pub fn add_time(&mut self, time: u8) {
    let next_time = self.timer + (time as u16);
    let base_start = self.timer / 16;
    let base_end = next_time / 16;
    if base_end > base_start {
      if base_end / 16 > base_start / 16 {
        // Increment divider
        self.zero_page[0x04] += 1;
      }
      let control = self.zero_page[0x07];
      if control & 0x4 > 0 {
        let speed_mod = match control & 0x3 {
          0 => 64,
          1 => 1,
          2 => 4,
          _ => 16,
        };
        if base_end % speed_mod == 0 {
          let (counter, overflow) = self.zero_page[0x5].overflowing_add(1);
          if overflow {
            // Trigger INT 50
            self.zero_page[0x0f] |= 4;
            // Reset to Modulo
            self.zero_page[0x05] = self.zero_page[0x06];
          } else {
            self.zero_page[0x05] = counter;
          }
        }
      }
    }
    if next_time > 1024 {
      self.timer = next_time - 1024;
    } else {
      self.timer = next_time;
    }

    self.audio.add_time(time);
  }

  pub fn is_cart_ram_dirty(&mut self) -> bool {
    let dirty = self.cart_ram_dirty;
    self.cart_ram_dirty = false;
    return dirty;
  }

  pub fn is_tile_data_dirty(&mut self) -> bool {
    let dirty = self.tile_data_dirty;
    self.tile_data_dirty = false;
    return dirty;
  }

  pub fn is_tile_map_0_dirty(&mut self) -> bool {
    let dirty = self.tile_map_0_dirty;
    self.tile_map_0_dirty = false;
    return dirty;
  }

  pub fn is_tile_map_1_dirty(&mut self) -> bool {
    let dirty = self.tile_map_1_dirty;
    self.tile_map_1_dirty = false;
    return dirty;
  }
}

#[cfg(test)]
mod tests {
  use vm::memmap::create_memmap;

  #[test]
  fn get_word() {
    let mut mem = create_memmap(0);
    mem.set_byte(0x9003, 0xc);
    mem.set_byte(0x9004, 0x12);
    assert_eq!(mem.get_word(0x9003), 0x120c);
  }

  #[test]
  fn set_word() {
    let mut mem = create_memmap(0);
    mem.set_word(0x9045, 0xface);
    assert_eq!(mem.get_byte(0x9045), 0xce);
    assert_eq!(mem.get_byte(0x9046), 0xfa);
  }

  #[test]
  fn divider() {
    let mut mem = create_memmap(0);
    assert_eq!(mem.get_byte(0xff04), 0);
    mem.add_time(12);
    assert_eq!(mem.get_byte(0xff04), 0);
    mem.add_time(8);
    assert_eq!(mem.timer, 20);
    assert_eq!(mem.get_byte(0xff04), 0);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    mem.add_time(8);
    mem.add_time(12);
    assert_eq!(mem.timer, 252);
    assert_eq!(mem.get_byte(0xff04), 0);
    mem.add_time(8);
    assert_eq!(mem.timer, 260);
    assert_eq!(mem.get_byte(0xff04), 1);
    mem.set_byte(0xff04, 12);
    assert_eq!(mem.get_byte(0xff04), 0);
  }

  #[test]
  fn counter() {
    let mut mem = create_memmap(0);
    assert_eq!(mem.get_byte(0xff05), 0);
    mem.set_byte(0xff07, 0x5); // running, speed = 01
    mem.add_time(12);
    mem.add_time(8);
    assert_eq!(mem.timer, 20);
    assert_eq!(mem.get_byte(0xff05), 1);
    mem.add_time(16);
    assert_eq!(mem.get_byte(0xff05), 2);
    mem.set_byte(0xff07, 0x6); // running, speed = 10
    mem.add_time(16);
    assert_eq!(mem.get_byte(0xff05), 2);
    mem.add_time(16);
    assert_eq!(mem.get_byte(0xff05), 3);
    mem.add_time(16);
    mem.add_time(16);
    mem.add_time(16);
    mem.add_time(16);
    assert_eq!(mem.get_byte(0xff05), 4);

    mem.set_byte(0xff07, 0x5);
    for _ in 0..251 {
      mem.add_time(16);
    }
    assert_eq!(mem.get_byte(0xff05), 255);
    mem.set_byte(0xff06, 7);
    mem.add_time(16);
    // resets to modulo
    assert_eq!(mem.get_byte(0xff05), 7);
  }
}
