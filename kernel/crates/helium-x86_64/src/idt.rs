use bitfield::{BitMut, BitRangeMut};
use macros::{init, interrupt};
use sync::Spinlock;

use crate::{
    cpu::{InterruptFrame, Privilege},
    gdt, instruction,
};

core::arch::global_asm!(include_str!("asm/interrupt.asm"));

pub static IDT: Spinlock<Table> = Spinlock::new(Table::empty());

/// All the handlers called when an interrupt is triggered should have this signature.
pub type Handler = unsafe extern "C" fn();

/// Represents the IDT and its 256 entries
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C, align(16))]
pub struct Table {
    entries: [Descriptor; Self::SIZE],
    register: Register,
}

impl Table {
    /// The number of entries in the IDT, which is always 256.
    const SIZE: usize = 256;

    /// Creates a new empty IDT. All entries are set to the MISSING descriptor by default. If a
    /// MISSING descriptor is triggered, a general protection fault is raised.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            entries: [Descriptor::MISSING; Self::SIZE],
            register: Register::null(),
        }
    }

    /// Returns the total number of entries in the IDT.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        Self::SIZE
    }

    /// Set the IDT entry at the given index to the given descriptor. If the entry is already
    /// present, it will be overwritten.
    pub fn set_descriptor(&mut self, index: u8, descriptor: Descriptor) {
        self.entries[index as usize] = descriptor;
    }

    /// Set the IDT register to point to the IDT and load it into the CPU.
    ///
    /// # Safety
    /// This function is unsafe because loading the IDT is a critical operation that can lead to
    /// undefined behavior if done incorrectly. In most cases, this would lead to a triple fault
    /// which would reboot the computer when loading the IDT or when a interrupt is triggered.
    pub unsafe fn load(&mut self) {
        self.register.limit = (core::mem::size_of::<Descriptor>() * Self::SIZE - 1) as u16;
        self.register.base = self.entries.as_ptr() as u64;
        self.register.load();
    }
}

/// Represents an IDT descriptor. An IDT descriptor is a 16 bytes structure that contains the
/// address of the handler, the segment selector and the descriptor flags. For more details, see
/// the Intel manual (Volume 3, Chapter 6).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C, packed)]
pub struct Descriptor {
    offset_low: u16,
    selector: u16,
    flags: DescriptorFlags,
    offset_middle: u16,
    offset_high: u32,
    zero: u32,
}

impl Descriptor {
    pub const MISSING: Self = Self::zero();

