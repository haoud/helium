//! TODO:
//!  - Modify the indentation of all files
//!  - Remove some useless abstractions
//!  - Remove all the VFS code
//!  - Add Seqlock to the kernel
//!  - Create a syscall crate to avoid code duplication
//!  - Better timer system
//!     + Maybe use async tasks ? Could be really nice
//!  - Better syscall return code
//!     * Maybe two registers for returning code ? Or maybe a struct ?
//!     * Unified return code for all syscalls to simplify the code
//! - Use `core::Duration` instead of our messy time units
//! - Use the zerocopy/bytemuck crate for the conversion between userland
//!   and kernel
//! - helium-addr:
//!     * Remove the `UserVirtual` and remplace it by `Virtual<User>` and
//!       `Virtual<Kernel>`, this will allow to have a better separation
//!       between the userland and the kernel and remove all range of
//!       problems with the canonical hole.
//!     * Implement differents frames sizes (4K, 2M, 1G)
//!  - Async support in the kernel
//!     * Simple executor with priority queues
//!     * Some futures utilities (yield, sleep, mutex...)
//!     * Use an event loop to handle user programs:
//!         + How to handle interrupts inside the kernel ?
//!         + How to reduce the stack consumption ?
//!     * Use the executor to remove the ugly and unsafe scheduler
//!
//! Unsure:
//! - Remove the prelude unstable feature ?
#![no_std]
#![no_main]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(dead_code)]
#![allow(internal_features)]
#![allow(clippy::match_bool)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::module_name_repetitions)]
#![feature(asm_const)]
#![feature(new_uninit)]
#![feature(step_trait)]
#![feature(extract_if)]
#![feature(const_option)]
#![feature(prelude_import)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(btree_extract_if)]
#![feature(panic_info_message)]

extern crate alloc;

#[cfg(not(target_arch = "x86_64"))]
compile_error!("Helium only supports x86_64 computers");

pub mod config;
pub mod device;
pub mod limine;
pub mod logger;
pub mod mm;
pub mod module;
pub mod panic;
pub mod qemu;
pub mod syscall;
pub mod time;
pub mod user;
pub mod x86_64;

/// The prelude of the kernel. It re-exports the prelude of the core standard
/// library and some imports that are often used in the kernel, allowing to use
/// them without having to import them in each file and improving the 
/// readability of the code (at least a little bit).
#[rustfmt::skip]
pub mod prelude {
    pub use core::prelude::rust_2021::*;
    pub use alloc::string::{String, ToString};
    pub use alloc::boxed::Box;
    pub use alloc::sync::Arc;
    pub use alloc::vec::Vec;
    pub use macros::*;
    pub use sync::*;
}

#[prelude_import]
pub use prelude::*;

/// # The entry point of the kernel. Initialises the kernel and jumps to
/// userland.
///
/// # Safety
/// This function is highly unsafe because we are in a minimal environment and
/// we have to initialize a lot of things before we can do anything. Since we
/// are in a bare metal environment, a lot of initialization code is written
/// in assembly or need the use of `unsafe` code to work properly, this is an
/// necessary evil.
#[init]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    // Initialize the logging system
    logger::setup();

    // Initialize the necessary x86_64 stuff that does not need the memory
    // manager to be initialized
    x86_64::early_setup();

    // Initialize the memory manager and the allocators
    mm::setup();

    // Initialize the x86_64 architecture dependent code that
    // needs the memory manager to be initialized first
    x86_64::setup();

    // Initialize date and time
    time::date::setup();

    // Initialize dynamic timers
    time::timer::setup();

    // Initialize the module system
    module::setup();

    // Setup the userland environment
    user::setup();

    // Run the APs
    x86_64::smp::go();

    // Terminate the setup and jump to userland
    terminate_setup();
}

/// Reclaim the memory only used during the boot process and jump to userland
/// (or run the integration tests depending on the `test` feature flag).
///
/// # Safety
/// This function is unsafe because it reclaim the memory used by the boot
/// process, which could result in undefined behavior if the memory is still
/// used (see the [`mm::reclaim_boot_memory`] function for more details).
#[inline(never)]
pub unsafe fn terminate_setup() -> ! {
    //mm::reclaim_boot_memory();

    // Jump to userland or run the tests depending on the features flags
    cfg_if::cfg_if! {
        if #[cfg(feature = "test")] {
            // TODO: Run the tests
            stop(Stop::Success);
        } else {
            log::info!("Helium booted successfully !");
            user::enter_userland();
        }
    }
}

/// A enum that represents the stopping reason of the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stop {
    Success = 1,
    Failure = 2,
}

/// Stop the execution of the kernel. Depending on the features flags, it
/// either closes the emulator or freezes the current CPU. This should be
/// used when the kernel can't continue its execution or when the kernel
/// has finished its execution during the tests.
///
/// # Safety
/// This function is unsafe because depending on some features flags, it
/// either closes the emulator or freezes the CPU, which could result in
/// undefined behavior if the kernel is not running in QEMU.
#[allow(unused_variables)]
pub unsafe fn stop(code: Stop) -> ! {
    cfg_if::cfg_if! {
        if #[cfg(feature = "test")] {
            qemu::exit(code as u32);
        } else {
            x86_64::cpu::freeze();
        }
    }
}
