#[cfg(not(test))]
extern "C" {
  fn set_channel_1_freq(f: u32);
  fn set_channel_1_gain(v: u8);
  fn set_channel_2_freq(f: u32);
  fn set_channel_2_gain(v: u8);
  fn set_channel_4_gain(v: u8);
}

#[cfg(test)]
fn set_channel_1_freq(f : u32) {}
#[cfg(test)]
fn set_channel_1_gain(v : u8) {}
#[cfg(test)]
fn set_channel_2_freq(f : u32) {}
#[cfg(test)]
fn set_channel_2_gain(v : u8) {}
#[cfg(test)]
fn set_channel_4_gain(v: u8) {}


pub struct SquareChannel {
  index: u8,

  freq: u32,
  volume: u8,
  volume_dir: u8,
  volume_counter: u8,
  counter: u8,
  time: u32,

  shadow_freq: u32,
  sweep_counter: u8,
  sweep_time: u8,
  sweep_dir: u8,
  sweep_shift: u8,
}

impl SquareChannel {
  pub fn sweep(&mut self, time: u8, dir: u8, shift: u8) {
    self.shadow_freq = self.freq;
    self.sweep_counter = 0;
    self.sweep_time = time;
    self.sweep_dir = dir;
    self.sweep_shift = shift;
  }

  pub fn reset(&mut self, len: u8, freq: u32, vol: u8, vol_dir: u8, vol_len: u8) {
    self.counter = 64 - (len & 0x3f);
    self.freq = freq;
    self.time = 0;
    self.volume = vol;
    self.volume_dir = vol_dir;
    self.volume_counter = vol_len;
    let f = 131072 / (2048 - freq);
    match self.index {
      0 => unsafe { set_channel_1_freq(f); set_channel_1_gain(vol); },
      1 => unsafe { set_channel_2_freq(f); set_channel_2_gain(vol); },
      _ => (),
    };
  }

  pub fn add_time(&mut self, t: u32) {
    let (next_time, overflow) = self.time.overflowing_add(t);
    if self.counter > 0 {
      // 256 Hz
      if overflow || ((next_time / 65536) > (self.time / 65536)) {
        self.counter -= 1;
        if self.counter <= 0 {
          self.counter = 0;
          match self.index {
            0 => unsafe { set_channel_1_gain(0); },
            1 => unsafe { set_channel_2_gain(0); },
            _ => (),
          };
        }
      }
    }

    if self.sweep_time > 0 && self.shadow_freq > 0 {
      // 128 Hz
      if overflow || ((next_time / 131072) > (self.time / 131072)) {
        let next_counter = self.sweep_counter + 1;
        if (next_counter / self.sweep_time) > (self.sweep_counter / self.sweep_time) {
          let delta = self.shadow_freq >> self.sweep_shift;
          if self.sweep_dir == 0 {
            self.shadow_freq -= delta;
          } else {
            self.shadow_freq += delta;
            if self.shadow_freq > 2047 {
              self.shadow_freq = 0;
            }
          }
          let f = 131072 / (2048 - self.shadow_freq);
          unsafe { set_channel_1_freq(f); }
        }
        self.sweep_counter = next_counter;
      }
    }

    if self.volume_counter > 0 {
      // 64 Hz
      if overflow || ((next_time / 262272) > (self.time / 262272)) {
        self.volume_counter -= 1;
        if self.volume_dir == 0 && self.volume > 0 {
          self.volume -= 1;
        } else if self.volume_dir == 1 && self.volume < 0xf {
          self.volume += 1;
        }
        match self.index {
          0 => unsafe { set_channel_1_gain(self.volume); },
          1 => unsafe { set_channel_2_gain(self.volume); },
          _ => (),
        };
      }
    }
    self.time = next_time;
  }
}

pub struct NoiseChannel {
  volume: u8,
  volume_dir: u8,
  volume_counter: u8,
  counter: u8,
  time: u32,
}

impl NoiseChannel {
  pub fn reset(&mut self, len: u8, vol: u8, vol_dir: u8, vol_len: u8) {
    self.counter = 64 - (len & 0x3f);
    self.time = 0;
    self.volume = vol;
    self.volume_dir = vol_dir;
    self.volume_counter = vol_len;
    unsafe { set_channel_4_gain(vol); }
  }

  pub fn add_time(&mut self, t: u32) {
    let (next_time, overflow) = self.time.overflowing_add(t);
    if self.counter > 0 {
      // 256 Hz
      if overflow || ((next_time / 65536) > (self.time / 65536)) {
        self.counter -= 1;
        if self.counter <= 0 {
          self.counter = 0;
          self.volume_counter = 0;
          unsafe { set_channel_4_gain(0); }
        }
      }
    }

    if self.volume_counter > 0 {
      // 64 Hz
      if overflow || ((next_time / 262272) > (self.time / 262272)) {
        self.volume_counter -= 1;
        if self.volume_dir == 0 && self.volume > 0 {
          self.volume -= 1;
        } else if self.volume_dir == 1 && self.volume < 0xf {
          self.volume += 1;
        }
        unsafe { set_channel_4_gain(self.volume); }
      }
    }
    self.time = next_time;
  }
}

pub struct Audio {
  pub channel_1: SquareChannel,
  pub channel_2: SquareChannel,
  pub channel_4: NoiseChannel,
}

impl Audio {
  pub fn add_time(&mut self, t: u8) {
    self.channel_1.add_time(t as u32);
    self.channel_2.add_time(t as u32);
    self.channel_4.add_time(t as u32);
  }
}

pub fn create_audio() -> Audio {
  return Audio {
    channel_1: SquareChannel {
      index: 0,
      volume: 0,
      volume_dir: 0,
      volume_counter: 0,
      counter: 0,
      time: 0,
      freq: 0,
      shadow_freq: 0,
      sweep_counter: 0,
      sweep_time: 0,
      sweep_dir: 0,
      sweep_shift: 0,
    },
    channel_2: SquareChannel {
      index: 1,
      volume: 0,
      volume_dir: 0,
      volume_counter: 0,
      counter: 0,
      time: 0,
      freq: 0,
      shadow_freq: 0,
      sweep_counter: 0,
      sweep_time: 0,
      sweep_dir: 0,
      sweep_shift: 0,
    },

    channel_4: NoiseChannel {
      volume: 0,
      volume_dir: 0,
      volume_counter: 0,
      counter: 0,
      time: 0,
    },
  };
}