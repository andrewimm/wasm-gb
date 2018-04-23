# wasm-gb

An experimental Game Boy emulator built with WebAssembly and WebGL 2.0

## About

This project is an exploration into the architecture of an non-trivial web
application built with Rust and WebAssembly. Nearly all Game Boy functionality
has been written in Rust, leveraging the ability to compile directly to
WebAssembly through LLVM. One exception is the graphics unit, which is
implemented as a series of WebGL 2.0 shaders, leveraging the latest abilities to
to create data textures from linear byte arrays.

While the emulator is capable of playing many games, it is still very much a
work in progress and should be considered an educational project rather than a
completed application.

**Please note** – because this project relies on WebGL 2.0, it will not run on iOS
devices. At the time of writing, only Firefox, Chrome, and Chrome for Android
implement the WebGL 2.0 spec. You can track support here: https://caniuse.com/#feat=webgl2

## Status

### Features

 - Implements most Game Boy functionality, including audio generation. Many
   popular games run, at least for a while.
 - Supports input via keyboard, touchscreen, or gamepads.
 - Stores save states locally via IndexedDB, so you can save your game and
   resume later on.
 - **Play in VR!** If you have a Rift / Vive and run the emulator on Firefox,
   you can play your games on a giant screen in VR.

### Remaining issues

**Unimplemented Features:**

 - Sprite priorities are unimplemented, backgrounds will not appear over sprites
   in some cases.
 - Audio channel 3 (sample playback) is unimplemented
 - Audio channel 4 (white noise) is missing the envelope function, so the pitch
   of all white noise will sound the same
 - LCD STAT interrupt is unimplemented
 - Some MBC variants remain unimplemented

**Other:**

 - CPU passes most, but not all, blargg test ROMs
 - There are definitely still some memory bugs, games have been known to crash

## Building

Compilation to WASM relies on the latest nightly version of the Rust toolchain.
You can install this by first installing the `rustup` tool, and then running

```
rustup toolchain install nightly
```

Then, install the `wasm32-unknown-unknown` build target with 

```
rustup target add wasm32-unknown-unknown --toolchain nightly
```

With the toolchain in place, you can build the Rust side by running `make`. This
creates a WASM file and moves it to the build folder. Once that's compiled, run
a local web server in the root directory, and open up `index.html` to run the
emulator.

## Design

The emulator is designed to keep as much functionality as possible in the Rust /
WebAssembly codebase. This helps avoid bridging back and forth between JS and
WASM, and theoretically makes the majority of the emulator portable to native
build targets.

### CPU

The Game Boy CPU is a variant of the Zilog Z80. This functionality is mostly
contained in `cpu.rs`, which implements a struct holding the CPU registers, as
well as a `step` function that executes one instruction. That function is
essentially a giant pattern-matcher that performs different operations based on
the current instruction.

Because instruction timing is encoded into the CPU, all other clocks derive from
the CPU's timing. Audio and graphics functionality will run proportional to the
frequency with which the CPU steps through the program.

### Memory

The Game Boy's memory map is split into three sections: on-device ROM/RAM,
on-cartridge ROM/RAM, and memory-mapped I/O. All memory reads and writes are
handled in `memmap.rs`, which implements a struct to store all of this data. It
not only contains linear memory for things like device RAM, it also triggers
side-effects based on specific reads or writes to simulate hardware behavior.

### Graphics

The Game Boy has four graphics layers: two background tile maps, an overlaid
"window" layer that can be used for HUD-type graphics, and a series of sprites
that can be moved anywhere on the screen. In true Game Boy hardware, the CPU
writes values to specific locations in Video RAM that are interpreted by the PPU
(Picture Processing Unit) to create graphics on the screen.

I have tried to simulate this behavior as closely as possible with a graphics
component that converts raw RAM values into the picture seen on the screen. This
rendering is entirely handled by WebGL 2.0 shaders, leveraging the abilities of
new data textures. By copying video memory directly into data textures, the
shader can reference raw memory values in order to build the background maps and
sprites.

In a single graphics pass, first the background is drawn as a single
screen-sized quad. Next, the window layer is (optionally) drawn as another
quad. Finally the sprites are draw, each containing its own quad. The texturing
for all of these is computed on-the-fly by performing lookups in the data
textures filled from memory.

### Audio

The Game Boy implements four audio channels: two backed by oscillators, one
that plays back wave samples, and one that generates psuedo-noise. Most of these
channels have additional timer behavior, allowing pitch or volume envelopes to
change, or playback to be stopped after a specific period of time.

Simulation of the Game Boy's audio hardware is implemented in three pieces. A
game accesses the hardware by writing values to specific memory-mapped I/O
addresses. When these are accessed in the emulator, values are passed to an
inner audio control system that handles timer behavior. The actual audio is
generated on the JS side using an AudioContext. Whenever audio pitch or volume
needs to be adjusted, a message is sent to JS to modify one of the oscillator
sources. This is probably the most explicit WASM<->JS orchestration found in the
entire emulator.

### Input

The Game Boy has eight buttons: four cardinal directions, as well as A, B,
Start, and Select. On true Game Boy hardware, these can be queried at any point
in time. In our emulator, we update button state once per frame, which is fast
enough. Since WebAssembly can't attach any event listeners or reference browser
APIs, button states need to be passed to the emulator. At the beginning of each
frame, the current button state is collected from all the input methods:
keyboard, on-screen buttons, and connected Gamepads, and sent to WebAssembly.

### Saving

On a periodic timer, the emulator checks if the game's save-RAM is dirty. If it
has been written to since the last time it was checked, that means the game
attempted to save some data to the cartridge. This data is stored locally in the
browser in IndexedDB storage, with one entry per game. With the right UI, this
could be extended to support multiple cartridge saves, as well as save states
that store the entire device memory and allow you to pick up exactly where you
left off.

---

The emulator found in this repository is entirely my own work, and does not
contain any ROMs or system bootloader code. Instead, it is configured to run
without the bootloader code found in original Game Boy hardware.

I am providing code in the repository to you under an open source license.
Because this is my personal repository,the license you receive to my code is
from me and not my employer (Facebook).
