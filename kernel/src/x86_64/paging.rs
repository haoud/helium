use super::{cpu, tlb};
use crate::mm::{
    frame::{allocator::Allocator, AllocationFlags, Frame},
    FRAME_ALLOCATOR,
};
use addr::{Physical, Virtual};
use alloc::sync::Arc;
use core::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};
use macros::init;
use sync::Lazy;

/// The kernel page table. This is used to create new page table very fast, simply by copying the
/// kernel page table into the new page table.
pub static KERNEL_PML4: Lazy<PageTableRoot> =
    Lazy::new(|| unsafe { PageTableRoot::from_page(Physical::new(cpu::read_cr3())) });

pub const PAGE_SIZE: usize = 4096;

bitflags::bitflags! {
    /// Represents the flags of a page table entry. See Intel Vol. 3A, Section 4.5 for more
    /// information about page tables.
    #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct PageEntryFlags: u64 {
        /// If set, the page is present in memory. Otherwise, the page is not present, and the
        /// bits 12-51 of the entry are ignored and free to use for other purposes.
        const PRESENT = 1 << 0;

        /// If set, the page is writable. Otherwise, the page is read-only. If the write protection
        /// bit of the CR0 register is not set, this flag is ignored.
        const WRITABLE = 1 << 1;

        /// If set, the page is accessible from user mode. Otherwise, the page is only accessible
        /// from kernel mode.
        const USER = 1 << 2;

        /// If set, the page caching strategy is set to write-through. Otherwise, the caching
        /// strategy is set to write-back. This is useful for memory-mapped I/O.
        const WRITE_THROUGH = 1 << 3;

        /// If set, the page is not cached. Otherwise, the page is cached according to the caching
        /// strategy set by the `WRITE_THROUGH` flag.
        const NO_CACHE = 1 << 4;

        /// If set, the page has been accessed. When the page is accessed, the flag is set by the
        /// processor (but never cleared by the processor). This flag can also be set manually.
        const ACCESSED = 1 << 5;

        /// If set, the page has been written to. When the page is written to, the flag is set by
        /// the processor (but never cleared by the processor). This flag can also be set manually.
        const DIRTY = 1 << 6;

        /// If set, the page is a huge page. This flags is only valid for PT entries and PD entries.
        /// IF the flags is set to a PT entry, the entry maps directly to a 2 MiB page (and the
        /// address must be aligned to 2 MiB too). If the flag is set to a PD entry, the entry maps
        /// to a 1 GiB page (and the address must be aligned to 1 GiB too).
        const HUGE_PAGE = 1 << 7;

        /// If set, the page is global. A global page is not flushed from the TLB when CR3 is
        /// modified. This is often used for kernel pages, and can improves performance.
        const GLOBAL = 1 << 8;

        const BIT_9  = 1 << 9;
        const BIT_10 = 1 << 10;
        const BIT_11 = 1 << 11;
        const BIT_52 = 1 << 52;
        const BIT_53 = 1 << 53;
        const BIT_54 = 1 << 54;
        const BIT_55 = 1 << 55;
        const BIT_56 = 1 << 56;
        const BIT_57 = 1 << 57;
        const BIT_58 = 1 << 58;
        const BIT_59 = 1 << 59;
        const BIT_60 = 1 << 60;
        const BIT_61 = 1 << 61;
        const BIT_62 = 1 << 62;

        /// If set, the page is not executable. By default, all pages are executable. This flag is
        /// only valid if the `NXE` bit of the `EFER` register is set, otherwise it is ignored.
        const NO_EXECUTE = 1 << 63;
    }

    /// Represents a set of flags pushed onto the stack by the CPU when a page fault occurs,
    /// indicating the cause of the fault.
    #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct PageFaultErrorCode: u64 {
        /// If set, the fault was caused by a page not being present. Otherwise, the fault was
        /// caused by a protection violation.
        const PRESENT = 1 << 0;

        /// If set, the fault was caused by a write access
        const WRITE_ACCESS = 1 << 1;

        /// If set, the fault was caused when the CPU was in user mode. Otherwise, the fault was
        /// caused when the CPU was in supervisor mode.
        const CPU_USER_MODE = 1 << 2;

        /// If set, the fault was caused by a malfored table entry (e.g. a reserved bit was set)
        const MALFORMED_TABLE = 1 << 3;

        /// If set, the fault was caused by an instruction fetch.
        const INSTRUCTION_FETCH = 1 << 4;
    }
}

