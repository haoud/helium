use crate::{
    limine::LIMINE_MODULES,
    mm::{frame::allocator::Allocator, FRAME_ALLOCATOR},
};
use addr::{frame::Frame, phys::Physical, virt::Virtual};
use hashbrown::HashMap;

static mut MODULES: Option<HashMap<String, Vec<u8>>> = None;

/// Setup the module system. It retrieves the module data from Limine, copies it to the heap
/// and put it in a hashmap with the module path as key.
/// After all modules are copied, it frees the physical frames used by the modules since
/// there are no longer needed.
///
/// This function can be quite slow since it has to copy the module data from the
/// boot modules to the heap in order to have modules located in a "safe" place.
/// This also allow simpler code when deallocating modules since we can just
/// remove the module from the hashmap and the memory will be freed automatically,
/// instead of having to manually free the physical frames used by the module.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the data provided
/// by Limine is invalid (but we can assume that it is valid since if we can't trust
/// Limine, what can we trust?). This function must also be called only once and only
/// during the kernel initialization.
/// Failure to ensure these conditions above may cause undefined behavior.
#[init]
#[allow(clippy::cast_possible_truncation)]
pub unsafe fn setup() {
    let modules = LIMINE_MODULES
        .get_response()
        .expect("No limine modules response")
        .modules();

    MODULES = Some(HashMap::new());

    for module in modules {
        let path = String::from_utf8_lossy(module.path());
        let data = core::slice::from_raw_parts(module.addr().cast_const(), module.size() as usize);

        MODULES
            .as_mut()
            .unwrap()
            .insert(path.to_string(), data.to_vec());

        // Free the frames used by the module since we don't need them anymore
        let addr = module.addr() as usize;
        let start = Frame::new(Physical::from(Virtual::new(addr)));
        let end = Frame::new(Physical::from(
            Virtual::new(addr + module.size() as usize).page_align_up(),
        ));

        FRAME_ALLOCATOR.lock().deallocate_range(start..end);
    }
}

/// Read the module data at the given path and return a slice to it if it exists.
///
/// # Panics
/// This function will panic if the module subsystem is not initialized.
pub fn read(path: &str) -> Option<&[u8]> {
    unsafe {
        MODULES
            .as_ref()
            .expect("Module system not initialized")
            .get(path)
            .map(alloc::vec::Vec::as_slice)
    }
}

/// Free the module data at the given path. If the module does not exist, this function does
/// nothing, otherwise it will return the number of bytes freed.
///
/// # Panics
/// This function will panic if the module subsystem is not initialized.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the module data is still
/// used after being freed. To safely use this function, you must ensure that there are no longer
/// any references to the module data before calling this function. Failure to do so will cause
/// undefined behavior.
#[allow(clippy::must_use_candidate)]
pub unsafe fn free(path: &str) -> Option<usize> {
    let size = MODULES
        .as_mut()
        .expect("Module system not initialized")
        .remove(path)
        .map(|data| data.len());

    MODULES
        .as_mut()
        .expect("Module system not initialized")
        .shrink_to_fit();

    size
}
