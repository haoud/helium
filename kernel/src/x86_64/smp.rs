use super::{ap_setup, msr, MAX_CPUS};
use crate::{limine::LIMINE_SMP, user};
use alloc::vec::Vec;
use core::{
    cell::OnceCell,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use limine::LimineSmpInfo;
use macros::{init, per_cpu};

/// Represent the number of CPUs that have started. After the initialization of the kernel, this
/// variable could be used to determine the number of CPUs in the system.
pub static CPU_COUNT: AtomicU64 = AtomicU64::new(1);

/// A boolean to know if the APs have finished their initialization. This is used because some
/// code can only be executed after the APs have finished their initialization. For example, if
/// the panic handler try to send an IPI to an CPU that is not yet initialized, the computer will
/// triple fault try to call an uninitialized interrupt handler.
pub static AP_BOOTED: AtomicBool = AtomicBool::new(false);

/// A boolean to know if the APs can terminate their initialization. This is used to avoid APs to
/// start before the BSP has finished its initialization.
static GO: AtomicBool = AtomicBool::new(false);

/// A per CPU variable that contains the ID of the current CPU. This is used to know which CPU is
/// currently running.
#[per_cpu]
static CPU_ID: OnceCell<u32> = OnceCell::new();

// Symbols defined in the linker script that represent the start and the end of the per
// CPU section. This section contains the initial per CPU data that is copied for each CPU.
// TODO: Free this memory after the initialization of the kernel because it is not needed
// since each CPU has its own copy of the per CPU data.
extern "C" {
    static __percpu_start: u64;
    static __percpu_end: u64;
}

/// This function is called by the BSP to start the APs. It tells Limine to start the APs and then
/// wait for them to start. Most of the work was done by Limine, so we don't have much to do here.
///
/// # Panics
/// This function panics if there is no SMP response from Limine, if there is too many cores (more
/// than `MAX_CPUS`) or if there is no core in the list returned by Limine.
#[init]
pub unsafe fn start_cpus() {
    let reponse = LIMINE_SMP
        .get_response()
        .get_mut()
        .expect("No SMP response from Limine");

    assert!(reponse.cpus().len() <= MAX_CPUS, "Too many core found");
    assert!(!reponse.cpus().is_empty(), "No core found");

    // Start the APs
    reponse
        .cpus()
        .iter_mut()
        .filter(|cpu| cpu.lapic_id != 0)
        .for_each(|cpu| {
            cpu.goto_address = ap_start;
        });

    // Wait for all APs to start
    let cpu_count = reponse.cpus().len() as u64;
    while CPU_COUNT.load(Ordering::Relaxed) != cpu_count {
        core::hint::spin_loop();
    }

    // Tell the kernel that the APs have finished their initialization
    AP_BOOTED.store(true, Ordering::Relaxed);
}

/// Setup the per CPU structure for the current CPU. This function is called by the BSP and the APs.
///
/// # Safety
/// This function is unsafe because it deals with very low level stuff that needs the use of unsafe
/// code to work properly (pointers and MSRs)
#[init]
#[inline(never)]
pub unsafe fn per_cpu_setup(lapic_id: u32) {
    let per_cpu_start = core::ptr::addr_of!(__percpu_start) as usize;
    let per_cpu_end = core::ptr::addr_of!(__percpu_end) as usize;
    let per_cpu_size = per_cpu_end - per_cpu_start;

    // Allocate the memory for the per CPU data and copy the per-cpu
    // data from the kernel to the allocated memory
    let per_cpu: *mut u8 = Vec::with_capacity(per_cpu_size).leak().as_mut_ptr();
    core::ptr::copy_nonoverlapping(per_cpu_start as *const u8, per_cpu, per_cpu_size);

    // Load the per CPU structure in the kernel GS base.
    msr::write(msr::Register::KERNEL_GS_BASE, per_cpu as u64);
    msr::write(msr::Register::GS_BASE, per_cpu as u64);

    // Set the LAPIC ID of the current CPU
    CPU_ID
        .local()
        .set(lapic_id)
        .expect("CPU ID was already set on the current CPU");
    log::debug!("CPU {} started", core_id());
}

/// Return `true` if the AP has finished their initialization,
/// `false` otherwise.
#[must_use]
pub fn ap_booted() -> bool {
    AP_BOOTED.load(Ordering::Relaxed)
}

/// Return the ID of the current core.
#[must_use]
pub fn core_id() -> u32 {
    *CPU_ID.local().get().unwrap_or(&0)
}

/// Tell the APs that they can terminate their initialization and start
/// waiting for interrupts.
pub fn go() {
    GO.store(true, Ordering::Relaxed);
}

/// Tell the kernel that this AP has finished its initialization and wait for the BSP to finish
/// its before returning.
fn ap_wait() {
    CPU_COUNT.fetch_add(1, Ordering::Relaxed);
    while !GO.load(Ordering::Relaxed) {
        core::hint::spin_loop();
    }
}

/// This function is called when an AP is started. This initialize the AP, increment the CPU count
/// and wait for the BSP to finish its initialization.
#[no_mangle]
extern "C" fn ap_start(info: *const LimineSmpInfo) -> ! {
    unsafe {
        ap_setup(&*info);
        ap_wait();
    }

    user::scheduler::engage_cpu();
}
