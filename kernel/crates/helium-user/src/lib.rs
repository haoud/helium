#![no_std]

use alloc::sync::Arc;
use macros::init;
use task::Task;
use x86_64::{paging::PageTableRoot, thread};

extern crate alloc;

pub mod elf;
pub mod syscall;
pub mod task;

#[init]
pub fn setup() {}

pub fn enter_userland() {
    let mm = Arc::new(PageTableRoot::new());
    let entry = elf::load(&mm, include_bytes!("../../../../iso/boot/init.elf"));
    let init = Task::new(mm, entry);
    thread::jump_to_thread(&mut init.thread().lock());
}