/// Represents a level inside the page table hierarchy. This is used to traverse the page table
/// hierarchy.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Pml4,
    Pdpt,
    Pd,
    Pt,
}

impl Level {
    /// Get the next level in the page table hierarchy from this level. Returns `None` if this is
    /// the last level.
    ///
    /// # Panics
    /// Panics if the current level is the last level in the hierarchy.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Pml4 => Self::Pdpt,
            Self::Pdpt => Self::Pd,
            Self::Pd => Self::Pt,
            Self::Pt => panic!("No next level after PT"),
        }
    }

    /// Return the index of this level in the page table hierarchy. The PML4 is the highest level
    /// and has index 4, the PT is the lowest level and has index 1.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Pml4 => 4,
            Self::Pdpt => 3,
            Self::Pd => 2,
            Self::Pt => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FetchBehavior {
    /// If an entry is missing, allocate a new page table entry and continue traversing the page
    /// table hierarchy.
    Allocate,

    /// Only traverse the page table hierarchy if all entries are present. If an entry is missing,
    /// return an error.
    Reach,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FetchError {
    /// The entry was not present in the page table
    NoSuchEntry,

    /// An allocation failed while trying to fetch the page table entry.
    OutOfMemory,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapError {
    /// The kernel ran out of memory while trying to allocate a new page table
    OutOfMemory,

    /// The virtual address was already mapped to a physical address
    AlreadyMapped,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnmapError {
    /// The virtual address was not mapped to a physical address
    NotMapped,
}

#[repr(C)]
pub struct PageEntry(u64);

impl PageEntry {
    pub const ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;
    pub const EMPTY: Self = Self(0);

    /// Create a new page table entry with the given address and flags. The address must be page
    /// aligned, because the last 12 bits are used for flags by the CPU.
    ///
    /// # Panics
    /// Panics if the address is not page aligned.
    #[must_use]
    pub fn new(addr: Physical, flags: PageEntryFlags) -> Self {
        assert!(addr.is_page_aligned(), "Address {addr} is not page aligned");
        Self((u64::from(addr) & Self::ADDRESS_MASK) | flags.bits())
    }

    /// Set the address of the page table entry. This function does not modify any flags of the
    /// entry.
    ///
    /// # Panics
    /// Panics if the address is not page aligned.
    pub fn set_address(&mut self, addr: Physical) {
        assert!(addr.is_page_aligned(), "Address {addr} is not page aligned",);
        self.0 = (self.0 & !Self::ADDRESS_MASK) | (u64::from(addr) & Self::ADDRESS_MASK);
    }

    /// Set the flags of the page table entry. This function does not modify the address of the
    /// entry, and simply overwrites the flags of the entry with the given flags.
    pub fn set_flags(&mut self, flags: PageEntryFlags) {
        self.0 = (self.0 & Self::ADDRESS_MASK) | flags.bits();
    }

    /// Clear the given flags of the page table entry. This function does not modify the address of
    /// the entry, and simply clears the given flags of the entry.
    pub fn clear_flags(&mut self, flags: PageEntryFlags) {
        self.0 &= !flags.bits();
    }

    /// Add the given flags to the page table entry. This function does not modify the address of
    /// the entry, and simply adds the given flags to the entry.
    pub fn add_flags(&mut self, flags: PageEntryFlags) {
        self.0 |= flags.bits();
    }

    /// Returns `true` if the page is present in memory, `false` otherwise.
    #[must_use]
    pub const fn present(&self) -> bool {
        self.flags().contains(PageEntryFlags::PRESENT)
    }

    /// Returns `true` if the page is executable, `false` otherwise.
    #[must_use]
    pub const fn executable(&self) -> bool {
        !self.flags().contains(PageEntryFlags::NO_EXECUTE)
    }

    /// Returns `true` if the page is writable, `false` otherwise.
    #[must_use]
    pub const fn writable(&self) -> bool {
        self.flags().contains(PageEntryFlags::WRITABLE)
    }

    /// Returns `true` if the page is dirty, `false` otherwise. A page is dirty if it has been
    /// written to, or if the flag has been set manually.
    #[must_use]
    pub const fn dirty(&self) -> bool {
        self.flags().contains(PageEntryFlags::DIRTY)
    }

    /// Returns `true` if the page has been accessed, `false` otherwise. A page is accessed if it
    /// has been read from, or if the flag has been set manually.
    #[must_use]
    pub const fn accessed(&self) -> bool {
        self.flags().contains(PageEntryFlags::ACCESSED)
    }

    /// Returns `true` if the page not user accessible, `false` otherwise.
    #[must_use]
    pub const fn kernel(&self) -> bool {
        !self.user()
    }

    /// Returns `true` if the page is user accessible, `false` otherwise.
    #[must_use]
    pub const fn user(&self) -> bool {
        self.flags().contains(PageEntryFlags::USER)
    }

    /// Set the entry to 0, indicating that the page is not present in memory.
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Returns the flags of this entry.
    #[must_use]
    pub const fn flags(&self) -> PageEntryFlags {
        PageEntryFlags::from_bits_truncate(self.0)
    }

    /// Consider that the address of this entry is a pointer to a page table, and return a
    /// pointer to this page table. If the entry is not present, `None` is returned.
    ///
    /// # Safety
    /// This function is safe because it only returns a pointer to a page table if the entry is
    /// present. It is up to the caller to ensure that he will manipulate the pointer correctly.
    #[must_use]
    pub fn table(&self) -> Option<*mut PageTable> {
        if self.flags().contains(PageEntryFlags::PRESENT) {
            let addr = self.0 & Self::ADDRESS_MASK;
            Some(Virtual::from(Physical::new(addr)).as_mut_ptr::<PageTable>())
        } else {
            None
        }
    }

    /// Returns the physical address of the page mapped by this entry. If the entry is not present,
    /// `None` is returned.
    #[must_use]
    pub const fn address(&self) -> Option<Physical> {
        if self.flags().contains(PageEntryFlags::PRESENT) {
            Some(Physical::new(self.0 & Self::ADDRESS_MASK))
        } else {
            None
        }
    }
}

/// Represents a page table. A page table is a 4 KiB page aligned structure that contains 512 page
/// table entries. Each entry can either point to an another page table or to a page, depending on
/// the level of the page table.
#[repr(C, align(4096))]
pub struct PageTable([PageEntry; 512]);

impl PageTable {
    pub const COUNT: usize = 512;

    /// Creates a new empty page table where all entries are set to 0.
    #[must_use]
    pub const fn empty() -> Self {
        Self([PageEntry::EMPTY; Self::COUNT])
    }

    /// Fetch the corresponding page table entry for the given virtual address. If the entry does
    /// not exist, it will be created if `behavior` is `FetchBehavior::Create`, otherwise an error
    /// will be returned.
    ///
    /// # Safety
    /// This function is unsafe because the caller must ensure that he will not create a multiple
    /// mutable references to the same page table entry.
    unsafe fn fetch(
        table: &mut PageTable,
        level: Level,
        addr: Virtual,
        behavior: FetchBehavior,
    ) -> Result<&mut PageEntry, FetchError> {
        let entry = &mut table.0[addr.page_index(level.index())];

        if level == Level::Pt {
            return Ok(entry);
        }

        // Read the entry at the given index. If the entry is not present and the user wants to
        // allocate all missing entries, allocate a new frame and set the address of the entry to
        // the start of the frame. Otherwise, return an error indicating that the entry is not
        // present.
        if !entry.present() {
            match behavior {
                FetchBehavior::Allocate => {
                    let flags = AllocationFlags::KERNEL | AllocationFlags::ZEROED;
                    let frame = FRAME_ALLOCATOR
                        .lock()
                        .allocate_frame(flags)
                        .ok_or(FetchError::OutOfMemory)?;

                    // If the address is not user accessible, we must not set the user flag.
                    // However, even if the final page will not be writable, we must set the
                    // writable flag, otherwise the whole virtual range covered by the page table
                    // will not be writable, even if the final page will be marked as writable.
                    if addr.is_user() {
                        entry.add_flags(PageEntryFlags::USER);
                    }
                    entry.add_flags(PageEntryFlags::WRITABLE);
                    entry.add_flags(PageEntryFlags::PRESENT);
                    entry.set_address(frame.addr());
                }
                FetchBehavior::Reach => return Err(FetchError::NoSuchEntry),
            }
        }

        let table = &mut *entry.table().unwrap();
        PageTable::fetch(table, level.next(), addr, behavior)
    }

    /// Creates a new page table from a page, and returns a reference to it.
    ///
    /// # Safety
    /// This is unsafe because the caller must ensure that the page is not freed while the table is
    /// in use. It must also ensure that the page is not aliased by other mutable references to the
    /// page.
    ///
    /// # Panics
    /// This function panics if the page is not page aligned.
    #[must_use]
    pub unsafe fn from_page(page: Virtual) -> &'static Self {
        assert!(page.is_page_aligned(), "Page {page} is not page aligned");
        &*(page.as_ptr::<Self>())
    }

    /// Creates a new page table from a page, and returns a mutable reference to it.
    ///
    /// # Safety
    /// This is unsafe because the caller must ensure that the page is not freed while the table is
    /// in use. It must also ensure that the page is not aliased by other any other reference to
    /// the page.
    ///
    /// # Panics
    /// This function panics if the page is not page aligned.
    #[must_use]
    pub unsafe fn from_page_mut(page: Virtual) -> &'static mut Self {
        assert!(page.is_page_aligned(), "Page is not page aligned");
        &mut *(page.as_mut_ptr::<Self>())
    }

    /// Clears all entries in the page table. This does not free any memory, it just marks all
    /// entries as not present ans clears all flags and addresses.
    pub fn clear(&mut self) {
        for entry in self.0.iter_mut() {
            entry.clear();
        }
    }

    /// Returns the virtual address of this page table.
    #[must_use]
    pub fn to_virtual(&self) -> Virtual {
        Virtual::new(self as *const Self as u64)
    }

    /// Returns `true` if all entries in the page table are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(PageEntry::present)
    }
}

