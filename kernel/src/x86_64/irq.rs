use super::pit;
use super::{
    cpu::InterruptFrame,
    idt, lapic,
    pic::{self, IRQ_BASE},
};
use crate::user::scheduler;
use macros::{init, interrupt, irq_handler};

/// The IPI vector used to inform the CPU that a new timer tick is available.
const CLOCK_VECTOR: u8 = 0x7E;

/// The IRQ number of the PIT.
const PIT_IRQ: u8 = 0;

/// The IRQ number of the keyboard.
const KEYBOARD_IRQ: u8 = 1;

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

/// Disables interrupts on the current core.
///
/// # Safety
/// This function is unsafe because disabling interrupts can have unexpected side effects if
/// a portion of code is designed to be executed with interrupts enabled.
#[inline]
pub unsafe fn disable() {
    super::instruction::cli();
}

/// Enables interrupts on the current core.
///
/// # Safety
/// This function is unsafe because enabling interrupts can cause undefined behavior if the
/// GDT or the IDT is not properly initialized. It can also have unexpected side effects, and can
/// create race conditions in some cases. For example, if an interrupt is triggered when a lock
/// is held and the interrupt handler tries to acquire the same lock, it will deadlock and freeze
/// the kernel.
#[inline]
pub unsafe fn enable() {
    super::instruction::sti();
}

/// Waits for the next interrupt to happen.
///
/// # Safety
/// This function is unsafe because it can cause unexpected side effects if used incorrectly.
/// For example, if interrupts are disabled, this function will likely wait forever and freeze
/// the kernel.
#[inline]
pub unsafe fn wait() {
    super::instruction::hlt();
}

/// Returns the current interrupt state on the current core.
#[inline]
#[must_use]
pub fn enabled() -> bool {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) flags
        );
    }
    flags & (1 << 9) != 0
}

/// Restores a previous interrupt state.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior when enabling or disabling
/// interrupts. See the documentation of the `enable` and `disable` functions for more details.
#[inline]
pub unsafe fn restore(state: bool) {
    match state {
        false => disable(),
        true => enable(),
    }
}

/// Executes the given function with interrupts disabled. The previous interrupt state is restored
/// after the function returns, so interrupts will not be re-enabled if they were disabled before
/// calling this function. However, this function will not prevent exceptions from happening !
///
/// # Safety
/// This function is safe to use, because it will not enable interrupts if they were disabled before
/// calling it. We consider that if the interrupts were enabled before calling this function, then
/// this is safe to re-enable them after the function returns.
pub fn without<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    unsafe {
        let irq = enabled();
        if irq {
            disable();
        }
        let ret = f();
        if irq {
            enable();
        }
        ret
    }
}

/// The IRQ manager. This function is called by the IRQ handlers after they have saved the CPU
/// state, and passed the state to this function. The IRQ triggered is passed as an argument in
/// the `code` field of the `state` argument.
///
/// Currently, this function does nothing, but in the future, it will be used to handle IRQs and
/// to wake up threads that are waiting for IRQs.
#[irq_handler]
#[allow(clippy::cast_possible_truncation)]
unsafe fn irq_handler(state: &mut InterruptFrame) {
    let irq = state.code as u8;

    pic::send_eoi(irq);
    match irq {
        PIT_IRQ => {
            pit::timer_tick();
            lapic::send_ipi(
                lapic::IpiDestination::All,
                lapic::IpiPriority::Normal,
                CLOCK_VECTOR,
            );
        }

        KEYBOARD_IRQ => {
            log::info!("Keyboard IRQ");
        }
        _ => log::error!("Unhandled IRQ: {}", state.code),
    }
}

#[interrupt(0)]
pub fn clock_handler(_: &mut InterruptFrame) {
    lapic::send_eoi();
    scheduler::timer_tick();
}
