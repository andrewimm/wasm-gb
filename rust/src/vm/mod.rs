pub mod audio;
pub mod cart;
pub mod cpu;
pub mod gpu;
pub mod memmap;

extern "C" {
  fn copy_tile_data();
  fn copy_map_0_data();
  fn copy_map_1_data();
  fn draw_gl();
}

pub struct VM {
  pub cpu: cpu::CPU,
  pub gpu: gpu::GPU,
  pub mem: memmap::MemMap,
  pub state: cpu::RunState,

  pub breakpoints: Vec<u16>,
}

impl VM {
pub fn step(&mut self) {
  let (state, _) = self.cpu.step(&mut self.mem);
  self.state = state;
}

pub fn frame(&mut self) -> bool {
  let mut cpu_state = self.state;
  let mut cycles = 4;
  let mut gpu_action = gpu::GPUAction::Noop;
  let mut breakpoint = false;
  while gpu_action != gpu::GPUAction::FlushBuffer {
    if !breakpoint && cpu_state == cpu::RunState::Run {
      let (s, c) = self.cpu.step(&mut self.mem);
      cpu_state = s;
      cycles = c;
    }

    if self.breakpoints.contains(&self.cpu.get_register_16(cpu::Register16::PC)) {
      breakpoint = true;
    }

    self.state = cpu_state;

    let time = cycles;

    gpu_action = self.gpu.add_clock_time(&mut self.mem, time);
    self.mem.add_time(time);

    match gpu_action {
      gpu::GPUAction::RenderScanline(_line) => {
        // Handled in WebGL now
      },

      gpu::GPUAction::IncrementLine(line) => {
        self.mem.set_byte(0xff44, line);
      },

      gpu::GPUAction::FlushBuffer => {
        self.mem.set_byte(0xff44, 143);
        unsafe {
          if self.mem.is_tile_data_dirty() {
            copy_tile_data();
          }
          if self.mem.is_tile_map_0_dirty() {
            copy_map_0_data();
          }
          if self.mem.is_tile_map_1_dirty() {
            copy_map_1_data();
          }
          draw_gl();
        }
      },

      _ => {},
    }

    if !breakpoint {
      if self.cpu.interrupt_enabled() {
        let i_enabled = self.mem.get_byte(0xffff);
        let i_fired = self.mem.get_byte(0xff0f);
        let mut i_fired_reset = i_fired;
        let fired = i_enabled & i_fired;
        if fired > 0 {
          if self.state != cpu::RunState::Run {
            self.state = cpu::RunState::Run;
          }
          // Perform triggered interrupts
          if fired & 1 > 0 {
            // VBlank
            i_fired_reset = i_fired_reset & 0xfe;
            self.mem.set_byte(0xff0f, i_fired_reset);
            self.cpu.int_vblank(&mut self.mem);
          }
          if fired & 2 > 0 {
            // LCD STAT
            i_fired_reset = i_fired_reset & 0xfd;
            self.mem.set_byte(0xff0f, i_fired_reset);
            self.cpu.int_stat(&mut self.mem);
          }
          if fired & 4 > 0 {
            // Timer
            i_fired_reset = i_fired_reset & 0xfb;
            self.mem.set_byte(0xff0f, i_fired_reset);
            self.cpu.int_timer(&mut self.mem);
          }
          if fired & 8 > 0 {
            // Serial
          }
          if fired & 16 > 0 {
            // Joypad
            i_fired_reset = i_fired_reset & 0xef;
            self.mem.set_byte(0xff0f, i_fired_reset);
            self.cpu.int_joypad(&mut self.mem);
          }
        }
      }
    }
  }
  return breakpoint;
}

pub fn set_mbc(&mut self, mbc: u8) {
  self.mem.cart.set_mbc(mbc);
}
}