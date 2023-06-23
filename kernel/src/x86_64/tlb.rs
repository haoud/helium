use super::{
    cpu::{self, InterruptFrame, Privilege},
    idt::{self, IDT},
    instruction::{self, invlpg},
    lapic::{self, IpiDestination, IpiPriority},
};
use addr::Virtual;
use macros::interrupt;

/// The vector number of the TLB invalidation interrupt.
pub const SHOOTDOWN_VECTOR: u8 = 0x7F;

/// Install the TLB shootdown interrupt handler. This handler is called when a TLB shootdown
/// is requested by another CPU core.
pub fn install() {
    let mut idt = IDT.lock();
    let flags = idt::DescriptorFlags::new()
        .set_privilege_level(Privilege::KERNEL)
        .present(true);

    let descriptor = idt::Descriptor::new()
        .set_handler(shootdown_handler)
        .set_options(flags)
        .build();

    idt.set_descriptor(SHOOTDOWN_VECTOR, descriptor);
}

/// Invalidate the TLB entry on the current core for the given virtual address.
pub fn invalidate(addr: Virtual) {
    instruction::invlpg(addr.into());
}

/// Flush all the TLB entries on the current CPU core. This is done by reloading the CR3 register.
/// This is a very expensive operation, as it flushes the entire TLB (except for the global pages)
/// and lead to cache misses on the next memory accesses.
pub fn flush() {
    unsafe {
        cpu::write_cr3(cpu::read_cr3());
    }
}

/// Flush the TLB entries on all CPU cores for the given virtual address. This is done by sending
/// an IPI to all other cores, which will then flush their TLB.
pub fn shootdown(address: Virtual) {
    unsafe {
        lapic::send_ipi(IpiDestination::Other, IpiPriority::Normal, SHOOTDOWN_VECTOR);
    }
    invlpg(address.into());
}

/// Called when a TLB shootdown interrupt is received. This simply reloads the CR3 register, which
/// flushes the entire TLB (except for the global pages).
/// In the future, this function wshould only invalidate the TLB entry for the given virtual
/// address.
#[interrupt(0)]
fn shootdown_handler(_: &mut InterruptFrame) {
    lapic::send_eoi();
    flush();
}
