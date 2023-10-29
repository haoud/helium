use crate::{
    logger, stop,
    x86_64::{
        self,
        lapic::{IpiDestination, IpiPriority},
    },
    Stop,
};
use core::sync::atomic::{AtomicBool, Ordering};

/// A flag indicating if the panic handler has already been called. This flag is used to prevent
/// the kernel from panicking while the panic handler is running (the panic handler as panicked)
/// which would result in a triple fault.
static ON_PANIC: AtomicBool = AtomicBool::new(false);

/// The panic handler. This function is called when the kernel encounters a fatal error that it
/// cannot recover from. This function will stop all other CPUs, print the panic message and stop
/// the kernel.
#[cold]
#[panic_handler]
unsafe fn panic(info: &core::panic::PanicInfo) -> ! {
    if !ON_PANIC.swap(true, Ordering::SeqCst) {
        // Send a non-maskable interrupt to all other CPUs to stop them, but only if
        // they have finished their initialization (otherwise, they will triple fault)
        if x86_64::smp::ap_booted() {
            x86_64::lapic::send_ipi(IpiDestination::Other, IpiPriority::Nmi, 0);
        }

        logger::on_panic();
        log::error!("The kernel has encountered a fatal error that it cannot recover from");
        log::error!("The kernel must stop to prevent further damage");

        if let Some(message) = info.message() {
            if let Some(location) = info.location() {
                // If the kernel panicked early (before the per-cpu variables are initialized),
                // we must not try to get the core ID because it will panic since the per-cpu
                // variables are not initialized. In this case, we are sure that the kernel is
                // running on the first core, so we can simply set the core ID to 0.
                let core = match x86_64::smp::ap_booted() {
                    true => x86_64::smp::core_id(),
                    false => 0,
                };

                log::error!("[CPU {}] {} at {}", core, message, location);
            } else {
                log::error!("{}", message);
            }
        }
    }

    stop(Stop::Failure);
}
