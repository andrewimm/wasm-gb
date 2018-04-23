// This code is intentionally left unminified for your perusal
(function() {
const DMG_ROM = [ /* bootloader code is not supplied as part of this repo, but can be used */ ];

const SAVE_PERIOD = 10000;

function fetchAndInstantiate(url, importObject) {
  return fetch(url).then(
    res => res.arrayBuffer()
  ).then(
    bytes => WebAssembly.instantiate(bytes, importObject)
  ).then(
    results => results.instance
  );
}

function memcpy(memory, source, dest) {
  for (let i = 0; i < source.length; i++) {
    memory[dest + i] = source[i];
  }
}

const PATH = 'build/wasm_gb.wasm';

const writes = {};
window._writes = writes;

function loadWASM(vm) {
  return fetchAndInstantiate(PATH, {
    env: {
      update_registers: function() {},
      draw_gl: function() {
        vm.drawScreen();
      },
      copy_tile_data: function() {
        vm.copyTileData();
      },
      copy_map_0_data: function() {
        vm.copyMap0Data();
      },
      copy_map_1_data: function() {
        vm.copyMap1Data();
      },
      log_line: function(line) {
        console.log('l', line.toString(16));
      },
      log_addr: function(addr, value) {
        //console.log('addr', addr.toString(16), value.toString(16));
        writes[addr.toString(16)] = value.toString(16);
      },
      set_channel_1_freq: function(freq, volume) {
        vm.audio.channels[0].setFrequency(freq);
      },
      set_channel_1_gain: function(volume) {
        vm.audio.channels[0].setGain(volume / 15);
      },
      set_channel_2_freq: function(freq, volume) {
        vm.audio.channels[1].setFrequency(freq);
      },
      set_channel_2_gain: function(volume) {
        vm.audio.channels[1].setGain(volume / 15);
      },
      set_channel_4_gain: function(volume) {
        vm.audio.channels[3].setGain(volume / 15);
      },
      set_master_gain: function(left, right) {
        vm.audio.setMasterGain(left / 15, right / 15);
      },
      audio_enabled: function(flag) {
        vm.audio.enableAudio(!!flag);
      }
    },
  }).then(instance => {
    return {
      memory: instance.exports.memory,
      createVM: instance.exports.create_vm,
      getBootPointer: instance.exports.get_boot_pointer,
      getRomPointer: instance.exports.get_rom_pointer,
      getRamPointer: instance.exports.get_ram_pointer,
      getVRamPointer: instance.exports.get_vram_pointer,
      getSpriteTablePointer: instance.exports.get_sprite_table_pointer,
      getZeroPagePointer: instance.exports.get_zero_page_pointer,
      frame: instance.exports.frame,
      reset: instance.exports.reset,
      resetAfterBootloader: instance.exports.reset_after_bootloader,
      keyDown: instance.exports.key_down,
      keyUp: instance.exports.key_up,
      setButtons: instance.exports.set_buttons,
      setDirections: instance.exports.set_directions,
      setMBC: instance.exports.set_mbc,
      isSramDirty: instance.exports.is_sram_dirty,
    };
  });
}

class VM {
  constructor(gl) {
    this.audio = new Audio();
    this.graphics = new Graphics(gl);
    this.controls = new Controls();
    this.saveState = new SaveState(this);
    if ('VRDisplay' in window) {
      this.vr = new VR(gl);
      this.enterVR = document.getElementById('vr');
      this.enterVR.addEventListener('click', () => {
        const display = this.vr.state.display;
        if (!display) {
          return;
        }
        if (display.isPresenting) {
          // Exit VR
          display.exitPresent().then(() => {
            gl.canvas.style.width = '320px';
            gl.canvas.width = 320;
            gl.canvas.height = 288;
          });
        }
        display.requestPresent([{source: gl.canvas}]).then(() => {
          const leftParams = display.getEyeParameters('left');
          const rightParams = display.getEyeParameters('right');
          gl.canvas.style.width = '100%';
          gl.canvas.width = leftParams.renderWidth + rightParams.renderWidth;
          gl.canvas.height = Math.min(leftParams.renderHeight, rightParams.renderHeight);
        }).catch(err => {
          console.error(err);
        });
      });
    }
    this.mod = null;
    this.mem = null;
    this.gb = null;
    this._lastSave = 0;

    this._ready = loadWASM(this);
    this._ready.then(mod => {
      this.mod = mod;

      this.gb = mod.createVM();
      const mem = {
        bootPtr: mod.getBootPointer(this.gb),
        romPtr: mod.getRomPointer(this.gb),
        ramPtr: mod.getRamPointer(this.gb),
        vramPtr: mod.getVRamPointer(this.gb),
        spriteTablePtr: mod.getSpriteTablePointer(this.gb),
        zeroPagePtr: mod.getZeroPagePointer(this.gb),
      };
      const buffer = mod.memory.buffer;
      mem.boot = new Uint8Array(buffer, mem.bootPtr, 0x100);
      mem.rom = new Uint8Array(buffer, mem.romPtr, 0x200000);
      mem.ram = new Uint8Array(buffer, mem.ramPtr, 0x8000);
      mem.vram = new Uint8Array(buffer, mem.vramPtr, 0x2000);
      mem.spriteTable = new Uint8Array(buffer, mem.spriteTablePtr, 0xa0);
      mem.zeroPage = new Uint8Array(buffer, mem.zeroPagePtr, 0x100);
      this.mem = mem;

      this.vramWindows = {
        bgTile0: new Uint8Array(buffer, mem.vramPtr + 0x1800, 1024),
        bgTile1: new Uint8Array(buffer, mem.vramPtr + 0x1c00, 1024),
        tileData: new Uint8Array(buffer, mem.vramPtr, 16 * 384),
      };
    });

    this.frame = this.frame.bind(this);
  }

  frame(ms) {
    // Periodic SRAM save
    if (this._lastSave === 0) {
      this._lastSave = ms;
    }

    if (ms - this._lastSave > SAVE_PERIOD) {
      this._lastSave = ms;
      if (this.mod.isSramDirty(this.gb)) {
        this.saveState.save().catch(err => {
          console.error('Attempted to save SRAM:', err);
        });
      }
    }

    const buttons = this.controls.getButtons();
    const directions = this.controls.getDirections();
    if (buttons !== null) {
      this.mod.setButtons(this.gb, buttons);
    }
    if (directions !== null) {
      this.mod.setDirections(this.gb, directions);
    }

    const state = this.mod.frame(this.gb);
    let inVR = false;

    if (this.vr && this.vr.active) {
      inVR = true;
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      this.vr.draw();
    }
    if (state === 0) {
      this._raf = inVR ? this.vr.state.display.requestAnimationFrame(this.frame) : requestAnimationFrame(this.frame);
    } else {
      if (state === 1) {
        alert('Unexpected Crash!');
      }
      this._raf = null;
      this._playing = false;
    }
  }

  copyTileData() {
    if (this.vramWindows) {
      this.graphics.loadTileData(this.vramWindows.tileData);
    }
  }

  copyMap0Data() {
    if (this.vramWindows) {
      this.graphics.loadBGMap0(this.vramWindows.bgTile0);
    }
  }

  copyMap1Data() {
    if (this.vramWindows) {
      this.graphics.loadBGMap1(this.vramWindows.bgTile1);
    }
  }

  drawScreen() {
    if (this.vr && this.vr.state.isPresenting()) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.vr.fb);
    } else {
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    }
    if (this.vramWindows) {
      this.graphics.loadBGTiles(this.vramWindows.bgTile0, this.vramWindows.bgTile1);
      this.graphics.loadOAMData(this.mem.spriteTable);
      this.graphics.draw(this.mem.spriteTable, this.mem.zeroPage);
    }
  }

  ready() {
    return this._ready;
  }

  reset(rom) {
    if (this._playing) {
      this.pause();
    }
    if (DMG_ROM.length > 0) {
      this.mod.reset();
      memcpy(this.mem.boot, DMG_ROM, 0);
    } else {
      this.mod.resetAfterBootloader(this.gb);
    }
    if (rom) {
      memcpy(this.mem.rom, rom, 0);
      // Extract the MBC ID
      const mbc = this.mem.rom[0x147];
      this.mod.setMBC(this.gb, mbc);
      this.saveState.load().then(() => {
        console.log('Loaded save state from IndexedDB');
        this.play();
      }, e => {
        console.log('No save file exists for this ROM');
        this.play();
      });
    } else {
      memcpy(this.mem.rom, ROM_HEAD, 0x100);
    }
  }

  play() {
    if (this._playing) {
      return;
    }
    this.audio.play();
    this._playing = true;
    if (this.vr) {
      if (this.vr.state.display) {
        this.enterVR.style.display = 'block';
      }
      this.vr.state.onDisplayChange(function(disp) {
        if (disp) {
          this.enterVR.style.display = 'block';
        }
      });
    }
    this._raf = requestAnimationFrame(this.frame);
  }

  pause() {

  }
}

window.VM = VM;
})();