use vm::memmap::MemMap;

#[derive(Debug, PartialEq)]
pub enum GPUAction {
  Noop,
  RenderScanline(u8),
  IncrementLine(u8),
  FlushBuffer, // implies line 143
}

enum GPUMode {
  Mode0,
  Mode1,
  Mode2,
  Mode3,
}

pub struct GPU {
  mode: GPUMode,
  time: u16,
  line: u8,
}

pub fn create_gpu() -> GPU {
  return GPU {
    mode: GPUMode::Mode2,
    time: 0,
    line: 0,
  };
}

impl GPU {
  pub fn get_line(&self) -> u8 {
    return self.line;
  }

  pub fn add_clock_time(&mut self, mem: &mut MemMap, time: u8) -> GPUAction {
    self.time += time as u16;
    return match self.mode {
      GPUMode::Mode2 => {
        if self.time >= 80 {
          self.time = 0;
          self.mode = GPUMode::Mode3;
          mem.zero_page[0x41] = mem.zero_page[0x41] | 3;
        }
        GPUAction::Noop
      },

      GPUMode::Mode3 => {
        if self.time >= 172 {
          self.time = 0;
          self.mode = GPUMode::Mode0;
          mem.zero_page[0x41] = mem.zero_page[0x41] & 0xfc;
          GPUAction::RenderScanline(self.line)
        } else {
          GPUAction::Noop
        }
      },

      GPUMode::Mode0 => {
        if self.time >= 204 {
          self.time = 0;
          self.line += 1;
          if self.line >= 143 {
            self.mode = GPUMode::Mode1;
            mem.zero_page[0x41] = (mem.zero_page[0x41] & 0xfc) | 1;
            // Enable vblank interrupt
            mem.zero_page[0x0f] = mem.zero_page[0x0f] | 1;
            GPUAction::FlushBuffer
          } else {
            self.mode = GPUMode::Mode2;
            mem.zero_page[0x41] = (mem.zero_page[0x41] & 0xfc) | 2;
            GPUAction::IncrementLine(self.line)
          }
        } else {
          GPUAction::Noop
        }
      },

      GPUMode::Mode1 => {
        if self.time >= 456 {
          self.time = 0;
          self.line += 1;
          if self.line > 153 {
            self.line = 0;
            self.mode = GPUMode::Mode2;
            mem.zero_page[0x41] = (mem.zero_page[0x41] & 0xfc) | 2;
            mem.zero_page[0x0f] = mem.zero_page[0x0f] & 0xfe;
          }
          GPUAction::IncrementLine(self.line)
        } else {
          GPUAction::Noop
        }
      },
    }
  }
}