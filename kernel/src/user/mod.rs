use crate::x86_64::{self, paging::PageTableRoot};
use alloc::sync::Arc;
use macros::init;

pub mod elf;
pub mod preempt;
pub mod scheduler;
pub mod task;

#[init]
pub fn setup() {
    // Load the init task
    let init = elf::load(
        Arc::new(PageTableRoot::new()),
        include_bytes!("../../../iso/boot/init.elf"),
    )
    .expect("Failed to load init task");

    scheduler::setup();
    scheduler::add_task(init);

    scheduler::add_task(
        elf::load(
            Arc::new(PageTableRoot::new()),
            include_bytes!("../../../iso/boot/init.elf"),
        )
        .expect("Failed to load init task"),
    );
    scheduler::add_task(
        elf::load(
            Arc::new(PageTableRoot::new()),
            include_bytes!("../../../iso/boot/init.elf"),
        )
        .expect("Failed to load init task"),
    );
    scheduler::add_task(
        elf::load(
            Arc::new(PageTableRoot::new()),
            include_bytes!("../../../iso/boot/init.elf"),
        )
        .expect("Failed to load init task"),
    );
    scheduler::add_task(
        elf::load(
            Arc::new(PageTableRoot::new()),
            include_bytes!("../../../iso/boot/init.elf"),
        )
        .expect("Failed to load init task"),
    );
    scheduler::add_task(
        elf::load(
            Arc::new(PageTableRoot::new()),
            include_bytes!("../../../iso/boot/init.elf"),
        )
        .expect("Failed to load init task"),
    );
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

pub fn idle() -> ! {
    loop {
        unsafe {
            x86_64::irq::enable();
            x86_64::irq::wait();
        }
    }
}
