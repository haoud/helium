#![no_std]
#![no_main]
#![allow(dead_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![feature(panic_info_message)]

#[cfg(not(target_arch = "x86_64"))]
compile_error!("Helium only supports x86_64 computers");

extern crate alloc;

use kernel::Stop;
use macros::init;

pub mod emulator;
pub mod glue;
pub mod kernel;
pub mod logger;

/// The entry point of the kernel. It setups the logger, the x86_64 architecture dependent code and
/// the memory manager.
///
/// # Safety
/// Do I really have to explain why the entry point of the function that will initialize the kernel
/// in a baremetal environment is not safe ? Anything can go wrong here...
#[init]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    // Initialize the logging system
    logger::setup();

    // Initialize the x86_64 architecture dependent code that
    // does not need the memory manager
    x86_64::early_setup();

    // Initialize the memory manager and the allocators
    mm::setup();

    // Initialize the x86_64 architecture dependent code that
    // needs the memory manager to be initialized first
    x86_64::setup();

    // Setup the userland environment
    user::setup();

    // Run the APs
    x86_64::smp::go();
    log::info!("Helium booted successfully !");

    user::enter_userland();

    // Stop the kernel
    kernel::stop(Stop::Success);
}
