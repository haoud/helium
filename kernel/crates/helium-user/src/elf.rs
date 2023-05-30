use crate::task::Task;
use addr::Virtual;
use alloc::sync::Arc;
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
pub fn load(mm: Arc<PageTableRoot>, file: &[u8]) -> Task {
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

        // Map and copy the data from the ELF file for each page of the segment
        for page in (start..end).step_by(PAGE_SIZE) {
            unsafe {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame(AllocationFlags::ZEROED)
                    .expect("failed to allocate frame for mapping an ELF segment");

                paging::map(&mm, page, frame, section_paging_flags(&phdr))
                    .unwrap_or_else(|_| panic!("Failed to map a segment of the ELF file"));

                let offset = u64::from(page - start);
                let dst = Virtual::from(frame.addr()).as_mut_ptr::<u8>();
                let src = file
                    .as_ptr()
                    .offset(phdr.p_offset as isize + offset as isize);

                let count = core::cmp::min(
                    PAGE_SIZE,
                    phdr.p_filesz.checked_sub(offset).map_or(0, |v| v as usize),
                );

                core::ptr::copy_nonoverlapping(src, dst, count)
            }
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
