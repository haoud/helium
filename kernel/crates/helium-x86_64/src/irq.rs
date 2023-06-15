use crate::{
    cpu::InterruptFrame,
    idt, lapic,
    pic::{self, IRQ_BASE},
    pit,
};
use macros::{init, irq_handler};

const CLOCK_VECTOR: u8 = 0x7E;

/// Install the IRQ handlers.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the GDT / IDT is not properly
/// initialized, or if an IRQ handler is not properly installed or not adapted to be called from
/// an interrupt.
#[init]
pub unsafe fn install() {
    register_irq_handler(IRQ_BASE, irq_0);
    register_irq_handler(IRQ_BASE + 1, irq_1);
    register_irq_handler(IRQ_BASE + 2, irq_2);
    register_irq_handler(IRQ_BASE + 3, irq_3);
    register_irq_handler(IRQ_BASE + 4, irq_4);
    register_irq_handler(IRQ_BASE + 5, irq_5);
    register_irq_handler(IRQ_BASE + 6, irq_6);
    register_irq_handler(IRQ_BASE + 7, irq_7);
    register_irq_handler(IRQ_BASE + 8, irq_8);
    register_irq_handler(IRQ_BASE + 9, irq_9);
    register_irq_handler(IRQ_BASE + 10, irq_10);
    register_irq_handler(IRQ_BASE + 11, irq_11);
    register_irq_handler(IRQ_BASE + 12, irq_12);
    register_irq_handler(IRQ_BASE + 13, irq_13);
    register_irq_handler(IRQ_BASE + 14, irq_14);
    register_irq_handler(IRQ_BASE + 15, irq_15);

    idt::register_interruption(CLOCK_VECTOR, clock_handler);
}

/// A convenience function to register an irq handler. The interrupt handler is registered as a
/// privilege level 0 handler (to avoid userland code from directly triggering interrupts) and
/// with interrupts disabled.
#[init]
fn register_irq_handler(index: u8, handler: unsafe extern "C" fn()) {
    idt::register_interruption(index, handler);
}

/// The IRQ manager. This function is called by the IRQ handlers after they have saved the CPU
/// state, and passed the state to this function. The IRQ triggered is passed as an argument in
/// the `code` field of the `state` argument.
///
/// Currently, this function does nothing, but in the future, it will be used to handle IRQs and
/// to wake up threads that are waiting for IRQs.
#[irq_handler]
unsafe fn irq_handler(state: &mut InterruptFrame) {
    let irq = state.code as u8;
    match irq {
        0 => {
            pit::timer_tick();
            lapic::send_ipi(
                lapic::IpiDestination::All,
                lapic::IpiPriority::Normal,
                CLOCK_VECTOR,
            );
        }
        _ => log::error!("Unhandled IRQ: {}", state.code),
    }
    pic::send_eoi(irq);
}

#[macros::interrupt(0)]
pub fn clock_handler(_: &mut InterruptFrame) {
    let cpu = crate::smp::core_id();
    log::debug!("Tick {cpu}");
    lapic::send_eoi();
}
