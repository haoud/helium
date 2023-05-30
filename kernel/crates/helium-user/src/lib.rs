#![no_std]

use alloc::sync::Arc;
use macros::init;
use task::Identifier;
use x86_64::{paging::PageTableRoot, thread};

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
    );

    scheduler::setup();
    scheduler::add_task(init);
}

pub fn enter_userland() {
    let init = scheduler::task(Identifier::new(1)).expect("Init task not found");
    thread::jump_to_thread(&mut init.thread().lock());
}
