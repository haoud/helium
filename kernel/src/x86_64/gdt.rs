use bitfield::BitRangeMut;
use bitflags::bitflags;

use super::{cpu::Privilege, instruction, tss::TaskStateSegment, MAX_CPUS};

/// The Global Descriptor Table. The size of the GDT is fixed and is equal to 6 + 2 * `MAX_CPU`.
/// The first 5 entries are reserved for the NULL descriptor, the kernel code, the kernel data, the
/// user code and the user data, and the remaining entries are used for the TSS of each CPU.
pub static GDT: Spinlock<Table<{ 6 + MAX_CPUS * 2 }>> = Spinlock::new(Table::empty());

/// A structure that represents a GDT table and its register. The table is an array of entries,
/// with a compile-time fixed size (maximum 8192 entries).
#[derive(Clone)]
pub struct Table<const N: usize> {
    descriptors: [Entry; N],
    register: Register,
}

impl<const N: usize> Table<N> {
    pub const MAX_SIZE: usize = 8192;
    const MAX_SIZE_ASSERT: () =
        assert!(N <= Self::MAX_SIZE, "GDT can't be larger than 8192 entries");

    /// Creates a new empty GDT. All entries are set to the NULL descriptor by default
    #[must_use]
    #[allow(clippy::let_unit_value)]
    pub const fn empty() -> Self {
        let _ = Self::MAX_SIZE_ASSERT; // Check that the GDT isn't too large
        Self {
            descriptors: [Entry::NULL; N],
            register: Register::null(),
        }
    }

    /// Returns the total number of entries in the GDT.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Set the GDT entry at the given index to the given descriptor. If the entry is already in
    /// use, it will be overwritten. If the index is out of bounds, the behavior is undefined.
    ///
    /// # Warning
    /// If you set a system descriptor (i.e. a TSS descriptor), remember that it requires two GDT
    /// entries ! If you want to add a descriptor after a system descriptor, you need increment the
    /// index by 2.
    ///
    /// # Panics
    /// This function panics if the index is out of bounds (i.e. greater than or equal to the
    /// GDT's capacity) or if the entry is already in use.
    pub fn set_descriptor(&mut self, index: usize, descriptor: &Descriptor) {
        if let Descriptor::Segment(x) = descriptor {
            self.descriptors[index] = Entry::new(*x);
        } else if let Descriptor::System(x, y) = descriptor {
            self.descriptors[index + 1] = Entry::new(*y);
            self.descriptors[index] = Entry::new(*x);
        }
    }

    /// Clear the GDT entry at the given index.
    ///
    /// # Panics
    /// This function panics if the index is out of bounds (i.e. greater than or equal to the
    /// GDT's capacity)
    pub fn clear_entry(&mut self, index: usize) {
        assert!(index < N, "out of bounds index when clearing a GDT entry");
        self.descriptors[index] = Entry::NULL;
    }

    /// Set the GDT register to point to the GDT and load it into the CPU.
    ///
    /// # Safety
    /// This function is unsafe because it can crash the kernel if the GDT is not properly
    /// configured.
    #[allow(clippy::cast_possible_truncation)]
    pub unsafe fn flush(&mut self) {
        self.register.limit = (N * core::mem::size_of::<Entry>() - 1) as u16;
        self.register.base = self.descriptors.as_ptr() as u64;
        self.register.load();
    }
}

#[derive(Clone)]
#[repr(C, packed)]
struct Register {
    limit: u16,
    base: u64,
}

impl Register {
    /// Create a new GDT register which points to NULL.
    pub const fn null() -> Self {
        Self { limit: 0, base: 0 }
    }

    /// Returns a raw pointer to the GDT register.
    pub fn pointer(&self) -> u64 {
        self as *const Self as u64
    }

    /// Load the GDT register into the CPU.
    pub unsafe fn load(&self) {
        instruction::lgdt(self.pointer());
    }
}

#[derive(Clone)]
pub enum Descriptor {
    System(u64, u64),
    Segment(u64),
}

impl Descriptor {
    pub const NULL: Self = Self::Segment(0);
    pub const KERNEL_CODE64: Self = Self::Segment(0x00af_9b00_0000_ffff);
    pub const KERNEL_DATA: Self = Self::Segment(0x00cf_9300_0000_ffff);
    pub const USER_DATA: Self = Self::Segment(0x00cf_f300_0000_ffff);
    pub const USER_CODE64: Self = Self::Segment(0x00af_fb00_0000_ffff);

