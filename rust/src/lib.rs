#![feature(box_syntax)]

mod vm;

use std::mem;
use vm::VM;

extern "C" {
  fn update_registers(a: u8, b: u8, c: u8, d: u8, e: u8, h: u8, l: u8, flags: u8, sp: u16, pc: u16);
}

#[no_mangle]
pub fn create_vm() -> *mut VM {
  let vm = VM {
    cpu: vm::cpu::create_cpu(),
    gpu: vm::gpu::create_gpu(),
    mem: vm::memmap::create_memmap(0),
    state: vm::cpu::RunState::Run,
    breakpoints: vec![0x100],
  };
  let b = Box::new(vm);
  return Box::into_raw(b);
}

#[no_mangle]
pub fn reset(raw: *mut VM) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    vm.cpu.reset();
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn reset_after_bootloader(raw: *mut VM) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    vm.cpu.reset();
    vm.cpu.simulate_bootloader();
    vm.mem.simulate_bootloader();
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn get_register(raw: *mut VM, reg: char) -> u8 {
  unsafe {
    let vm = Box::from_raw(raw);
    let ret = match reg {
      'a' => vm.cpu.get_register_8(vm::cpu::Register8::A),
      _ => 0,
    };
    mem::forget(vm);
    return ret;
  }
}

#[no_mangle]
pub fn step(raw: *mut VM) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    vm.step();
    update_registers(
      vm.cpu.get_register_8(vm::cpu::Register8::A),
      vm.cpu.get_register_8(vm::cpu::Register8::B),
      vm.cpu.get_register_8(vm::cpu::Register8::C),
      vm.cpu.get_register_8(vm::cpu::Register8::D),
      vm.cpu.get_register_8(vm::cpu::Register8::E),
      vm.cpu.get_register_8(vm::cpu::Register8::H),
      vm.cpu.get_register_8(vm::cpu::Register8::L),
      vm.cpu.get_register_8(vm::cpu::Register8::Flags),
      vm.cpu.get_register_16(vm::cpu::Register16::SP),
      vm.cpu.get_register_16(vm::cpu::Register16::PC),
    );
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn frame(raw: *mut VM) -> i32 {
  let mut state = 0;
  unsafe {
    let mut vm = Box::from_raw(raw);
    let breakpoint = vm.frame();
    if vm.state != vm::cpu::RunState::Run {
      state = match vm.state {
        vm::cpu::RunState::Crash => 1,
        vm::cpu::RunState::Halt => 0,
        vm::cpu::RunState::Stop => 0,
        _ => state,
      };
    } else if breakpoint {
      state = 4;
    }
    update_registers(
      vm.cpu.get_register_8(vm::cpu::Register8::A),
      vm.cpu.get_register_8(vm::cpu::Register8::B),
      vm.cpu.get_register_8(vm::cpu::Register8::C),
      vm.cpu.get_register_8(vm::cpu::Register8::D),
      vm.cpu.get_register_8(vm::cpu::Register8::E),
      vm.cpu.get_register_8(vm::cpu::Register8::H),
      vm.cpu.get_register_8(vm::cpu::Register8::L),
      vm.cpu.get_register_8(vm::cpu::Register8::Flags),
      vm.cpu.get_register_16(vm::cpu::Register16::SP),
      vm.cpu.get_register_16(vm::cpu::Register16::PC),
    );
    mem::forget(vm);
  }
  return state;
}

#[no_mangle]
pub fn get_boot_pointer(raw: *mut VM) -> *mut u8 {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let ptr = vm.mem.boot_ptr();
    mem::forget(vm);
    return ptr;
  }
}

#[no_mangle]
pub fn get_rom_pointer(raw: *mut VM) -> *mut u8 {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let ptr = vm.mem.rom_ptr();
    mem::forget(vm);
    return ptr;
  }
}

#[no_mangle]
pub fn get_ram_pointer(raw: *mut VM) -> *mut u8 {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let ptr = vm.mem.external_ram_ptr();
    mem::forget(vm);
    return ptr;
  }
}

#[no_mangle]
pub fn get_vram_pointer(raw: *mut VM) -> *mut u8 {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let ptr = vm.mem.vram_ptr();
    mem::forget(vm);
    return ptr;
  }
}

#[no_mangle]
pub fn get_sprite_table_pointer(raw: *mut VM) -> *mut u8 {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let ptr = vm.mem.sprite_table_ptr();
    mem::forget(vm);
    return ptr;
  }
}

#[no_mangle]
pub fn get_zero_page_pointer(raw: *mut VM) -> *mut u8 {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let ptr = vm.mem.zero_page_ptr();
    mem::forget(vm);
    return ptr;
  }
}

#[no_mangle]
pub fn read_mem(raw: *mut VM, addr: u16) -> u8 {
  unsafe {
    let vm = Box::from_raw(raw);
    let value = vm.mem.get_byte(addr);
    mem::forget(vm);
    return value;
  }
}

#[no_mangle]
pub fn set_breakpoint(raw: *mut VM, addr: u16) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    vm.breakpoints.push(addr);
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn clear_breakpoint(raw: *mut VM, addr: u16) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let index = vm.breakpoints.iter().position(|&x| x == addr);
    match index {
      Some(n) => vm.breakpoints.remove(n),
      None => 0,
    };
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn key_down(raw: *mut VM, btn: u8) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    match btn {
      0 => vm.mem.key_down_button(0xe),
      1 => vm.mem.key_down_button(0xd),
      2 => vm.mem.key_down_button(0xb),
      3 => vm.mem.key_down_button(0x7),
      4 => vm.mem.key_down_direction(0xe),
      5 => vm.mem.key_down_direction(0xd),
      6 => vm.mem.key_down_direction(0xb),
      7 => vm.mem.key_down_direction(0x7),
      _ => (),
    };
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn key_up(raw: *mut VM, btn: u8) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    match btn {
      0 => vm.mem.key_up_button(0x1),
      1 => vm.mem.key_up_button(0x2),
      2 => vm.mem.key_up_button(0x4),
      3 => vm.mem.key_up_button(0x8),
      4 => vm.mem.key_up_direction(0x1),
      5 => vm.mem.key_up_direction(0x2),
      6 => vm.mem.key_up_direction(0x4),
      7 => vm.mem.key_up_direction(0x8),
      _ => (),
    };
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn set_buttons(raw: *mut VM, buttons: u8) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    vm.mem.set_buttons(buttons & 0xf);
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn set_directions(raw: *mut VM, directions: u8) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    vm.mem.set_directions(directions & 0xf);
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn set_mbc(raw: *mut VM, mbc: u8) {
  unsafe {
    let mut vm = Box::from_raw(raw);
    vm.set_mbc(mbc);
    mem::forget(vm);
  }
}

#[no_mangle]
pub fn is_sram_dirty(raw: *mut VM) -> u8 {
  unsafe {
    let mut vm = Box::from_raw(raw);
    let value = if vm.mem.is_cart_ram_dirty() { 1 } else { 0 };
    mem::forget(vm);
    return value;
  }
}
