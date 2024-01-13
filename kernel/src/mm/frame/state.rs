use crate::limine::LIMINE_MEMMAP;

use super::{FrameFlags, Stats};
use addr::{
    frame::{self, Frame},
    phys::Physical,
    virt::Virtual,
};
use core::{mem::size_of, ops::Range};
use limine::NonNullPtr;

/// Represents the state of a physical memory frame, and contains information about the frame such
/// as its flags and its reference count.
/// It allow a generic type `T` to be stored in the frame state, which can be used to store
/// additional information about the frame, to allow having additional data when using a custom
/// allocator.
#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct FrameInfo<T> {
    pub flags: FrameFlags,
    pub count: u32,
    pub custom: T,
}

impl<T: Default + 'static> FrameInfo<T> {
    /// Create a new frame info. By default, the frame count is set to 0 (meaning that the frame is
    /// not used).
    #[must_use]
    pub fn new(flags: FrameFlags) -> Self {
        Self {
            flags,
            count: 0,
            custom: T::default(),
        }
    }

    /// Increment the frame count, meaning that the frame is used by another
    /// object/structure/thread/etc.
    ///
    /// # Panics
    /// Panics if the frame count overflows.
    pub fn retain(&mut self) {
        match self.count.checked_add(1) {
            Some(count) => self.count = count,
            None => panic!("Frame count overflow"),
        }
    }

    /// Decrement the frame count, meaning that the frame is no longer used by another
    /// object/structure/thread/etc.
    ///
    /// # Panics
    /// Panics if the frame count is already 0, meaning that the frame is not retained but
    /// [`release`] is called.
    ///
    /// # Returns
    /// Returns `true` if the frame count is 0 after the decrement, meaning that the frame is no
    /// longer used and can be reused, false if the frame is still used after the call to this
    /// function.
    pub fn release(&mut self) -> bool {
        match self.count.checked_sub(1) {
            Some(count) => self.count = count,
            None => panic!("Trying to release a frame that is not retained"),
        }
        self.count == 0
    }

    /// Check if the frame is free (i.e. if the flags [`FrameFlags::FREE`] is set). This method
    /// should only called for regular memory frames, and not for special frames such as the ACPI
    /// tables or the framebuffer.
    ///
    /// # Panics
    /// Panics if the frame count is not 0 but the flags [`FrameFlags::FREE`] is set.
    pub fn is_free(&self) -> bool {
        if self.flags.contains(FrameFlags::FREE) {
            assert!(self.count == 0, "Free frame with non-zero count");
        }
        self.flags.contains(FrameFlags::FREE)
    }
}

impl<T: Default + 'static> Default for FrameInfo<T> {
    fn default() -> Self {
        Self::new(FrameFlags::POISONED)
    }
}

/// Represents the state of all physical memory frames. This state is used to keep track of which
/// frames are allocated, free, etc.
///
/// It is important to note that this state only store information about regular memory frames, and
/// should not be used to keep track of special frames such as the ACPI tables or framebuffer. To
/// avoid allocation a overly large array when there is few memory and there is a lot of special
/// frames (such as the framebuffer) at high addresses, frame out of the range of the array are
/// considered as reserved/poisoned and should only be used if you know what you are doing.
pub struct State<T: Default + 'static> {
    pub frames: &'static mut [FrameInfo<T>],
    pub statistics: Stats,
}

impl<T: Default> State<T> {
    /// Create a uninitialized frames state. The frame array is empty and the statistics are
    /// all set to 0.
    #[must_use]
    pub const fn uninitialized() -> Self {
        Self {
            frames: &mut [],
            statistics: Stats::uninitialized(),
        }
    }

    /// Setup the frame state by parsing the memory map and filling the frame array, by
    /// parsing the memory map given by Limine.
    #[init]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(mmap: &[NonNullPtr<limine::MemmapEntry>]) -> Self {
        let last = Self::find_last_usable_frame_index(mmap);
        let array_location = Self::find_array_location(mmap, last);

        // We need first to initialize the frame array before creating a slice from it (the
        // opposite would be a direct UB).
        //
        // By default, all frames are marked as poisoned. After this loop, we will update the
        // flags for each frame accordingly to the memory map. If a frame is not in the memory
        // map, it will remain poisoned and will not be usable to prevent any potential issues.
        let array: &mut [FrameInfo<T>] = unsafe {
            let ptr = Virtual::from(array_location).as_mut_ptr::<FrameInfo<T>>();
            for i in 0..last.0 {
                ptr.add(i).write(FrameInfo::default());
            }

            core::slice::from_raw_parts_mut(ptr, last.0)
        };

        let mut statistics = Stats::default();
        statistics.poisoned.0 += last.0;
        statistics.total.0 += last.0;

