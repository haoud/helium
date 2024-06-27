use super::{
    cpu::Privilege,
    gdt::{self, Selector},
    instruction, smp,
};
use addr::virt::Virtual;

/// The Task State Segment (TSS) for the current CPU. This is a per-cpu
/// variable, and therefore each CPU has its own TSS.
#[per_cpu]
static mut TSS: TaskStateSegment = TaskStateSegment::new();

/// The index of the first TSS selector in the GDT. The TSS selector
/// associated with the current CPU is `SELECTOR_BASE_IDX + (smp::core_id()
/// * 2)`.
const SELECTOR_BASE_IDX: u16 = 6;

/// Represents the Task State Segment (TSS) structure. It is used by the
/// interrupt to determine which stack to use when handling an interrupt
/// and by the CPU to determine which I/O ports are available to the running
/// process. See Intel Vol. 3A §7.2 for more details.
#[repr(C, packed(4))]
pub struct TaskStateSegment {
    reserved_1: u32,
    stack_table: [u64; 3],
    reserved_2: u64,
    interrupt_stack_table: [u64; 7],
    reserved_3: u64,
    reserved_4: u16,
    iomap_base: u16,
}

impl TaskStateSegment {
    /// Create a new TSS with default values: All fields are set to 0, except
    /// the I/O map base field which is set to 104 (meaning that the I/O
    /// permission bitmap does not exist).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            reserved_1: 0,
            stack_table: [0; 3],
            reserved_2: 0,
            interrupt_stack_table: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: 104,
        }
    }

    /// Set the kernel stack for the current CPU that will be used when
    /// handling interrupts if the CPU is in user mode and needs to switch
    /// to the kernel.
    pub fn set_kernel_stack(&mut self, stack: u64) {
        self.stack_table[0] = stack;
    }
}

/// Install the per-cpu TSS inside the GDT, and load it into the CPU.
///
/// # Safety
/// This function is unsafe because it can crash the kernel if the TSS is
/// not properly configured, or if the TSS selector is invalid.
#[init]
#[allow(clippy::cast_possible_truncation)]
pub unsafe fn install() {
    let index = SELECTOR_BASE_IDX + (smp::core_id() * 2) as u16;
    let descriptor = gdt::Descriptor::tss(&TSS.local());
    let selector = Selector::new(index, Privilege::KERNEL);
    gdt::GDT
        .lock()
        .set_descriptor(selector.index().into(), &descriptor);
    instruction::ltr(selector.0);
}

/// Set the kernel stack for the current CPU that will be used when handling
/// interrupts: If a interruption is triggered while the CPU is in user mode
/// (syscall, exception, etc.), the CPU will switch to the kernel stack before
/// calling the interrupt handler.
pub fn set_kernel_stack(stack: Virtual) {
    // SAFETY: This is safe if we make sure that we make sure that we does
    // not create multiple mutable references to the TSS. And because this
    // is a per-cpu variable, this static variable is only accessed by the
    // current CPU and therefore implement the Send and Sync traits.
    unsafe {
        TSS.local_mut().set_kernel_stack(u64::from(stack));
    }
}
