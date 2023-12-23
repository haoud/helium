/// The request to the limine bootloader to get a memory map.
pub static LIMINE_MEMMAP: limine::MemmapRequest = limine::MemmapRequest::new(0);

/// The request to the limine bootloader to get a HHDM, mapping all the physical memory at
/// a specific address (`0xFFFF_8000_0000_0000`).
pub static LIMINE_HHDM: limine::HhdmRequest = limine::HhdmRequest::new(0);

/// The Limine SMP request. This tells Limine to start the APs, so we have much less work
/// to do and we can focus on more important things.
pub static LIMINE_SMP: limine::SmpRequest = limine::SmpRequest::new(0);
