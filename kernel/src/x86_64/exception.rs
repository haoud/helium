use super::{
    cpu::{self, InterruptFrame, Privilege},
    idt::{self, IDT},
    paging,
};
use macros::{exception, exception_err, init};

/// Setup the exception handlers.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the GDT / IDT is
/// not properly initialized.
#[init]
pub unsafe fn install() {
    register_exception_handler(0, divide_by_zero);
    register_exception_handler(1, debug);
    register_exception_handler(2, non_maskable_interrupt);
    register_exception_handler(3, breakpoint);
    register_exception_handler(4, overflow);
    register_exception_handler(5, bound_range_exceeded);
    register_exception_handler(6, invalid_opcode);
    register_exception_handler(7, device_not_available);
    register_exception_handler(8, double_fault);
    register_exception_handler(9, coprocessor_overrun);
    register_exception_handler(10, invalid_tss);
    register_exception_handler(11, segment_not_present);
    register_exception_handler(12, stack_segment_fault);
    register_exception_handler(13, general_protection_fault);
    register_exception_handler(14, page_fault);
    register_exception_handler(15, reserved);
    register_exception_handler(16, x87_floating_point);
    register_exception_handler(17, alignment_check);
    register_exception_handler(18, machine_check);
    register_exception_handler(19, simd_floating_point);
    register_exception_handler(20, virtualization);
    register_exception_handler(21, control_protection);
    register_exception_handler(22, reserved);
    register_exception_handler(23, reserved);
    register_exception_handler(24, reserved);
    register_exception_handler(25, reserved);
    register_exception_handler(26, reserved);
    register_exception_handler(27, reserved);
    register_exception_handler(28, hypervisor_injection);
    register_exception_handler(29, virtualization);
    register_exception_handler(30, security_exception);
    register_exception_handler(31, reserved);
}

/// A convenience function to register an exception handler.
#[init]
fn register_exception_handler(index: u8, handler: unsafe extern "C" fn()) {
    let mut idt = IDT.lock();

    let flags = idt::DescriptorFlags::new()
        .set_privilege_level(Privilege::KERNEL)
        .present(true);

    let descriptor = idt::Descriptor::new()
        .set_handler(handler)
        .set_options(flags)
        .build();

    idt.set_descriptor(index, descriptor);
}

#[exception]
fn divide_by_zero(_state: &InterruptFrame) {
    panic!("Divide by zero exception");
}

#[exception]
fn debug(_state: &InterruptFrame) {
    panic!("Debug exception");
}

/// A non-maskable interrupt (NMI) is a hardware interrupt that standard interrupt-masking
/// techniques in the system cannot ignore. It typically occurs to signal attention for
/// non-recoverable hardware errors.
/// Since it cannot be ignored, the NMI is often used to handle important tasks such as
/// the correction of memory errors and other hardware errors but in our case, we just
/// halt the CPU forever. This is also used when the kernel panics to stop all the CPUs.
#[exception]
fn non_maskable_interrupt(_state: &InterruptFrame) {
    cpu::freeze();
}

#[exception]
fn breakpoint(_state: &InterruptFrame) {
    panic!("Breakpoint exception");
}

#[exception]
fn overflow(_state: &InterruptFrame) {
    panic!("Overflow exception");
}

#[exception]
fn bound_range_exceeded(_state: &InterruptFrame) {
    panic!("Bound range exceeded exception");
}

#[exception]
fn invalid_opcode(_state: &InterruptFrame) {
    panic!("Invalid opcode exception");
}

#[exception]
fn device_not_available(_state: &InterruptFrame) {
    panic!("Device not available exception");
}

#[exception_err]
fn double_fault(_state: &InterruptFrame) {
    panic!("Double fault exception");
}

#[exception]
fn coprocessor_overrun(_state: &InterruptFrame) {
    panic!("Coprocessor segment overrun exception");
}

#[exception_err]
fn invalid_tss(_state: &InterruptFrame) {
    panic!("Invalid TSS exception");
}

#[exception_err]
fn segment_not_present(_state: &InterruptFrame) {
    panic!("Segment not present exception");
}

#[exception_err]
fn stack_segment_fault(_state: &InterruptFrame) {
    panic!("Stack segment fault exception");
}

#[exception_err]
fn general_protection_fault(state: &InterruptFrame) {
    panic!(
        "General protection fault exception (code: {:#x})",
        state.code
    );
}

#[exception_err]
fn page_fault(state: &InterruptFrame) {
    paging::handle_page_fault(
        cpu::Cr2::address(),
        paging::PageFaultErrorCode::from_bits_truncate(state.code),
    );
}

#[exception]
fn reserved(_state: &InterruptFrame) {
    panic!("Reserved exception");
}

#[exception]
fn x87_floating_point(_state: &InterruptFrame) {
    panic!("x87 floating point exception");
}

#[exception_err]
fn alignment_check(_state: &InterruptFrame) {
    panic!("Alignment check exception");
}

#[exception]
fn machine_check(_state: &InterruptFrame) {
    panic!("Machine check exception");
}

#[exception]
fn simd_floating_point(_state: &InterruptFrame) {
    panic!("SIMD floating point exception");
}

#[exception]
fn virtualization(_state: &InterruptFrame) {
    panic!("Virtualization exception");
}

#[exception_err]
fn control_protection(_state: &InterruptFrame) {
    panic!("Control protection exception");
}

#[exception]
fn hypervisor_injection(_state: &InterruptFrame) {
    panic!("Hypervisor injection exception");
}

#[exception_err]
fn vmm_communication(_state: &InterruptFrame) {
    panic!("Hypervisor injection exception");
}

#[exception_err]
fn security_exception(_state: &InterruptFrame) {
    panic!("Security exception");
}
