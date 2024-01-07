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
    let response = LIMINE_MODULES
        .get_response()
        .get()
        .expect("No limine modules response");

    MODULES = Some(HashMap::new());

    // SAFETY: This is safe because the data provided by Limine is assumed to
    // be valid (if we can't trust Limine, what can we trust?). The code below
    // use this assumption to create slices from the raw pointers provided by
    // Limine. If the data is invalid, we will have undefined behavior, but
    // there is nothing we can do about it.
    let count = response.module_count as usize;
    let mods = response.modules.as_ptr();
    let modules = core::slice::from_raw_parts(mods, count);

    for module in modules {
        // Copy the module data to the heap and insert it in the hashmap
        let path = &module.path.to_str().unwrap().to_str().unwrap();
        let ptr = module.base.as_ptr().unwrap();
        let len = module.length as usize;
        let data = core::slice::from_raw_parts(ptr, len);
        MODULES
            .as_mut()
            .unwrap()
            .insert((*path).to_string(), data.to_vec());

        // Free the frames used by the module since we don't need them anymore
        let addr = ptr as usize;
        let start = Frame::new(Physical::from(Virtual::new(addr)));
        let end = Frame::new(Physical::from(
            Virtual::new(addr + module.length as usize).page_align_up(),
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
/// nothing.
///
/// # Panics
/// This function will panic if the module subsystem is not initialized.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the module data is still
/// used after being freed. To safely use this function, you must ensure that there are no longer
/// any references to the module data before calling this function. Failure to do so will cause
/// undefined behavior.
pub unsafe fn free(path: &str) {
    MODULES
        .as_mut()
        .expect("Module system not initialized")
        .remove(path);
    MODULES
        .as_mut()
        .expect("Module system not initialized")
        .shrink_to_fit();
}
