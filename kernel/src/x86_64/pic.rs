use super::io::Port;

static MASTER_PIC_CMD: Port<u8> = Port::new(0x20);
static MASTER_PIC_DATA: Port<u8> = Port::new(0x21);
static SLAVE_PIC_CMD: Port<u8> = Port::new(0xA0);
static SLAVE_PIC_DATA: Port<u8> = Port::new(0xA1);

/// The base IRQ number for the PICs. We remap the PICs IRQs right after the CPU exceptions, which
/// are the first 32 IRQs (0-31).
pub const IRQ_BASE: u8 = 32;

/// The number of IRQs that each PIC supports.
pub const IRQ_PER_PIC: u8 = 8;

/// The number of PICs in the system.
pub const PIC_COUNT: u8 = 2;

/// The number of IRQs supported by the PICs.
pub const IRQ_COUNT: usize = 16;

/// The end-of-interrupt (EOI) command for the PICs.
const PIC_EOI: u8 = 0x20;

/// Remap the PICs from their default IRQs (0-15) to the given base IRQ. This is necessary because
/// the default IRQs conflict with the CPU exceptions, which are the first 32 IRQs (0-31). The
/// master PIC will use IRQs [base, base + 7] and the slave PIC will use IRQs [base + 8, base + 15].
/// After remapping, all interrupts are unmasked, but no interrupts will occur until the interrupts
/// are enabled with the `sti` instruction.
///
/// # Safety
/// This function is unsafe because it writes to the PICs with I/O ports, which can cause undefined
/// behavior if the PICs do not exist or are not in the expected state.
#[init]
pub unsafe fn remap() {
    // ECW1: Cascade mode, ICW4 needed
    MASTER_PIC_CMD.write_and_pause(0x11);
    SLAVE_PIC_CMD.write_and_pause(0x11);

    // ICW2: Write the base IRQs for the PICs
    MASTER_PIC_DATA.write_and_pause(IRQ_BASE);
    SLAVE_PIC_DATA.write_and_pause(IRQ_BASE + 8);

    // ICW3: Connect the PICs to each other
    MASTER_PIC_DATA.write_and_pause(4); // The slave PIC is connected to IRQ4 on the master PIC
    SLAVE_PIC_DATA.write_and_pause(2); // The master PIC is connected to IRQ2 on the slave PIC

    // ICW4: Request 8086 mode
    MASTER_PIC_DATA.write_and_pause(0x01);
    SLAVE_PIC_DATA.write_and_pause(0x01);

    // OCW1: Enable all interrupts
    unmask_all();
}

/// Check if the given IRQ number is in the range of the PICs. This is useful for checking if an
/// interrupt handler should send an EOI to the PICs.
/// The parameter `int` is the interrupt number raised by the CPU.
#[must_use]
pub fn concerned(int: u8) -> bool {
    (IRQ_BASE..IRQ_BASE + 16).contains(&int)
}

/// Send an end-of-interrupt (EOI) to the PICs. This must be called after an interrupt handler
/// finishes executing. If the IRQ number is not in the range of the PICs, this function does
/// nothing.
///
/// The IRQ number needed as an argument is the IRQ number that the PICs are configured to use, not
/// the IRQ number that the CPU uses. For example, if the PICs are configured to use an base IRQ of
/// 32, then the IRQ number needed as an argument is 0 if the CPU raised IRQ 32, 1 if the CPU raised
/// IRQ 33, etc.
///
/// # Safety
/// This function is unsafe because it writes to the PICs with I/O ports, which can cause undefined
/// behavior if the PICs do not exist or are not in the expected state, or if it is used incorrectly.
pub unsafe fn send_eoi(irq: u8) {
    if (0..8).contains(&irq) {
        MASTER_PIC_CMD.write_and_pause(PIC_EOI);
    } else if (8..16).contains(&irq) {
        SLAVE_PIC_CMD.write_and_pause(PIC_EOI);
        MASTER_PIC_CMD.write_and_pause(PIC_EOI);
    } else {
        log::error!(
            "Tried to send EOI for IRQ {} which is not in the range of the PICs",
            irq
        );
    }
}

/// Unmask all interrupts on the PICs. This is the default state after remapping the PICs.
///
/// # Safety
/// This function is unsafe because it writes to the PICs with I/O ports, which can cause undefined
/// behavior if the PICs do not exist or are not in the expected state, or if the PICs raise an
/// interrupt that is not handled by the IDT.
pub unsafe fn unmask_all() {
    MASTER_PIC_DATA.write_and_pause(0x00);
    SLAVE_PIC_DATA.write_and_pause(0x00);
}

/// Mask all interrupts on the PICs. An interrupt masked by the PICs will never occur and will not
/// be sent to the CPU (lost forever).
///
/// # Safety
/// This function is unsafe because it writes to the PICs with I/O ports, which can cause undefined
/// behavior if the PICs do not exist or are not in the expected state.
pub unsafe fn mask_all() {
    MASTER_PIC_DATA.write_and_pause(0xFF);
    SLAVE_PIC_DATA.write_and_pause(0xFF);
}
