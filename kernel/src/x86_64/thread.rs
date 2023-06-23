use super::{
    gdt::Selector,
    msr,
    paging::{self, PageEntryFlags, PageTableRoot, PAGE_SIZE},
    percpu, tss,
};
use crate::mm::{
    frame::{allocator::Allocator, AllocationFlags, Frame},
    FRAME_ALLOCATOR,
};
use addr::Virtual;
use alloc::sync::Arc;
use core::ops::Range;

core::arch::global_asm!(include_str!("asm/thread.asm"));

extern "C" {
    fn switch_context(prev: *mut *mut State, next: *mut *mut State);
    fn enter_userland(stack: u64) -> !;
}

/// The state of a thread is saved in this structure when the thread is not running. This
/// structure is higly optimized to be as small as possible, and to make context switching
/// as fast as possible.
///
/// In order to achieve this, the state saved are not really the state of the user thread, but
/// the state of the kernel when switching to the an another thread. The real state of the thread
/// are already saved by the interrupt handler when the thread is interrupted. This allow to evict
/// some register from this structure.
/// In addition, the `switch_context` function use advantage of the fact that the system V ABI
/// specify that some register must be saved by the caller, and some other by the callee. This
/// allow to save some additional registers without having to save them manually, the compiler will
/// do it for us, but in a more efficient way.
#[derive(Clone, PartialEq, Eq)]
#[repr(C)]
struct State {
    pub rip: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rflags: u64,
}

impl Default for State {
    #[allow(clippy::fn_to_numeric_cast)]
    fn default() -> Self {
        Self {
            rip: enter_userland as u64,
            rbp: 0,
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0x02, // Interrupts enabled
        }
    }
}

/// The kernel stack of a thread. This structure is used to know where the kernel stack
/// of a thread is located, and to know the base address of the stack. This also contain
/// a pointer to the saved state of the thread if the thread is not running.
struct KernelStack {
    frames: Range<Frame>,
    state: *mut State,
}

/// This is safe to implentend Send for the kernel stack because the kernel stack is
/// only used in the context of a single thread, meaning that it would not be send
/// across multiple thread, but we still need to implement Send because the compiler
/// does not know that and this is required to store the kernel stack in a global
/// variable.
unsafe impl Send for KernelStack {}

impl KernelStack {
    /// Create a new kernel stack with the given frames.
    pub fn new(frames: Range<Frame>) -> Self {
        Self {
            frames,
            state: core::ptr::null_mut(),
        }
    }

    /// Write into the stack the trampoline that will be used to switch to the thread for the
    /// first time. A small space of the stack is reserved for this purpose (see the `base`
    /// method for more information).
    fn write_initial_trampoline(&mut self, entry: u64, stack: u64) {
        let base = self.base().as_mut_ptr::<u64>();
        let cs = u64::from(Selector::USER_CODE.0);
        let ss = u64::from(Selector::USER_DATA.0);
        let rflags = 0x02;

        // Write in the stack the trampoline that will be used to switch to the thread for the
        // first time. It is simply the registers that will be restored by the iretq instruction.
        unsafe {
            base.offset(0).write(entry);
            base.offset(1).write(cs);
            base.offset(2).write(rflags);
            base.offset(3).write(stack);
            base.offset(4).write(ss);
        }
    }

    /// Write the initial state of the thread on the stack. This function must be called
    /// only when the thread is created, and write at the start of the stack the initial
    /// state of the thread. This is the state that will be restored when the thread will
    /// be executed for the first time.
    fn write_initial_state(&mut self, state: State) {
        unsafe {
            self.state = self.base().as_mut_ptr::<State>().offset(-1);
            self.state.write(state);
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
        core::ptr::addr_of_mut!(self.state)
    }

    /// Verify if the kernel stack contains the state of the thread.
    pub fn has_saved_state(&self) -> bool {
        self.state.is_null()
    }
}

/// A thread is a sequence of instructions that can be executed by the CPU. It is
/// associated with a kernel stack, a paging table, and with FS and GS segment
/// base addresses.
pub struct Thread {
    /// The paging table used by this thread
    mm: Arc<PageTableRoot>,

    /// The kernel stack for this thread
    kstack: KernelStack,

    /// The user base address for the GS segment
    gsbase: u64,

    /// The user base address for the FS segment
    fsbase: u64,
}

impl Thread {
    /// The number of frames used for the kernel stack
    const KSTACK_FRAMES: usize = 4;