impl Deref for PageTable {
    type Target = [PageEntry];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Represents a page table root, which is the first page table in the page table hierarchy. This
/// structure also contains a lock that prevents concurrent access to the page table.
pub struct PageTableRoot {
    lock: AtomicBool,
    pml4: Virtual,
    frame: Frame,
}

unsafe impl Send for PageTableRoot {}
unsafe impl Sync for PageTableRoot {}

impl PageTableRoot {
    /// Create a new page table root. This will allocate a new frame and copy the kernel page table
    /// into it. The result will be an empt user space, and a kernel space that is identical to the
    /// other page table root.
    ///
    /// # Panics
    /// Panics if the kernel page table is not initialized or if the kernel ran out of memory.
    pub fn new() -> Self {
        unsafe {
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame(AllocationFlags::KERNEL)
                .expect("Failed to allocate frame for page table root");

            let dst = Virtual::from(frame.addr()).as_mut_ptr::<u8>();
            let src = KERNEL_PML4.pml4.as_ptr::<u8>();

            core::ptr::copy_nonoverlapping(src, dst, PAGE_SIZE);
            Self::from_page(frame.addr())
        }
    }

    /// Use a frame as a page table root. This will take ownership of the frame and return a new
    /// page table root that points to the frame. The frame must contain a valid PML4 table.
    ///
    /// # Safety
    /// This function is unsafe because the caller must ensure that the page passed as argument is
    /// exclusively owned by the caller, and the ownership of the page is transferred to the page
    /// table root. The caller must also ensure that the page is not freed while the page table
    /// root is in use. The page will be automatically freed when the page table root is dropped.
    #[must_use]
    pub unsafe fn from_page(page: Physical) -> Self {
        Self {
            frame: Frame::new(page),
            lock: AtomicBool::new(false),
            pml4: Virtual::from(page),
        }
    }

