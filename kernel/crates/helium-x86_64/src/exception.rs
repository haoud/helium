use macros::{exception, exception_err, init};

use crate::{
    cpu::{self, Privilege, State},
    idt::{self, IDT},
};

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
fn divide_by_zero(_state: &State) {
    panic!("Divide by zero exception");
}

#[exception]
fn debug(_state: &State) {
    panic!("Debug exception");
}

#[exception]
fn non_maskable_interrupt(_state: &State) {
    cpu::freeze();
}

#[exception]
fn breakpoint(_state: &State) {
    panic!("Breakpoint exception");
}

#[exception]
fn overflow(_state: &State) {
    panic!("Overflow exception");
}

#[exception]
fn bound_range_exceeded(_state: &State) {
    panic!("Bound range exceeded exception");
}

#[exception]
fn invalid_opcode(_state: &State) {
    panic!("Invalid opcode exception");
}

#[exception]
fn device_not_available(_state: &State) {
    panic!("Device not available exception");
}

#[exception_err]
fn double_fault(_state: &State) {
    panic!("Double fault exception");
}

#[exception]
fn coprocessor_overrun(_state: &State) {
    panic!("Coprocessor segment overrun exception");
}

#[exception_err]
fn invalid_tss(_state: &State) {
    panic!("Invalid TSS exception");
}

#[exception_err]
fn segment_not_present(_state: &State) {
    panic!("Segment not present exception");
}

#[exception_err]
fn stack_segment_fault(_state: &State) {
    panic!("Stack segment fault exception");
}

#[exception_err]
fn general_protection_fault(_state: &State) {
    panic!("General protection fault exception");
}

#[exception_err]
fn page_fault(_state: &State) {
    panic!("Page fault exception");
}

#[exception]
fn reserved(_state: &State) {
    panic!("Reserved exception");
}

#[exception]
fn x87_floating_point(_state: &State) {
    panic!("x87 floating point exception");
}

#[exception_err]
fn alignment_check(_state: &State) {
    panic!("Alignment check exception");
}

#[exception]
fn machine_check(_state: &State) {
    panic!("Machine check exception");
}

#[exception]
fn simd_floating_point(_state: &State) {
    panic!("SIMD floating point exception");
}

#[exception]
fn virtualization(_state: &State) {
    panic!("Virtualization exception");
}

#[exception_err]
fn control_protection(_state: &State) {
    panic!("Control protection exception");
}

#[exception]
fn hypervisor_injection(_state: &State) {
    panic!("Hypervisor injection exception");
}

#[exception_err]
fn vmm_communication(_state: &State) {
    panic!("Hypervisor injection exception");
}

#[exception_err]
fn security_exception(_state: &State) {
    panic!("Security exception");
}
