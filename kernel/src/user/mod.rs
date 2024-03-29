use crate::{module, x86_64};

use self::scheduler::{Scheduler, SCHEDULER};
pub mod buffer;
pub mod object;
pub mod pointer;
pub mod scheduler;
pub mod string;
pub mod task;
pub mod vmm;

// Re-export the userland types to avoid having to import them from the userland module.
// Example: "user::pointer::Pointer" will become "user::Pointer". This is useful because
// the userland types are heavily used in the kernel, espcielly in the syscalls.
pub use object::Object;
pub use pointer::Pointer;
pub use string::String;

/// Setup the userland subsystem. This function initialize the scheduler and
/// load the init task.
///
/// # Panics
/// This function will panic if the init task cannot be loaded.
#[init]
pub fn setup() {
    scheduler::setup();

    let module = module::read("/boot/init.elf").expect("Failed to read init task");
    let task = task::elf::load(module).expect("Failed to load init task");
    SCHEDULER.add_task(task);
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
