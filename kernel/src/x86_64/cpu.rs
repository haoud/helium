use super::instruction;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct InterruptFrame {
    // Preserved registers
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // Scratch registers
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,

    /// Used internally by the interrupt macros
    internal: u64,

    // Error code if any
    pub code: u64,

    // Pushed by the CPU automatically when an interrupt is triggered
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// The different privilege levels of the CPU. In the kernel, we only use Ring 0 and Ring 3 are
/// used, respectively for kernel and user code.
pub enum Privilege {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

impl Privilege {
    pub const KERNEL: Self = Self::Ring0;
    pub const USER: Self = Self::Ring3;
}

/// Halt the current CPU core forever.
#[cold]
pub extern "C" fn freeze() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}

/// Read the current value of the CR2 register. This register contains the address that caused the
/// last page fault.
///
/// # Safety
/// This function is unsafe because it deals with very low level stuff that needs the use of unsafe
/// code to work properly.
#[must_use]
pub unsafe fn read_cr2() -> u64 {
    let cr2: u64;
    core::arch::asm!("mov {}, cr2", out(reg) cr2);
    cr2
}

/// Read the current value of the CR3 register. This register contains the physical address of the
/// page table (PML4) for the current CPU core.
///
/// # Safety
/// This function is unsafe because it can break memory safety if the returned value is used
/// incorrectly.
#[must_use]
pub unsafe fn read_cr3() -> u64 {
    let cr3: u64;
    core::arch::asm!("mov {}, cr3", out(reg) cr3);
    cr3
}

/// Write the given value to the CR3 register and change the current page table (PML4) for the
/// current CPU core. It will also flush all the TLB entries for the current CPU core, except the
/// global ones.
///
/// # Safety
/// This function is unsafe because it can break memory safety if the page table is not properly
/// constructed.
pub unsafe fn write_cr3(cr3: u64) {
    core::arch::asm!("mov cr3, {}", in(reg) cr3);
}

/// Wait for an interrupt to be triggered. This function is used to wait for an interrupt
/// when there is nothing else to do. It will enable interrupts, halt the CPU and wait for
/// an interrupt to be triggered. When an interrupt is triggered, it will disable interrupts
/// and return.
///
/// # Safety
/// This function is unsafe because the caller could be interrupted by an interrupt handler,
/// so the caller must take precautions to make this code preemptable (by example, by not
/// locking a mutex before calling this function).
pub unsafe fn wait_for_interrupt() {
    instruction::sti();
    instruction::hlt();
    instruction::cli();
}