        // Update the flags for each frame according to the memory map.
        for entry in mmap {
            let start = frame::Index::from_address(entry.base as usize)
                .0
                .min(last.0);
            let end = Frame::upper(entry.base + entry.len).index().0.min(last.0);

            for frame in &mut array[start..end] {
                match entry.typ {
                    limine::MemoryMapEntryType::Usable => {
                        frame.flags.remove(FrameFlags::POISONED);
                        frame.flags.insert(FrameFlags::FREE);
                        statistics.poisoned.0 -= 1;
                        statistics.usable.0 += 1;
                    }
                    limine::MemoryMapEntryType::BootloaderReclaimable => {
                        frame.flags.remove(FrameFlags::POISONED);
                        frame.flags.insert(FrameFlags::BOOT);
                        statistics.poisoned.0 -= 1;
                        statistics.allocated.0 += 1;
                        statistics.kernel.0 += 1;
                        statistics.usable.0 += 1;
                        frame.count = 1;
                    }
                    limine::MemoryMapEntryType::KernelAndModules => {
                        frame.flags.remove(FrameFlags::POISONED);
                        frame.flags.insert(FrameFlags::KERNEL);
                        statistics.poisoned.0 -= 1;
                        statistics.allocated.0 += 1;
                        statistics.kernel.0 += 1;
                        statistics.usable.0 += 1;
                        frame.count = 1;
                    }
                    limine::MemoryMapEntryType::AcpiReclaimable
                    | limine::MemoryMapEntryType::Framebuffer
                    | limine::MemoryMapEntryType::Reserved
                    | limine::MemoryMapEntryType::AcpiNvs => {
                        frame.flags.remove(FrameFlags::POISONED);
                        frame.flags.insert(FrameFlags::RESERVED);
                        statistics.poisoned.0 -= 1;
                        statistics.reserved.0 += 1;
                    }
                    limine::MemoryMapEntryType::BadMemory => (),
                }
            }
        }

        // Mark the frames used by the frame array as reserved: we don't want to
        // allocate them again.
        let count = array.len() * size_of::<Frame>() / Frame::SIZE;
        let start = frame::Index::from(array_location).0;
        let end = start + count;

        for frame in &mut array[start..=end] {
            frame.flags.insert(FrameFlags::KERNEL);
            frame.flags.remove(FrameFlags::FREE);
            statistics.allocated.0 += 1;
            statistics.kernel.0 += 1;
        }

        State {
            frames: array,
            statistics,
        }
    }

    /// Reclaim the memory used by the bootloader during the boot process. This function
    /// remove the [`FrameFlags::BOOT`] flag from the frame flags, and add the [`FrameFlags::FREE`]
    /// flag, and return a list of a range of frames that can be used by the kernel.
    ///
    /// This is the responsibility of the caller to ensure that the returned frames are taken
    /// into account by the frame allocator, because some allocators may not be able to allocate
    /// frames that were not marked as free by the frame allocator during their initialization.
    ///
    /// # Panics
    /// Panics if the memory map is not found. This should never happen, because the memory map
    /// is needed to initialize the frame state and if this function is called, it means that the
    /// frame state was correctly initialized.
    ///
    /// # Safety
    /// TODO:
    #[must_use]
    pub unsafe fn reclaim_boot_memory(&mut self) -> Vec<Range<Frame>> {
        let boot_reclaimable = LIMINE_MEMMAP
            .get_response()
            .get()
            .expect("No memory map found")
            .memmap()
            .iter()
            .filter(|entry| entry.typ == limine::MemoryMapEntryType::BootloaderReclaimable)
            .map(|entry| {
                let start = Frame::new(Physical::from(entry.base));
                let end = Frame::upper(Physical::from(entry.base + entry.len));
                start..end
            })
            .collect::<Vec<_>>();

        for range in &boot_reclaimable {
            for frame in range.start..range.end {
                let frame_info = self.frame_info_mut(frame.addr()).unwrap();
                frame_info.flags.remove(FrameFlags::BOOT);
                frame_info.flags.insert(FrameFlags::FREE);
                frame_info.count = 0;

                // Update the statistics
                self.statistics.allocated.0 -= 1;
                self.statistics.kernel.0 -= 1;
            }
        }

        boot_reclaimable
    }

    /// Return an mutable reference to the frame info for the given physical address, or `None` if
    /// the address does not exist.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn frame_info_mut(&mut self, address: Physical) -> Option<&mut FrameInfo<T>> {
        self.frames.get_mut(address.frame_index())
    }

    /// Return an immutable reference to the frame info for the given physical address, or `None`
    /// if the address does not exist.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn frame_info(&self, address: Physical) -> Option<&FrameInfo<T>> {
        self.frames.get(address.frame_index())
    }

    /// Find in the memory map a free region that is big enough to hold the frame array. This is
    /// used to place the frame array in a free region of memory.
    ///
    /// # Panics
    /// Panics if no free region enough big to hold the frame array is found. This often means that
    /// there is barely enough memory to run the kernel, and this is futile to try to resolve this
    /// issue.
    #[init]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    fn find_array_location(
        mmap: &[NonNullPtr<limine::MemmapEntry>],
        last: frame::Index,
    ) -> Physical {
        mmap.iter()
            .filter(|entry| entry.typ == limine::MemoryMapEntryType::Usable)
            .find(|entry| entry.len as usize >= last.0 * size_of::<FrameInfo<T>>())
            .map_or_else(
                || panic!("Could not find a free region to place the frame array!"),
                |entry| Physical::new(entry.base as usize),
            )
    }

    /// Find the last usable frame index of regular memory. This is used to determine the size of
    /// the frame array. As being say in the documentation of the [`State`] struct, frames out of
    /// the range of the array are considered as reserved/poisoned.
    #[init]
    #[must_use]
    fn find_last_usable_frame_index(mmap: &[NonNullPtr<limine::MemmapEntry>]) -> frame::Index {
        mmap.iter()
            .filter(|entry| {
                entry.typ == limine::MemoryMapEntryType::Usable
                    || entry.typ == limine::MemoryMapEntryType::KernelAndModules
                    || entry.typ == limine::MemoryMapEntryType::BootloaderReclaimable
            })
            .map(|entry| entry.base + entry.len)
            .max()
            .map_or(frame::Index::default(), |address| {
                frame::Index::from(Frame::upper(address))
            })
    }
}
