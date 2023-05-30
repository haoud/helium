use crate::{
    gdt::Selector,
    msr,
    paging::{self, PageEntryFlags, PageTableRoot, PAGE_SIZE},
    percpu,
    tss::TSS,
};
use addr::Virtual;
use alloc::sync::Arc;
use core::{arch::global_asm, ops::Range};
use mm::{
    frame::{allocator::Allocator, AllocationFlags, Frame},
    FRAME_ALLOCATOR,
};

global_asm!(include_str!("asm/thread.asm"));

/// The state of a thread is saved in this structure when the thread is not running. This
/// structure is higly optimized to be as small as possible, and to make context switching
/// as fast as possible.
///
/// In order to achieve this, the state saved are not really the state of the user thread, but
/// the state of the kernel when switching to the an another thread. The real state of the thread
/// are already saved by the interrupt handler when the thread is interrupted. This allow to evict
/// some register from this structure.
/// In addition, the switch_context function use advantage of the fact that the system V ABI specify
/// that some register must be saved by the caller, and some other by the callee. This allow to
/// save some additional registers without having to save them manually, the compiler will do it
/// for us, but in a more efficient way.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct State {
    pub rip: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rflags: u64,
}

/// The kernel stack of a thread. This structure is used to know where the kernel stack
/// of a thread is located, and to know the base address of the stack.
pub struct KernelStack {
    frames: Range<Frame>,
    state: *mut State,
}

impl KernelStack {
    pub fn new(frames: Range<Frame>) -> Self {
        Self {
            frames,
            state: core::ptr::null_mut(),
        }
    }

    /// Return the base address of this kernel stack. Because the stack grows down, the base
    /// address is actually what is can be normally considered as the end of the stack.
    pub fn base(&self) -> Virtual {
        // We substract 64 bytes to the end of the stack, because this space is reserved for
        // when switching to the thread for the first time.
        Virtual::from(self.frames.end.end() - 64u64)
    }

    /// Return a mutable pointer to the saved state of this thread.
    pub fn state_ptr_mut(&mut self) -> *mut *mut State {
        &mut self.state
    }
}

pub struct Thread {
    /// The kernel stack for this thread
    kstack: KernelStack,

    /// The paging table used by this thread
    mm: Arc<PageTableRoot>,

    /// The user base address for the GS segment
    gsbase: u64,

    /// The user base address for the FS segment
    fsbase: u64,
}

impl Thread {
    #[must_use]
    #[allow(clippy::fn_to_numeric_cast)]
    pub fn new(mm: Arc<PageTableRoot>, rip: u64, rsp: u64, size: u64) -> Self {
        let kstack = KernelStack::new(unsafe {
            FRAME_ALLOCATOR
                .lock()
                .allocate_range(4, AllocationFlags::KERNEL)
                .expect("Failed to allocate kernel stack")
        });

        unsafe {
            // Create the stack frame for the first time the thread will be executed, so
            // the `enter_user` function will be abled to return to the thread with the
            // `iretq` instruction. We can safely use the base address of the stack, because
            // the stack base is 64 bytes before the real end of the stack, allowing us to
            // use this space to create the stack frame.
            let base = kstack.base().as_mut_ptr::<u64>();
            base.offset(0).write(rip);
            base.offset(1).write(Selector::USER_CODE.0 as u64);
            base.offset(2).write(0x02);
            base.offset(3).write(rsp);
            base.offset(4).write(Selector::USER_DATA.0 as u64);
        }

        // Allocate the user stack
        let stack_start = Virtual::new(rsp - size).page_align_down();
        let stack_end = Virtual::new(rsp).page_align_up();

        // Map the user stack
        for virt in (stack_start..stack_end).step_by(PAGE_SIZE) {
            unsafe {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame(AllocationFlags::KERNEL)
                    .expect("Failed to allocate user stack");

                let flags = PageEntryFlags::USER | PageEntryFlags::WRITABLE;
                paging::map(&mm, virt, frame, flags).expect("Failed to map the user stack !");
            };
        }

        let fsbase = 0;
        let gsbase = 0;
        Self {
            kstack,
            mm,
            gsbase,
            fsbase,
        }
    }

    pub fn kstack(&self) -> &KernelStack {
        &self.kstack
    }

    pub fn mm(&self) -> Arc<PageTableRoot> {
        Arc::clone(&self.mm)
    }
}

#[optimize(speed)]
pub unsafe fn switch(prev: &mut Thread, next: &mut Thread) {
    prev.fsbase = user_fs();
    prev.gsbase = user_gs();


    // Update the kernel stack in the TSS and in the per-CPU data
    percpu::set_kernel_stack(next.kstack.base());
    TSS.local()
        .borrow_mut()
        .set_kernel_stack(u64::from(next.kstack.base()));

    set_user_fs_gs(next.fsbase, next.gsbase);

    // Change page table if needed
    if !Arc::ptr_eq(&prev.mm, &next.mm) {
        next.mm.set_current();
    }

    // Save the current state and restore the new state
    debug_assert!(!prev.kstack.state_ptr_mut().is_null());
    debug_assert!(!next.kstack.state_ptr_mut().is_null());
    switch_context(prev.kstack.state_ptr_mut(), next.kstack.state_ptr_mut())
}

#[optimize(speed)]
pub fn jump_to_thread(thread: &mut Thread) -> ! {
    unsafe {
        percpu::set_kernel_stack(thread.kstack.base());
        TSS.local()
            .borrow_mut()
            .set_kernel_stack(u64::from(thread.kstack.base()));

        thread.mm.set_current();
        set_user_fs_gs(thread.fsbase, thread.gsbase);
        enter_user(thread.kstack.base().into())
    }
}

/// Set the user FS and GS base
fn set_user_fs_gs(fs: u64, gs: u64) {
    // The code here is CORRECT. When we enter the kernel, we use the `swapgs` instruction
    // which swaps the kernel GS base with the user GS base. So when we write the kernel GS
    // base in kernel mode, we actually change the user GS base.
    unsafe {
        msr::write(msr::Register::KERNEL_GS_BASE, gs);
        msr::write(msr::Register::FS_BASE, fs);
    }
}

/// Return the user GS base
fn user_gs() -> u64 {
    // The code here is CORRECT. When we enter the kernel, we use the `swapgs` instruction
    // which swaps the kernel GS base with the user GS base. So when we read the kernel GS
    // base in kernel mode, we actually read the user GS base.
    unsafe { msr::read(msr::Register::KERNEL_GS_BASE) }
}

/// Return the user FS base
fn user_fs() -> u64 {
    unsafe { msr::read(msr::Register::FS_BASE) }
}

extern "C" {
    fn switch_context(prev: *mut *mut State, next: *mut *mut State);
    fn enter_user(stack: u64) -> !;
}
