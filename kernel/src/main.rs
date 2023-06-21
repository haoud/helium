#![no_std]
#![no_main]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(dead_code)]
#![allow(clippy::unreadable_literal)]
#![feature(asm_const)]
#![feature(step_trait)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(panic_info_message)]

use macros::init;

extern crate alloc;

#[cfg(not(target_arch = "x86_64"))]
compile_error!("Helium only supports x86_64 computers");

pub mod arch;
pub mod logger;
pub mod mm;
pub mod panic;
pub mod syscall;
pub mod user;

#[init]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    // Initialize the logging system
    logger::setup();

    // Initialize the necessary x86_64 stuff that does not need the memory
    // manager to be initialized
    arch::early_setup();

    // Initialize the memory manager and the allocators
    mm::setup();

    // Initialize the x86_64 architecture dependent code that
    // needs the memory manager to be initialized first
    arch::setup();

    // Setup the userland environment
    user::setup();

    // Run the APs
    arch::smp::go();

    // Jump to userland
    log::info!("Helium booted successfully !");
    user::enter_userland();
}

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
            qemu::exit(code as u32);
        }
    }
    arch::cpu::freeze();
}
