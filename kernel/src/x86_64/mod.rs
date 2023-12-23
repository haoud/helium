use self::paging::tlb;
use crate::user::scheduler;
use macros::init;

pub mod apic;
pub mod cpu;
pub mod exception;
pub mod fpu;
pub mod gdt;
pub mod idt;
pub mod instruction;
pub mod io;
pub mod irq;
pub mod lapic;
pub mod msr;
pub mod paging;
pub mod percpu;
pub mod pic;
pub mod pit;
pub mod serial;
pub mod smp;
pub mod syscall;
pub mod thread;
pub mod tss;
pub mod user;

/// The maximum number of CPUs supported by the kernel. This should be enough for a while.
const MAX_CPUS: usize = 32;

/// This function is one of the first function called by the kernel during the boot process
/// and is responsible for initializing basic hardware features like the GDT, IDT, PIC, PIT, etc
/// in order to have a working environment for the rest of the kernel. This function work in
/// a minimal environment and should not use any feature that is not already initialized, like
/// the memory manager.
///
/// # Safety
/// This function is unsafe because it initializes a lot of hardware devices or low level CPU
/// features that need the use of `unsafe` code to work properly.
#[init]
pub unsafe fn early_setup() {
    gdt::setup();
    idt::setup();
    exception::install();
    irq::install();
    pit::setup();
    pic::remap();
    fpu::setup();
    tlb::install();
    syscall::setup();
}

/// This function is called after the memory manager was initialized.
///
/// # Safety
/// This function is unsafe because it initializes a lot of hardware devices or low level CPU
/// features that need the use of `unsafe` code to work properly.
#[init]
pub unsafe fn setup() {
    // Assume that the BSP is CPU 0
    smp::per_cpu_setup(0);

    tss::install();
    paging::setup();
    apic::remap();
    lapic::enable();
    smp::start_cpus();
}

/// This function is called by the APs after their startup. It is responsible for initializing
/// per-CPU features like the GDT, IDT, TSS, etc. This function do less work than the BSP setup
/// function because the BSP already initialized a lot of things that only need to be done once.
#[init]
#[inline(never)]
unsafe fn ap_setup(info: &limine::SmpInfo) {
    smp::per_cpu_setup(info.lapic_id);

    gdt::load();
    idt::load();
    fpu::setup();
    tss::install();
    lapic::enable();
    syscall::setup();
    scheduler::setup();
}
