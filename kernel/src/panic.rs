use crate::{
    stop,
    x86_64::{
        self,
        lapic::{self, IpiDestination, IpiPriority},
    },
    Stop,
};
use cfg_if::cfg_if;

#[cold]
#[panic_handler]
unsafe fn panic(info: &core::panic::PanicInfo) -> ! {
    // Send a non-maskable interrupt to all other CPUs to stop them.
    lapic::send_ipi(IpiDestination::Other, IpiPriority::Nmi, 0);

    cfg_if!(
        if #[cfg(feature = "panic-info")] {
            crate::logger::on_panic();
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
            _ = info;
        }
    );

    stop(Stop::Failure);
}
