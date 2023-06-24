use super::{AllocationFlags, Frame};
use core::ops::Range;

pub mod dummy;

/// A trait used to allocate and deallocate physical frames. This is used to abstract the
/// underlying memory management mechanism and allowing multiple memory management policies.*
#[allow(clippy::missing_safety_doc)]
pub unsafe trait Allocator {
    unsafe fn allocate_frame(&mut self, flags: AllocationFlags) -> Option<Frame>;
    unsafe fn deallocate_range(&mut self, range: Range<Frame>);
    unsafe fn deallocate_frame(&mut self, frame: Frame);
    unsafe fn reference_frame(&mut self, frame: Frame);
    unsafe fn allocate_range(
        &mut self,
        count: usize,
        flags: AllocationFlags,
    ) -> Option<Range<Frame>>;
}
