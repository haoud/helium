use crate::{stop, x86_64, Stop};
use cfg_if::cfg_if;

#[cold]
#[panic_handler]
unsafe fn panic(info: &core::panic::PanicInfo) -> ! {
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
