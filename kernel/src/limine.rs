/// The request to the limine bootloader to get a memory map.
pub static LIMINE_MEMMAP: limine::request::MemoryMapRequest =
    limine::request::MemoryMapRequest::new();

/// The request to the limine bootloader to get a HHDM, mapping all the physical memory at
/// a specific address (`0xFFFF_8000_0000_0000`).
pub static LIMINE_HHDM: limine::request::HhdmRequest = limine::request::HhdmRequest::new();

/// The Limine SMP request. This tells Limine to start the APs, so we have much less work
/// to do and we can focus on more important things.
pub static LIMINE_SMP: limine::request::SmpRequest = limine::request::SmpRequest::new();

/// The Limine framebuffer request. This tells Limine to set up a framebuffer for us.
pub static LIMINE_FRAMEBUFFER: limine::request::FramebufferRequest =
    limine::request::FramebufferRequest::new();

/// The Limine module request. This tells Limine to load modules for us.
pub static LIMINE_MODULES: limine::request::ModuleRequest = limine::request::ModuleRequest::new();
