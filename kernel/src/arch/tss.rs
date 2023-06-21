use super::{
    cpu::Privilege,
    gdt::{self, Selector},
    instruction, smp,
};
use core::cell::RefCell;
use macros::{init, per_cpu};

#[per_cpu]
pub static TSS: RefCell<TaskStateSegment> = RefCell::new(TaskStateSegment::default());

const SELECTOR_BASE_IDX: u16 = 6;

/// Represents the Task State Segment (TSS) structure. It is used by the interrupt to determine
/// which stack to use when handling an interrupt and by the CPU to determine which I/O ports are
/// available to the running process.
/// See Intel Vol. 3A §7.2 for more details.
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
    #[must_use]
    pub const fn default() -> Self {
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

    pub fn set_kernel_stack(&mut self, stack: u64) {
        self.stack_table[0] = stack;
    }
}

/// Install the per-cpu TSS inside the GDT, and load it into the CPU.
///
/// # Safety
/// This function is unsafe because it can crash the kernel if the TSS is not properly
/// configured, or if the TSS selector is invalid.
#[init]
#[allow(clippy::cast_possible_truncation)]
pub unsafe fn install() {
    let index = SELECTOR_BASE_IDX + (smp::core_id() * 2) as u16;
    let descriptor = gdt::Descriptor::tss(&TSS.local().borrow());
    let selector = Selector::new(index, Privilege::KERNEL);
    gdt::GDT
        .lock()
        .set_descriptor(selector.index().into(), &descriptor);
    instruction::ltr(selector.0);
}