    /// Switch between two page table roots, the current one and the next one. This will update the
    /// CR3 register to point to the next page table root, but only if the next page table root is
    /// different from the current one, avoiding unnecessary TLB flushes.
    ///
    /// # Safety
    /// This function is unsafe because the caller must ensure that:
    ///  - the page table root is not freed while it is in use.
    ///  - The page table root is correctly initialized.
    ///  - The current page table root  passed as argument is the one currently in use.
    /// Failure to ensure these conditions will result in undefined behavior or a panic.
    pub unsafe fn switch(current: &Arc<PageTableRoot>, next: &Arc<PageTableRoot>) {
        // Verify that the previous page table root is the same as the current one. We only
        // check this in debug mode because it is expensive and should not happen in normal
        // conditions (it is a bug if it happens).
        debug_assert!(
            u64::from(current.frame.addr()) == cpu::read_cr3(),
            "Incorrect previous page table root"
        );

        if !Arc::ptr_eq(current, next) {
            unsafe {
                next.set_current();
            }
        }
    }

    /// Acquire an exclusive access to the page table root. This will block until the page table
    /// root is available, and return a guard that will release the lock when dropped. This works
    /// approximately like a `Mutex` and a `MutexGuard`.
    pub fn lock(&self) -> PageTableRootGuard {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        PageTableRootGuard { root: self }
    }

