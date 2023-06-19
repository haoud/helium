#![no_std]
#![no_main]
#![allow(dead_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![feature(panic_info_message)]

#[cfg(not(target_arch = "x86_64"))]
compile_error!("Helium only supports x86_64 computers");

extern crate alloc;

use macros::init;

/// A enum that represents the stopping reason of the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stop {
    Success = 1,
    Failure = 2,
}

/// Stop the execution of the kernel. Depending on the features flags, it either closes the
/// emulator or freezes the CPU. This should be used when the kernel can't continue its execution,
/// or when the kernel has finished its execution.
///
/// # Safety
/// This function is unsafe because depending on some features flags, it either closes the emulator
/// or freezes the CPU, which could result in undefined behavior if the kernel is not running in
/// QEMU.
#[allow(unused_variables)]
pub unsafe fn stop(code: Stop) -> ! {
    cfg_if::cfg_if! {
        if #[cfg(feature = "test")] {
            crate::emulator::qemu::exit(code as u32);
        }
    }
    x86_64::cpu::freeze();
}

pub mod emulator;
pub mod glue;
pub mod logger;

/// The entry point of the kernel. It setups the logger and the kernel.
///
/// # Safety
/// Do I really have to explain why the entry point of the function that will initialize the kernel
/// in a baremetal environment is not safe ? Anything can go wrong here...
#[init]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    // Initialize the logging system
    logger::setup();

    // Initialize the kernel and jump into the userland
    kernel::setup();
}