    /// Create a new TSS descriptor.
    #[must_use]
    pub fn tss(tss: &TaskStateSegment) -> Self {
        let mut low = DescriptorFlags::PRESENT.bits();
        let address = tss as *const _ as u64;

        // Set the limit to the size of the TSS minus 1 (because the limit is inclusive)
        low.set_bit_range(15, 0, (core::mem::size_of::<TaskStateSegment>() - 1) as u64);

        // Set the low 32 bits of the base address
        low.set_bit_range(39, 16, address & 0xFF_FFFF);
        low.set_bit_range(63, 56, (address >> 24) & 0xFF);

        // Set the type to 0b1001 (x86_64 available TSS)
        low.set_bit_range(43, 40, 0b1001);

        Self::System(low, (address >> 32) & 0xFFFF_FFFF)
    }
}

bitflags! {
    #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DescriptorFlags: u64 {
        const ACCESSED          = 1 << 40;
        const WRITABLE          = 1 << 41;
        const CONFORMING        = 1 << 42;
        const EXECUTABLE        = 1 << 43;
        const USER_SEGMENT      = 1 << 44;
        const DPL_RING_3        = 3 << 45;
        const PRESENT           = 1 << 47;
        const AVAILABLE         = 1 << 52;
        const LONG_MODE         = 1 << 53;
        const DEFAULT_SIZE      = 1 << 54;
        const GRANULARITY       = 1 << 55;
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct Entry(u64);

impl Entry {
    const NULL: Self = Self(0);
    const fn new(x: u64) -> Self {
        Self(x)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Selector(pub u16);

impl Selector {
    pub const NULL: Selector = Selector::new(0, Privilege::KERNEL);
    pub const KERNEL_CODE: Selector = Selector::new(1, Privilege::KERNEL);
    pub const KERNEL_DATA: Selector = Selector::new(2, Privilege::KERNEL);
    pub const USER_DATA: Selector = Selector::new(3, Privilege::USER);
    pub const USER_CODE: Selector = Selector::new(4, Privilege::USER);

    /// Create a new segment selector. The index is the index of the segment in the GDT, and the
    /// privilege is the privilege level used for this segment.
    #[must_use]
    pub const fn new(index: u16, privilege: Privilege) -> Self {
        Self((index * 8) | (privilege as u16))
    }

    /// Get the index of the segment in the GDT.
    #[must_use]
    pub const fn index(self) -> u16 {
        self.0 / 8
    }
}

/// Setup the GDT for the current CPU, load it and set the different segment selectors registers.
///
/// # Safety
/// This function is unsafe because it can crash the kernel if the GDT is not properly configured,
/// or if the selectors are not properly set.
#[init]
pub unsafe fn setup() {
    GDT.lock().set_descriptor(0, &Descriptor::NULL);
    GDT.lock().set_descriptor(1, &Descriptor::KERNEL_CODE64);
    GDT.lock().set_descriptor(2, &Descriptor::KERNEL_DATA);
    GDT.lock().set_descriptor(3, &Descriptor::USER_DATA);
    GDT.lock().set_descriptor(4, &Descriptor::USER_CODE64);
    load();
}

/// Load the GDT into the current CPU, and set the segment registers to their default values.
///
/// # Safety
/// This function is unsafe because it can crash the kernel and/or break the memory safety if
/// the GDT is not properly configured before calling this function.
#[init]
pub unsafe fn load() {
    GDT.lock().flush();

    unsafe {
        // Some black magic to load a new code segment selector. This is a bit tricky because
        // we cant directly load the new selector into the CS register, and far jumps are not
        // allowed in 64 bits mode. So we use the 'retfq' instruction to set a new code segment
        // selector
        core::arch::asm!(
            "mov ss, {1:r}",
            "mov ds, {1:r}",
            "mov es, {1:r}",
            "push {0:r}",
            "lea {tmp}, [1f + rip]",
            "push {tmp}",
            "retfq",
            "1:",
            in(reg) Selector::KERNEL_CODE.0,
            in(reg) Selector::KERNEL_DATA.0,
            tmp = lateout(reg) _,
            options(preserves_flags),
        );
    }
}