    /// Remplace the current page table root by this one. This will set the CR3 register to the
    /// physical address of the page table root. This change will flush all the TLB entries,
    /// except the ones that are marked as global.
    ///
    /// # Safety
    /// This function is unsafe because the caller must ensure that the page table root is not
    /// freed while it is in use. The caller must also ensure that the page table root is
    /// correctly initialized.
    pub unsafe fn set_current(&self) {
        // TODO: Read the current CR3 register and only update it if it is different from the
        // current one. This will avoid unnecessary TLB flushes and should improve performance.
        cpu::write_cr3(u64::from(self.frame.addr()));
    }
}

impl Default for PageTableRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PageTableRoot {
    /// When a page table root is dropped, recursively free all the user space frames. Kernel space
    /// frames are not freed because they are still in use by the kernel and other processes.
    fn drop(&mut self) {
        unsafe {
            debug_assert!(
                cpu::read_cr3() != u64::from(self.frame.addr()),
                "Cannot drop the current page table root"
            );
            let pml4 = PageTable::from_page_mut(self.pml4);
            deallocate_recursive(&mut pml4[0..256], Level::Pml4);
        }
    }
}

/// A guard that holds a lock on a page table root. This will release the lock when dropped.
/// This is necessary because we need to ensure that only one thread can access to an page table
/// root and its subtables at a time, to avoid any undefined behavior.
pub struct PageTableRootGuard<'a> {
    root: &'a PageTableRoot,
}

impl<'a> PageTableRootGuard<'a> {
    /// Fetch the corresponding page table entry for the given virtual address. If the entry does
    /// not exist, it will be created if `behavior` is `FetchBehavior::Create`, otherwise an error
    /// will be returned.
    ///
    /// # Safety
    /// This function is unsafe because the caller must ensure that he will not create a multiple
    /// mutable references to the same page table entry.
    unsafe fn fetch_last_entry(
        &mut self,
        addr: Virtual,
        behavior: FetchBehavior,
    ) -> Result<&mut PageEntry, FetchError> {
        PageTable::fetch(self, Level::Pml4, addr, behavior)
    }

    /// Clear the user space by setting the first 256 entries to 0. This will not free any memory,
    /// it will just set all entries to 0.
    fn clear_userspace(&mut self) {
        self.user_space_mut().iter_mut().for_each(PageEntry::clear);
    }

    /// Returns a mutable slice to the kernel space entries of the PML4 table.
    pub fn kernel_space_mut(&mut self) -> &mut [PageEntry] {
        &mut self[256..512]
    }

    /// Returns a mutable slice to the user space entries of the PML4 table.
    pub fn user_space_mut(&mut self) -> &mut [PageEntry] {
        &mut self[0..256]
    }

    /// Returns a slice to the kernel space entries of the PML4 table.
    #[must_use]
    pub fn kernel_space(&self) -> &[PageEntry] {
        &self[256..512]
    }

    /// Returns a slice to the user space entries of the PML4 table.
    #[must_use]
    pub fn user_space(&self) -> &[PageEntry] {
        &self[0..256]
    }
}

impl Drop for PageTableRootGuard<'_> {
    /// When a page table root guard is dropped, release the lock, allowing other threads to access
    /// the page table root.
    fn drop(&mut self) {
        self.root.lock.store(false, Ordering::Release);
    }
}

impl<'a> Deref for PageTableRootGuard<'a> {
    type Target = PageTable;
    fn deref(&self) -> &Self::Target {
        unsafe { PageTable::from_page(self.root.pml4) }
    }
}

impl<'a> DerefMut for PageTableRootGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { PageTable::from_page_mut(self.root.pml4) }
    }
}

