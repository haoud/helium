use crate::x86_64::{self, paging::PageTableRoot};
use alloc::sync::Arc;
use macros::init;

pub mod elf;
pub mod preempt;
pub mod ptr;
pub mod scheduler;
pub mod string;
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

    // Load 10 init tasks for testing
    for _ in 0..10 {
        scheduler::add_task(
            elf::load(
                Arc::new(PageTableRoot::new()),
                include_bytes!("../../../iso/boot/init.elf"),
            )
            .expect("Failed to load init task"),
        );
    }
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

/// The idle function. This function will be used by the idle kernel task when there is no other
/// task to run. This function is a simple loop that simply put the CPU in a low power state until
/// an interrupt is received. When an interrupt is received, the CPU will wake up and the kernel
/// will be able to eventually switch to an another task.
pub fn idle() -> ! {
    loop {
        // SAFETY: Enabling interrupts here is safe because we just enable interrupts to be wake
        // up by an interrupt later: there is no risk of undefined behavior by enabling interrupts
        // here.
        unsafe {
            x86_64::irq::enable();
            x86_64::irq::wait();
        }
    }
}
