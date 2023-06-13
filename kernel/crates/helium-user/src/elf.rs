use crate::task::Task;
use addr::Virtual;
use alloc::sync::Arc;
use core::cmp::min;
use elf::{endian::NativeEndian, segment::ProgramHeader, ElfBytes};
use macros::init;
use mm::{
    frame::{allocator::Allocator, AllocationFlags},
    FRAME_ALLOCATOR,
};
use x86_64::paging::{self, PageEntryFlags, PageTableRoot, PAGE_SIZE};

/// Parse an ELF file, load it into the passed page table, and return a new task with the entry
/// point of the ELF file as the entry point of the task.
///
/// # Safety
/// This function is safe, but since it is called only during the initialization of the kernel,
/// it does not perform any checks to verify that the ELF file is valid and compatible with the
/// kernel and the system.
#[init]
pub fn load(mm: Arc<PageTableRoot>, file: &[u8]) -> Arc<Task> {
    let elf = ElfBytes::<NativeEndian>::minimal_parse(file);
    let elf = elf.expect("failed to parse ELF file");

    // Map all the segments of the ELF file that are loadable
    for phdr in elf
        .segments()
        .unwrap()
        .iter()
        .filter(|phdr| phdr.p_type == elf::abi::PT_LOAD)
    {
        let end = Virtual::new(phdr.p_vaddr + phdr.p_memsz);
        let start = Virtual::new(phdr.p_vaddr);

        let mut page = start;
        while page < end {
            unsafe {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame(AllocationFlags::ZEROED)
                    .expect("failed to allocate frame for mapping an ELF segment");

                paging::map(&mm, page, frame, section_paging_flags(&phdr))
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
                    .offset(phdr.p_offset as isize)
                    .offset(segment_offset as isize);

                // The destination address in the virtual address space (use the HHDM to directly
                // write to the physical frame)
                let dst = Virtual::from(frame.addr())
                    .as_mut_ptr::<u8>()
                    .offset(start_offset as isize);

                // The remaning bytes to copy from the segment
                let remaning = phdr
                    .p_filesz
                    .checked_sub(segment_offset)
                    .map_or(0, |v| v as usize);

                // The number of bytes to copy in this iteration: the minimum between the
                // remaining bytes to copy and the remaining bytes in the page from the current
                // start offset
                let size = min(remaning, PAGE_SIZE - start_offset as usize);
                core::ptr::copy_nonoverlapping(src, dst, size)
            }

            // Advance to the next page
            page = page.page_align_down() + PAGE_SIZE;
        }
    }

    Task::new(mm, elf.ehdr.e_entry)
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
