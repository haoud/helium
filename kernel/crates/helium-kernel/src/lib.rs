#![no_std]
#![feature(asm_const)]
#![feature(naked_functions)]

use macros::init;

pub mod exception;
pub mod irq;

/// The entry point of the kernel. It setups the logger, the x86_64 architecture dependent code and
/// the memory manager.
///
/// # Safety
/// Do I really have to explain why the entry point of the function that will initialize the kernel
/// in a baremetal environment is not safe ? Anything can go wrong here...
#[init]
pub unsafe fn setup() -> ! {
    // Initialize the x86_64 architecture dependent code that
    // does not need the memory manager
    x86_64::early_setup();

    // Install exception handlers
    exception::install();

    // Initialize the memory manager and the allocators
    mm::setup();

    // Initialize the x86_64 architecture dependent code that
    // needs the memory manager to be initialized first
    x86_64::setup();

    // Setup the irq
    irq::install();

    // Setup the userland environment
    user::setup();

    // Run the APs
    x86_64::smp::go();
    log::info!("Helium booted successfully !");

    user::enter_userland();
}
