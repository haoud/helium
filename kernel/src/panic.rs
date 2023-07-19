use crate::{
    logger, stop,
    x86_64::{
        self,
        lapic::{IpiDestination, IpiPriority},
    },
    Stop,
};
use cfg_if::cfg_if;

/// The panic handler. This function is called when the kernel encounters a fatal error that it
/// cannot recover from. This function will stop all other CPUs, print the panic message and stop
/// the kernel.
#[cold]
#[panic_handler]
unsafe fn panic(info: &core::panic::PanicInfo) -> ! {
    // Send a non-maskable interrupt to all other CPUs to stop them, but only if
    // they have finished their initialization (otherwise, they will triple fault)
    if x86_64::smp::ap_booted() {
        x86_64::lapic::send_ipi(IpiDestination::Other, IpiPriority::Nmi, 0);
    }

    cfg_if!(
        if #[cfg(feature = "panic-info")] {
            logger::on_panic();
            log::error!("The kernel has encountered a fatal error that it cannot recover from");
            log::error!("The kernel must stop to prevent further damage");

            if let Some(message) = info.message() {
                if let Some(location) = info.location() {
                    log::error!("[CPU {}] {} at {}", x86_64::smp::core_id(), message, location);
                } else {
                    log::error!("{}", message);
                }
            }
        } else {
            // Silence the unused variable warning
            _ = info;
        }
    );

    stop(Stop::Failure);
}
