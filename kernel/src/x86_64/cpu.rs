use bitflags::bitflags;
use super::instruction;

/// The interrupt frame pushed by the CPU and by the interrupt stubs when an interrupt
/// is triggered.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

bitflags! {
    /// The different flags of the xCR0 register.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct XCr0 : u64 {
        /// Enable using the x87 FPU state with the `XSAVE` and `XRSTOR`
        /// instructions. This flag is always set on modern processors.
        const X87 = 1 << 0;

        /// Enable using MXCSR and the XMM register with the `XSAVE` and
        /// `XRSTOR`instructions. This flag must be set if the `AVX` flag
        /// is set.
        const SSE = 1 << 1;

        /// Enable the AVX instruction set and using the upper 128 bits of
        /// AVX registers.
        const AVX = 1 << 2;
    }

    /// The different flags of the CR0 register.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Cr0 : u64 {
        /// Enables protected mode.
        const PE = 1 << 0;

        /// Enable monitor of the coprocessor.
        const MP = 1 << 1;

        /// Force all x87 FPU and MMX instructions to cause an #NE exception,
        /// allowing the software to emulate FPU/MMX/SSE/SSE2/SSE3 instructions
        const EM = 1 << 2;

        /// When set, using x87 FPU or MMX instructions will cause an # NM
        /// exception. This is used to implement lazy FPU saving/restoring.
        const TS = 1 << 3;

        // Indicates that the processor supports the 387DX math coprocessor
        // instructions. On modern processors, this is always set and cannot
        // be cleared.
        const ET = 1 << 4;

        /// Enable the native error reporting mechanism for x87 FPU errors.
        const NE = 1 << 5;

        /// When set, disables the rights of supervisor code to write into
        /// read-only pages.
        const WP = 1 << 16;

        /// Enables automatic usermode alignment checking if the RFLAGS.AC
        /// flag is also set.
        const AM = 1 << 18;

        /// Ignored on modern processors, used to control the write-back or
        /// write-through cache strategy.
        const NW = 1 << 29;

        /// Disable some processor cache (model-dependent).
        const CD = 1 << 30;

        /// Enable paging. This bit required the `Self::PG` bit to be set. This
        /// bit is also required to enable long mode.
        const PG = 1 << 31;
    }

    /// The different flags of the Cr4 register.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Cr4: u64 {
        /// Enables virtual-8086 mode support with hardware-supported performance
        /// enhancements.
        const VME = 1 << 0;

        /// Enables protected-mode virtual interrupts.
        const PVI = 1 << 1;

        /// Restrict the use of RDTSC and RDTSCP instructions to privileged code.
        const TSD = 1 << 2;

        /// Enable debug extensions that enable I/O breakpoints capability and
        /// enforcement treatment of DR4 and DR5 as reserved.
        const DE = 1 << 3;

        /// Enable the use of 4 MB physical frames in protected mode. In long mode,
        /// this flags is simply ignored.
        const PSE = 1 << 4;

        /// Enable physical address extension and 2 Mb physical frames. This flag
        /// is required to be set in long mode.
        const PAE = 1 << 5;

        /// Enable machine check exception to occur.
        const MCE = 1 << 6;

        /// Enable the global pages feature, which allow to make the page translation
        /// inside the TLB global to all processes. Those pages translations are not
        /// flushed when changing the CR3 register.
        const PGE = 1 << 7;

        /// Enable the performance monitoring counter and the RDPMC instruction to be
        /// used at any privilege level.
        const PCE = 1 << 8;

        /// Enable the FXSAVE and FXRSTOR instructions to manage the FPU state.
        const OSFXSR = 1 << 9;

        /// Enable the SIMD floating point exception (#XF) for handling SIMD floating
        /// point error.
        const OSXMMEXCPT = 1 << 10;

        /// Prevent the execution of the SGDT, SIDT, SLDT, SMSW, and STR instructions
        /// in user mode software.
        const UMPI = 1 << 11;

        /// Enable level 5 paging.
        const LA57 = 1 << 12;

        /// Enable VMX instructions.
        const VMXE = 1 << 13;

        /// Enable SMX instructions.
        const SMXE = 1 << 14;

        /// Enable user software to read and write their own FS and GS segment base
        const FSGSBASE = 1 << 16;

        /// Enable process-context identifiers (PCIDs) to tag TLB entries.
        const PCIDE = 1 << 17;

        /// Enable extended processor state management instructions, including XSAVE,
        /// XRESTORE, and XSETBV/XGETBV.
        const OSXSAVE = 1 << 18;

        /// Prevent the execution of instructions that reside in user pages when the
        /// processor is in supervisor mode.
        const SMEP = 1 << 20;

        /// Enable restriction for supervisor-mode read and write access to user-mode
        /// pages: access to used-mode pages is denied when the AC flag in EFLAGS is
        /// clear.
        const SMAP = 1 << 21;

        /// Enable protection keys feature.
        const PKE = 1 << 22;

        //// Enable CET shadow stack.
        const CET = 1 << 23;

        /// Enables 4 level paging to associate each address with a protection key.
        const PKS = 1 << 24;
    }
}

impl XCr0 {
    /// Read the current value of the XCR0 register.
    ///
    /// # Safety
    /// This function is unsafe because it can trigger an #UD exception if the CPU does not support
    /// the `XGETBV` instruction or if this instruction use is not enabled in the CR4 register.
    #[must_use]
    pub unsafe fn read() -> Self {
        Self::from_bits_retain(read_xcr0())
    }

