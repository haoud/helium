#![no_std]

use alloc::sync::Arc;
use macros::init;
use x86_64::paging::PageTableRoot;

extern crate alloc;

pub mod elf;
pub mod scheduler;
pub mod syscall;
pub mod task;

#[init]
pub fn setup() {
    // Load the init task
    let init = elf::load(
        Arc::new(PageTableRoot::new()),
        include_bytes!("../../../../iso/boot/init.elf"),
    )
    .expect("Failed to load init task");

    scheduler::setup();
    scheduler::add_task(init);
}

/// Enter userland. This function after the kernel has been initialized and jumps to the init
/// task to engage userland.
///
/// # Safety
/// This function is unsafe because why not ? More seriously, this function is unsafe simply
/// because it use pointer and assembly to jump to the init task.
pub unsafe fn enter_userland() -> ! {
    scheduler::engage_cpu();
}
