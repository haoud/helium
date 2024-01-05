use crate::x86_64;

use self::scheduler::{Scheduler, SCHEDULER};
pub mod buffer;
pub mod object;
pub mod pointer;
pub mod scheduler;
pub mod string;
pub mod task;
pub mod vmm;

/// Setup the userland subsystem. This function initialize the scheduler and
/// load the init task.
///
/// # Panics
/// This function will panic if the init task cannot be loaded.
#[init]
pub fn setup() {
    scheduler::setup();
    SCHEDULER.add_task(
        task::elf::load(
            Arc::new(Spinlock::new(vmm::Manager::new())),
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
    SCHEDULER.engage_cpu();
}

/// The idle function. This function will be used by the idle kernel task when there is no other
/// task to run. This function is a simple loop that simply put the CPU in a low power state until
/// an interrupt is received. When an interrupt is received, the CPU will wake up and the kernel
/// will be able to eventually switch to an another task.
pub fn idle() -> ! {
    loop {
        // SAFETY: Enabling interrupts here is safe because we just enable interrupts to be wake
        // up by an interrupt later: there is no risk of undefined behavior by enabling interrupts
        // here. Of course, the IDT must be correctly initialized before calling this function.
        unsafe {
            x86_64::irq::enable();
            x86_64::irq::wait();
        }
    }
}
