use limine::LimineSmpInfo;
use macros::init;

pub mod apic;
pub mod cpu;
pub mod exception;
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
pub mod tlb;
pub mod tss;

/// The maximum number of CPUs supported by the kernel. This should be enough for a while.
const MAX_CPUS: usize = 32;

#[init]
pub unsafe fn early_setup() {
    gdt::setup();
    idt::setup();
    exception::install();
    irq::install();
    pit::setup();
    pic::remap();
    tlb::install();
    syscall::setup();
}

#[init]
pub unsafe fn setup() {
    tss::install();
    paging::setup();
    apic::remap();
    lapic::enable();
    smp::start_cpus();
}

#[init]
#[inline(never)]
unsafe fn ap_setup(info: &LimineSmpInfo) {
    smp::per_cpu_setup(info.lapic_id);

    gdt::load();
    idt::load();
    tss::install();
    lapic::enable();
    syscall::setup();
}