    /// Create a new descriptor with the default values. The default values are:
    /// - The descriptor is not marked as present
    /// - The handler address is set to 0
    /// - The descriptor flags are set to the default flags (see [`DescriptorFlags::new`])
    /// - The segment selector is set to the kernel code segment
    #[must_use]
    pub const fn new() -> Self {
        Self {
            offset_low: 0,
            selector: gdt::Selector::KERNEL_CODE.0,
            flags: DescriptorFlags::new(),
            offset_middle: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    /// Create a new descriptor with all fields set to 0. This descriptor will raise a general
    /// protection fault when triggered.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            offset_low: 0,
            selector: gdt::Selector::NULL.0,
            flags: DescriptorFlags::zero(),
            offset_middle: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    /// Set the address of the handler. The handler should be a function generated by the
    /// [`interrupt_handler`] macro, because rust functions cannot be called directly when a
    /// interrupt is triggered.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn set_handler(&mut self, handler: Handler) -> &mut Self {
        let handler = handler as usize;
        self.offset_middle = (handler >> 16) as u16;
        self.offset_high = (handler >> 32) as u32;
        self.offset_low = handler as u16;
        self
    }

    /// Set the descriptor flags. The default is to set the present bit and to disable interrupts
    /// when the handler is invoked (see [`DescriptorFlags`] for more details)
    #[must_use]
    pub fn set_options(&mut self, flags: DescriptorFlags) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Set the segment selector that will be loaded into the CS register when the handler is
    /// invoked. The default is the kernel code segment
    #[must_use]
    pub fn set_selector(&mut self, selector: gdt::Selector) -> &mut Self {
        self.selector = selector.0;
        self
    }

    /// Build the descriptor from the current state.
    #[must_use]
    pub fn build(&mut self) -> Self {
        let mut result = Self::new();
        core::mem::swap(&mut result, self);
        result
    }
}

/// Represents an descriptor flags, used to control the behavior of the CPU when a interrupt is
/// triggered. For more details, see the Intel manual (Volume 3, Chapter 6).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DescriptorFlags(u16);

impl DescriptorFlags {
    #[must_use]
    pub const fn new() -> Self {
        Self(0x0F00)
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Set or reset the present bit. If the present bit is not set, the CPU will raise a
    /// general protection fault when the handler is invoked.
    #[must_use]
    pub fn present(mut self, present: bool) -> Self {
        self.0.set_bit(15, present);
        self
    }

    /// Set the interrupt gate type and enable or not interrupts when the handler is invoked. If
    /// enabled is set to false (default), the IF flag is cleared when the handler is invoked.
    #[must_use]
    pub fn with_interrupts(mut self, enabled: bool) -> Self {
        self.0.set_bit(8, !enabled);
        self
    }

    /// Set the required privilege level (DPL) for invoking the handler via the `int` instruction.
    /// The default is 0 (kernel). If CPL < DPL when the handler is invoked, the CPU will raise a
    /// general protection fault. If a interrupt is triggered by the hardware, the DPL is ignored.
    /// This is useful to prevent user code from invoking privileged handlers.
    #[must_use]
    pub fn set_privilege_level(mut self, dpl: Privilege) -> Self {
        self.0.set_bit_range(15, 13, dpl as u16);
        self
    }

    /// Set the stack index for the handler. The default is 0 (no IST). The index represents the
    /// index of the stack in the TSS. The hardware will use the stack at the given index when the
    /// handler is invoked. This is useful to prevent stack overflows when the handler.
    #[must_use]
    pub fn set_stack_index(mut self, index: u16) -> Self {
        // The hardware IST index starts at 1 (0 means no IST).
        self.0.set_bit_range(3, 0, index + 1);
        self
    }
}

impl Default for DescriptorFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Represent the IDT register. This register is used by the CPU to find the IDT. It contains the
/// base address of the IDT and its limit.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C, packed)]
pub struct Register {
    limit: u16,
    base: u64,
}

impl Register {
    /// Create a new IDT register with a null base and limit.
    #[must_use]
    pub const fn null() -> Self {
        Self { limit: 0, base: 0 }
    }

    /// Set the IDT register to point to the given IDT.
    pub fn set_table(&mut self, table: &Table) {
        self.limit = (core::mem::size_of::<Descriptor>() * Table::SIZE - 1) as u16;
        self.base = table as *const Table as u64;
    }

    /// Load the IDT register into the CPU. This is unsafe because the caller must ensure that the
    /// IDT is valid and that the IDT register is correctly set.
    ///
    /// # Safety
    /// This function is unsafe because the caller must ensure that the IDT is valid and correctly
    /// set. If not, the CPU will reset when loading the IDT register, or when invoking an
    /// interrupt. The caller must also ensure that the IDT stay valid and at the same address
    /// as long as the IDT register is loaded into the CPU.
    pub unsafe fn load(&self) {
        instruction::lidt(self as *const _ as u64);
    }
}

/// Setup the IDT. This function must be called before invoking any interrupt and create an IDT
/// which contains the default handler for all interrupts.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the IDT is not correctly
/// initialized that could lead to a system crash when loading the IDT register or invoking an
/// interrupt.
#[init]
pub unsafe fn setup() {
    for i in 0..Table::SIZE {
        register_interruption(i as u8, default);
    }

    IDT.lock().load();
}

pub fn register_interruption(vector: u8, handler: unsafe extern "C" fn()) {
    let flags = DescriptorFlags::new()
        .set_privilege_level(Privilege::KERNEL)
        .present(true);

    let descriptor = Descriptor::new()
        .set_handler(handler)
        .set_options(flags)
        .build();

    IDT.lock().set_descriptor(vector, descriptor);
}

/// Load the IDT register into the current CPU.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the IDT is valid and correctly
/// set. If not, the CPU will reset when loading the IDT register, or when invoking an
/// interrupt.
pub unsafe fn load() {
    IDT.lock().load();
}

/// The default interrupt handler. This function is called when an interrupt is triggered but no
/// handler is registered for it.
#[interrupt(0)]
fn default(state: &mut InterruptFrame) {
    panic!("Unhandled interrupt: {:#x}", state.code);
}
