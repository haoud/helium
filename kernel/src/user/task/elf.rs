use crate::mm::{
    frame::{allocator::Allocator, AllocationFlags},
    vmm, FRAME_ALLOCATOR,
};
use crate::x86_64::paging::{self, PageEntryFlags, PAGE_SIZE};
use addr::virt::{InvalidVirtual, Virtual};
use alloc::sync::Arc;
use core::{cmp::min, num::TryFromIntError};
use elf::{endian::NativeEndian, segment::ProgramHeader, ElfBytes};
use sync::Spinlock;

use super::Task;

/// Error that can occur when loading an ELF file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    InvalidElf,
    InvalidAddress,
    InvalidOffset,
    UnsupportedArchitecture,
    UnsupportedEndianness,
}

impl From<InvalidVirtual> for LoadError {
    fn from(_: InvalidVirtual) -> Self {
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

/// Parse an ELF file, load it into the passed page table, and return a new task with the entry
/// point of the ELF file as the entry point of the task.
///
/// # Errors
/// Returns an `LoadError` if the the ELF file could not be loaded. On success, returns a new task
/// with the entry point of the ELF file as the entry point of the task.
///
/// # Panics
/// Panics if the kernel ran out of memory when loading the ELF file.
#[allow(clippy::cast_possible_truncation)]
pub fn load(vmm: Arc<Spinlock<vmm::Manager>>, file: &[u8]) -> Result<Arc<Task>, LoadError> {
    let elf = check_elf(ElfBytes::<NativeEndian>::minimal_parse(file)?)?;

    // Map all the segments of the ELF file that are loadable
    for phdr in elf
        .segments()
        .unwrap()
        .iter()
        .filter(|phdr| phdr.p_type == elf::abi::PT_LOAD)
    {
        let end = Virtual::try_new(phdr.p_vaddr as usize + phdr.p_memsz as usize)?;
        let start = Virtual::try_new(phdr.p_vaddr as usize)?;

        // Check that the segment is not in the kernel space
        if start.is_kernel() || end.is_kernel() {
            return Err(LoadError::InvalidAddress);
        }

        // Check that there is no overflow when computing the end address
        if start > end {
            return Err(LoadError::InvalidOffset);
        }

        let mut page = start;
        while page < end {
            unsafe {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame(AllocationFlags::ZEROED)
                    .expect("failed to allocate frame for mapping an ELF segment")
                    .into_inner();

                paging::map(vmm.lock().table(), page, frame, section_paging_flags(&phdr))
                    .unwrap_or_else(|_| panic!("Failed to map a segment of the ELF file"));

                // The start offset of the writing in the page: it is needed to handle the case
                // where the segment is not page aligned, and therefore the first page of the
                // segment is not fully filled.
                let start_offset = u64::from(page - page.page_align_down());

                // The offset of the segment in the ELF file
                let segment_offset = u64::from(page - start);

                // The source address in the ELF file
                let src = file
                    .as_ptr()
                    .offset(isize::try_from(phdr.p_offset)?)
                    .offset(isize::try_from(segment_offset)?);

                // The destination address in the virtual address space (use the HHDM to directly
                // write to the physical frame)
                let dst = Virtual::from(frame.addr())
                    .as_mut_ptr::<u8>()
                    .offset(isize::try_from(start_offset)?);

                // The remaning bytes to copy from the segment
                let remaning = phdr
                    .p_filesz
                    .checked_sub(segment_offset)
                    .map_or(0, |v| v as usize);

                // The number of bytes to copy in this iteration: the minimum between the
                // remaining bytes to copy and the remaining bytes in the page from the current
                // start offset
                let size = min(remaning, PAGE_SIZE - start_offset as usize);
                core::ptr::copy_nonoverlapping(src, dst, size);
            }

            // Advance to the next page
            page = page.page_align_down() + PAGE_SIZE;
        }
    }

    Ok(Task::user(vmm, elf.ehdr.e_entry as usize))
}

/// Convert the ELF flags of a section into the paging flags, used to map the section with
/// the correct permissions.
fn section_paging_flags(phdr: &ProgramHeader) -> PageEntryFlags {
    let mut flags = PageEntryFlags::USER;
    if phdr.p_flags & elf::abi::PF_W != 0 {
        flags |= PageEntryFlags::WRITABLE;
    }
    if phdr.p_flags & elf::abi::PF_X == 0 {
        flags |= PageEntryFlags::NO_EXECUTE;
    }
    flags
}

fn check_elf(elf: ElfBytes<NativeEndian>) -> Result<ElfBytes<NativeEndian>, LoadError> {
    // Check that the ELF file is for the x86_64 architecture
    if elf.ehdr.e_machine != elf::abi::EM_X86_64 {
        return Err(LoadError::UnsupportedArchitecture);
    }

    Ok(elf)
}