    /// Write the given value to the XCR0 register.
    ///
    /// # Safety
    /// This function is unsafe because changing the XCR0 register can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn write(xcr0: Self) {
        unsafe { write_xcr0(xcr0.bits()) }
    }

    /// Enable the given flags in the XCR0 register.
    ///
    /// # Safety
    /// This function is unsafe because changing enabling XCR0 flags can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn enable(flags: Self) {
        unsafe { write_xcr0(read_xcr0() | flags.bits()) }
    }

    /// Disable the given flags in the XCR0 register.
    ///
    /// # Safety
    /// This function is unsafe because changing disabling XCR0 flags can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn disable(flags: Self) {
        unsafe { write_xcr0(read_xcr0() & !flags.bits()) }
    }
}

impl Cr0 {
    /// Read the current value of the CR0 register.
    #[must_use]
    pub fn read() -> Self {
        Self::from_bits_retain(read_cr0())
    }

    /// Write the given value to the CR0 register.
    ///
    /// # Safety
    /// This function is unsafe because changing the CR0 register can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn write(cr0: Self) {
        unsafe { write_cr0(cr0.bits()) }
    }

    /// Enable the given flags in the CR0 register.
    ///
    /// # Safety
    /// This function is unsafe because changing enabling CR0 flags can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn enable(flags: Self) {
        unsafe { write_cr0(read_cr0() | flags.bits()) }
    }

    /// Disable the given flags in the CR0 register.
    ///
    /// # Safety
    /// This function is unsafe because changing disabling CR0 flags can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn disable(flags: Self) {
        unsafe { write_cr0(read_cr0() & !flags.bits()) }
    }
}

impl Cr4 {
    /// Read the current value of the CR4 register.
    #[must_use]
    pub fn read() -> Self {
        Self::from_bits_retain(read_cr4())
    }

    /// Write the given value to the CR4 register.
    ///
    /// # Safety
    /// This function is unsafe because changing the CR4 register can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn write(cr4: Self) {
        unsafe { write_cr4(cr4.bits()) }
    }

    /// Enable the given flags in the CR4 register.
    ///
    /// # Safety
    /// This function is unsafe because changing enabling CR4 flags can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn enable(flags: Self) {
        unsafe { write_cr4(read_cr4() | flags.bits()) }
    }

    /// Disable the given flags in the CR4 register.
    ///
    /// # Safety
    /// This function is unsafe because changing disabling CR4 flags can, depending on the flags
    /// and on the current state of the CPU, break memory safety, cause undefined behavior or
    /// even reboot the computer if a triple fault occurs.
    pub unsafe fn disable(flags: Self) {
        unsafe { write_cr4(read_cr4() & !flags.bits()) }
    }
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

/// Read the current value of the XCR0 register.
///
/// # Safety
/// This function is unsafe because it can trigger an #UD exception if the CPU does not support
/// the `XGETBV` instruction or if this instruction use is not enabled in the CR4 register.
#[must_use]
pub unsafe fn read_xcr0() -> u64 {
    let (low, high): (u32, u32);
    core::arch::asm!(
        "xgetbv",
        in("ecx") 0,
        out("eax") low, out("edx") high,
    );

    u64::from(high) << 32 | u64::from(low)
}

/// Write the given value to the XCR0 register.
///
/// # Safety
/// This function is unsafe because it can trigger an #UD exception if the CPU does not support
/// the `XSETBV` instruction or if this instruction use is not enabled in the CR4 register.
/// Furthermore, changing the XCR0 register can, depending on the flags and on the current state
/// of the CPU, break memory safety, cause undefined behavior or even reboot the computer if a
/// triple fault occurs.
#[allow(clippy::cast_possible_truncation)]
pub unsafe fn write_xcr0(xcr0: u64) {
    let high = (xcr0 >> 32) as u32;
    let low = xcr0 as u32;

    core::arch::asm!(
        "xsetbv",
        in("ecx") 0,
        in("eax") low, in("edx") high,
    );
}

/// Read the current value of the CR0 register.
#[must_use]
pub fn read_cr0() -> u64 {
    let cr0: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) cr0);
    }
    cr0
}

/// Write the given value to the CR0 register.
///
/// # Safety
/// This function is unsafe because it deals with very low level stuff that needs the use of unsafe
/// code to work properly.
pub unsafe fn write_cr0(cr0: u64) {
    core::arch::asm!("mov cr0, {}", in(reg) cr0);
}

/// Read the current value of the CR2 register. This register contains the address that caused the
/// last page fault.
#[must_use]
pub fn read_cr2() -> u64 {
    let cr2: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2);
    }
    cr2
}

/// Read the current value of the CR3 register. This register contains the physical address of the
/// page table (PML4) for the current CPU core.
#[must_use]
pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
    }
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

/// Read the current value of the CR4 register.
#[must_use]
pub fn read_cr4() -> u64 {
    let cr4: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) cr4);
    }
    cr4
}

/// Write the given value to the CR4 register.
///
/// # Safety
/// This function is unsafe because it deals with very low level stuff that needs the use of unsafe
/// code to work properly.
pub unsafe fn write_cr4(cr4: u64) {
    core::arch::asm!("mov cr4, {}", in(reg) cr4);
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