    /// Create a new thread with the given kernel stack, paging table, and FS and GS
    /// segment base addresses. It will allocate a kernel stack and write the initial
    /// state of the thread on the stack. It will also allocate the user stack with
    /// the given top address and size.
    ///
    /// # Panics
    /// Panics an allocation failed, either the kernel stack or the user stack. Also panic
    /// if the user stack cannot be mapped for a reason or another.
    #[must_use]
    pub fn new(mm: Arc<PageTableRoot>, entry: u64, rsp: u64, size: u64) -> Self {
        let mut kstack = KernelStack::new(unsafe {
            FRAME_ALLOCATOR
                .lock()
                .allocate_range(Self::KSTACK_FRAMES, AllocationFlags::KERNEL)
                .expect("Failed to allocate kernel stack")
        });

        // Create the stack frame for the first time the thread will be executed, so
        // the `enter_user` function will be abled to return to the thread with the
        // `iretq` instruction. We can safely use the base address of the stack, because
        // the stack base is 64 bytes before the real end of the stack, allowing us to
        // use this space to create the stack frame.
        kstack.write_initial_trampoline(entry, rsp);
        kstack.write_initial_state(State::default());

        // Compute the start and end address of the user stack
        let stack_start = Virtual::new(rsp - size).page_align_down();
        let stack_end = Virtual::new(rsp).page_align_up();

        // Map the user stack at the given address with the given size
        // It allocate a frame for each page of the stack and then map
        // it in the paging table with user and write access.
        for virt in (stack_start..stack_end).step_by(PAGE_SIZE) {
            unsafe {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame(AllocationFlags::KERNEL)
                    .expect("Failed to allocate user stack");

                let flags = PageEntryFlags::USER | PageEntryFlags::WRITABLE;
                paging::map(&mm, virt, frame, flags)
                    .unwrap_or_else(|_| panic!("Failed to map the user stack !"));
            };
        }

        Self {
            mm,
            kstack,
            gsbase: 0,
            fsbase: 0,
        }
    }

    /// Clone and return a `Arc` to the paging table used by this thread.
    #[must_use]
    pub fn mm(&self) -> Arc<PageTableRoot> {
        Arc::clone(&self.mm)
    }
}

/// Switch from the current thread to the given thread. This function will save the
/// current thread state, and restore the given thread state while changing the
/// kernel stack to the next thread kernel stack. It will also switch the paging table
/// if needed, and switch the GS and FS segment base addresses to the next thread ones.
///
/// # Safety
/// This function is unsafe because calling it can have unexpected results (probably too
/// many to list here). It is also unsafe because it deals with raw pointers, low level
/// registers...
/// The caller must ensure that the previous thread is effectively the current thread
/// running on the CPU (but not for long).
pub unsafe fn switch(prev: &mut Thread, next: &mut Thread) {
    // Here, we need to read the kernel GS base to get the user GS base because
    // the kernel use the `swapgs` instruction when entering in kernel mode, so
    // the user GS base is saved in the KERNEL_GS_BASE and the kernel GS base is
    // set in the GS_BASE register.
    prev.gsbase = msr::read(msr::Register::KERNEL_GS_BASE);
    prev.fsbase = msr::read(msr::Register::FS_BASE);

    msr::write(msr::Register::KERNEL_GS_BASE, next.gsbase);
    msr::write(msr::Register::FS_BASE, next.fsbase);

    set_kernel_stack(&next.kstack);
    PageTableRoot::switch(&prev.mm, &next.mm);
    switch_context(prev.kstack.state_ptr_mut(), next.kstack.state_ptr_mut());
}

/// Jump to the given thread. This function should be used when there is no need
/// to save any thread state, for example when the kernel is starting the first
/// thread after the boot process.
/// Because this function doesn't save any thread state, it will never return to
/// the caller. The thread will only be controlled by the interruption handlers.
///
/// # Safety
/// This function is unsafe for approximately the same reasons as the `switch`
/// function.
pub unsafe fn jump_to(thread: &mut Thread) -> ! {
    unsafe {
        msr::write(msr::Register::KERNEL_GS_BASE, thread.gsbase);
        msr::write(msr::Register::FS_BASE, thread.fsbase);

        thread.mm.set_current();
        set_kernel_stack(&thread.kstack);
        enter_userland(u64::from(thread.kstack.base()))
    }
}

/// Set the given kernel stack as the current kernel stack. This will update the
/// TSS, so the CPU can switch to the new kernel stack when an interrupt occurs,
/// and also set the kernel stack in the percpu structure, to switch stack when
/// entering in the syscall handler.
fn set_kernel_stack(stack: &KernelStack) {
    unsafe {
        percpu::set_kernel_stack(stack.base());
        tss::set_kernel_stack(stack.base());
    }
}