/// Initializes the paging system. This function must be called before any other function in this
/// module. This function initialize the kernel page table with the current page table, and clears
/// the first 256 entries in the PML4 table, which are reserved for the user space.
///
/// We also preallocate all the kernel PML4 entries, so that when we create a new address space,
/// we can simply copy the kernel PML4 entries to the new PML4 table without worrying when a kernel
/// PML4 entry will be allocated in a address space (which would need synchronization between all
/// address spaces).
///
/// # Safety
/// This function is unsafe because the caller must ensure that it is called only once, and before
/// any other function in this module. The caller must also ensure that the current page table will
/// remain valid for the lifetime of the kernel.
#[init]
pub unsafe fn setup() {
    let mut pml4 = KERNEL_PML4.lock();

    pml4.clear_userspace();
    pml4.kernel_space_mut()
        .iter_mut()
        .filter(|entry| !entry.present())
        .for_each(|entry| {
            let flags = PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE;
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame(AllocationFlags::KERNEL | AllocationFlags::ZEROED)
                .expect("Out of memory while preallocating kernel page tables");

            entry.set_address(frame.addr());
            entry.set_flags(flags);
        });
}

/// Map a frame at the specified virtual address. If the address is already mapped, an error is
/// returned.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the frame will remain free until
/// the page is unmapped. The caller must also ensure that the mapping does not break the memory
/// safety of the kernel.
pub unsafe fn map(
    root: &PageTableRoot,
    address: Virtual,
    frame: Frame,
    flags: PageEntryFlags,
) -> Result<(), MapError> {
    let mut table = root.lock();
    let pte = table
        .fetch_last_entry(address, FetchBehavior::Allocate)
        .map_err(|_| MapError::OutOfMemory)?;

    if !pte.present() {
        pte.set_flags(PageEntryFlags::PRESENT | flags);
        pte.set_address(frame.addr());
        tlb::shootdown(address);
        return Ok(());
    }

    log::warn!(
        "Attempt to map an already mapped page (frame: {})",
        pte.address().unwrap()
    );
    Err(MapError::AlreadyMapped)
}

/// Unmap the page mapped at the specified address. If an entry is the hierarchy is not present,
/// we return an error, otherwise we clear the entry, flush the TLB on all CPUs, and return the
/// previously mapped physical frame. It is the responsibility of the caller to free the returned
/// frame.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the virtual address will not be
/// used after the function returns. The caller must also ensure that the frame returned by this
/// frame is correctly freed if allocated with the frame allocator.
pub unsafe fn unmap(root: &PageTableRoot, address: Virtual) -> Result<Frame, UnmapError> {
    let mut table = root.lock();
    let pte = table
        .fetch_last_entry(address, FetchBehavior::Reach)
        .map_err(|_| UnmapError::NotMapped)?;

    if let Some(physical) = pte.address() {
        pte.clear();
        tlb::shootdown(address);
        return Ok(Frame::new(physical));
    }

    Err(UnmapError::NotMapped)
}

/// Resolve a virtual address to a physical address. If the address is not mapped, return `None`.
/// This function allow the address passed as argument to not be page aligned, but will always
/// return a page aligned physical address.
pub fn resolve(root: &PageTableRoot, address: Virtual) -> Option<Physical> {
    unsafe {
        root.lock()
            .fetch_last_entry(address, FetchBehavior::Reach)
            .map_or(None, |pte| pte.address())
    }
}

/// Deallocates all frames recursively in a page table and frees the page table itself. We take
/// as parameter a slice of page entries to allow the caller to only deallocate recursively a
/// subset of a page table.
///
/// Even if only an subset of the page table is specified, the page table will be still
/// deallocated by the function: specify an subset of the page table only prevents the function
/// from deallocating the frames recursively in the non specified part of the page table (see
/// the `Drop` implementation of `PageTableRoot` for more details).
///
/// # Safety
/// This function is unsafe because the caller must ensure that the page table is not used
/// after this function is called.
unsafe fn deallocate_recursive(table: &mut [PageEntry], level: Level) {
    table
        .iter()
        .filter_map(PageEntry::address)
        .for_each(|address| match level {
            Level::Pml4 | Level::Pdpt | Level::Pd => {
                let table = PageTable::from_page_mut(Virtual::from(address));
                deallocate_recursive(table, level.next());
            }
            Level::Pt => {
                FRAME_ALLOCATOR.lock().deallocate_frame(Frame::new(address));
            }
        });

    let virt = Virtual::from_ptr(table.as_mut_ptr());
    let phys = Physical::from(virt.page_align_down());
    FRAME_ALLOCATOR.lock().deallocate_frame(Frame::new(phys));
}

pub fn handle_page_fault(addr: Virtual, _: PageFaultErrorCode) {
    panic!("Page fault exception at {:#x}", addr);
}
