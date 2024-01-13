use super::Task;
use crate::{
    mm::{
        frame::{allocator::Allocator, AllocationFlags},
        FRAME_ALLOCATOR,
    },
    user::{self, vmm},
    x86_64::paging::{self, table::PageEntryFlags, PAGE_SIZE},
};
use addr::{
    user::{InvalidUserVirtual, UserVirtual},
    virt::Virtual,
};
use core::{cmp::min, num::TryFromIntError};
use elf::{endian::NativeEndian, segment::ProgramHeader, ElfBytes};

/// Create a empty user address space and load the ELF file into it.
///
/// # Errors
/// Returns an `LoadError` if the the ELF file could not be loaded. On success, returns a new task
/// with the entry point of the ELF file as the entry point of the task.
///
/// # Panics
/// Panics if the kernel ran out of memory when loading the ELF file or if the ELF file contains
/// overlapping segments
#[allow(clippy::cast_possible_truncation)]
pub fn load(file: &[u8]) -> Result<Arc<Task>, LoadError> {
    let elf = check_elf(ElfBytes::<NativeEndian>::minimal_parse(file)?)?;
    let vmm = Arc::new(Spinlock::new(vmm::Manager::new()));

    // Map all the segments of the ELF file that are loadable
    for phdr in elf
        .segments()
        .unwrap()
        .iter()
        .filter(|phdr| phdr.p_type == elf::abi::PT_LOAD)
    {
        let start = phdr.p_vaddr as usize;
        let size = phdr.p_memsz as usize;
        let end = start + size;

        let start_address = UserVirtual::try_new(start)?;
        let end_address = UserVirtual::try_new(end)?;
        let mapping_flags = section_paging_flags(&phdr);

        // Check that there is no overflow when computing the end address
        if start_address > end_address {
            return Err(LoadError::InvalidOffset);
        }

        // Reserve the area in the VMM to avoid conflicts with other mappings
        // during execution of the program. What ? How did I know that ? Well...
        let area = user::vmm::area::Area::builder()
            .range(start_address.page_align_down()..end_address.page_align_up())
            .access(user::vmm::area::Access::from(mapping_flags))
            .flags(user::vmm::area::Flags::FIXED)
            .kind(user::vmm::area::Type::Anonymous)
            .offset(0)
            .build();

        vmm.lock()
            .mmap(area)
            .expect("Failed to reserve an memory area for an ELF segment");

        let mut segment_offset = 0usize;
        let mut page = start_address;
        while page < end_address {
            unsafe {
                let mapping_vaddr = Virtual::from(page);
                let mapped_frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame(AllocationFlags::ZEROED)
                    .expect("failed to allocate frame for mapping an ELF segment")
                    .into_inner();

                paging::map(
                    vmm.lock().table(),
                    mapping_vaddr,
                    mapped_frame,
                    mapping_flags,
                )
                .expect("Failed to map a segment of the ELF file");

                // The start offset of the writing in the page: it is needed to handle the case
                // where the segment is not page aligned, and therefore the first page of the
                // segment is not fully filled.
                let start_offset = page.page_offset();

                // The source address in the ELF file
                let src = file
                    .as_ptr()
                    .offset(isize::try_from(phdr.p_offset)?)
                    .offset(isize::try_from(segment_offset)?);

                // The destination address in the virtual address space (use the HHDM to directly
                // write to the physical frame)
                let dst = Virtual::from(mapped_frame.addr())
                    .as_mut_ptr::<u8>()
                    .offset(isize::try_from(start_offset)?);

                // The remaning bytes to copy from the segment
                let remaning = phdr
                    .p_filesz
                    .checked_sub(segment_offset as u64)
                    .map_or(0, |v| v as usize);

                // The number of bytes to copy in this iteration: the minimum between the
                // remaining bytes to copy and the remaining bytes in the page from the current
                // start offset
                let size = min(remaning, PAGE_SIZE - start_offset);
                core::ptr::copy_nonoverlapping(src, dst, size);

                // Advance to the next page
                page = page.next_aligned_page();
                segment_offset += size;
            }
        }
    }

    Ok(Task::user(vmm, elf.ehdr.e_entry as usize))
}

/// Convert the ELF flags of a section into the paging flags, used to map the section with
/// the correct permissions.
fn section_paging_flags(phdr: &ProgramHeader) -> PageEntryFlags {
    let mut flags = PageEntryFlags::PRESENT | PageEntryFlags::USER;

    if phdr.p_flags & elf::abi::PF_W != 0 {
        flags |= PageEntryFlags::WRITABLE;
    }
    if phdr.p_flags & elf::abi::PF_X == 0 {
        flags |= PageEntryFlags::NO_EXECUTE;
    }
    flags
}

/// Verify that the ELF file is valid and can be run on the system.
fn check_elf(elf: ElfBytes<NativeEndian>) -> Result<ElfBytes<NativeEndian>, LoadError> {
    // Check that the ELF file is for the x86_64 architecture
    if elf.ehdr.e_machine != elf::abi::EM_X86_64 {
        return Err(LoadError::UnsupportedArchitecture);
    }
    Ok(elf)
}

/// Error that can occur when loading an ELF file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    /// The ELF headers are invalid
    InvalidElf,

    /// The ELF file contains an invalid address (e.g. in the kernel space)
    InvalidAddress,

    /// The ELF file contains an invalid offset (e.g. an overflow when computing
    /// the end address or overlapping with kernel space)
    InvalidOffset,

    /// The ELF file contains overlapping segments
    OverlappingSegments,

    /// The ELF file is for an unsupported architecture
    UnsupportedArchitecture,

    /// The ELF file is for an unsupported endianness
    UnsupportedEndianness,
}

impl From<InvalidUserVirtual> for LoadError {
    fn from(_: InvalidUserVirtual) -> Self {
        LoadError::InvalidAddress
    }
}

impl From<TryFromIntError> for LoadError {
    fn from(_: TryFromIntError) -> Self {
        LoadError::InvalidOffset
    }
}

impl From<elf::ParseError> for LoadError {
    fn from(_: elf::ParseError) -> Self {
        LoadError::InvalidElf
    }
}
