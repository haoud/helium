use super::{
    gdt::Selector,
    msr,
    paging::{self, PageEntryFlags, PageTableRoot, PAGE_SIZE},
    percpu, tss,
};
use crate::{
    mm::{
        frame::{allocator::Allocator, AllocationFlags, Frame},
        FRAME_ALLOCATOR,
    },
    user::task::Task,
};
use addr::virt::Virtual;
use alloc::sync::Arc;
use core::ops::{Range, Sub};
use lib::align::Align;

core::arch::global_asm!(include_str!("asm/thread.asm"));

extern "C" {
    fn switch_context(prev: *mut *mut State, next: *mut *mut State);
    fn restore_context(next: *mut *mut State) -> !;
    fn enter_thread() -> !;

    #[allow(improper_ctypes)]
    fn exit_thread(current: *const Task, next: &mut Thread, stack: u64) -> !;
}

/// When a kernel thread is created, the function that will be executed by the thread
/// should have the same signature as this type.
pub type KernelThreadFn = fn() -> !;

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
    pub rflags: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
}

impl Default for State {
    #[allow(clippy::fn_to_numeric_cast)]
    fn default() -> Self {
        Self {
            rflags: 0,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rip: enter_thread as u64,
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

    /// Allocate a new kernel stack with the given number of frames.
    ///
    /// # Panics
    /// Panics if the kernel stack could not be allocated.
    pub fn allocate(frames: usize) -> Self {
        KernelStack::new(unsafe {
            FRAME_ALLOCATOR
                .lock()
                .allocate_range(frames, AllocationFlags::KERNEL)
                .expect("Failed to allocate kernel stack")
        })
    }

    fn write_initial_kernel_trampoline(&mut self, entry: u64) {
        let cs = u64::from(Selector::KERNEL_CODE.0);
        let ss = u64::from(Selector::KERNEL_DATA.0);
        self.write_initial_trampoline(entry, u64::from(self.base()), cs, ss);
    }

    fn write_initial_user_trampoline(&mut self, entry: u64, stack: u64) {
        let cs = u64::from(Selector::USER_CODE.0);
        let ss = u64::from(Selector::USER_DATA.0);
        self.write_initial_trampoline(entry, stack, cs, ss);
    }

    /// Write the initial trampoline of the thread on the stack. This function will write
    /// on the kernel stack a fake interrupt frame that will be used to switch to the thread
    /// for the first time with the `iretq` instruction and allow specify the initial state
    /// of the thread:
    /// - The instruction pointer will be set to the given entry point.
    /// - The stack pointer will be set to the given stack.
    /// - The code segment will be set to the given code segment. Depending on the code segment,
    ///  the thread will be in kernel mode or in user mode.
    /// - The stack segment will be set to the given stack segment.
    /// - The rflags will be set to 0x200, enabling interrupts when jumping to the thread
    fn write_initial_trampoline(&mut self, entry: u64, stack: u64, cs: u64, ss: u64) {
        let base = self.base().as_mut_ptr::<u64>();
        let rflags = 0x200;

        // Write in the stack the trampoline that will be used to switch to the thread for the
        // first time. It is simply the registers that will be restored by the iretq instruction.
        // We can write after the stack base because we saved a bit of space in the stack in the
        // `base` method.
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
        Virtual::from(self.frames.end.addr()) - 64u64
    }

    /// Return a mutable pointer to the saved state of this thread.
    pub fn state_ptr_mut(&mut self) -> *mut *mut State {
        core::ptr::addr_of_mut!(self.state)
    }

    /// Return the address where the state of the thread is saved. If there is no saved state,
    /// this will return 0.
    pub fn state_addr(&self) -> u64 {
        self.state as u64
    }

    /// Verify if the kernel stack contains the state of the thread.
    pub fn has_saved_state(&self) -> bool {
        !self.state.is_null()
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
        let mut kstack = KernelStack::allocate(Self::KSTACK_FRAMES);

        // Create the stack frame for the first time the thread will be executed, so
        // the `enter_user` function will be abled to return to the thread with the
        // `iretq` instruction. We can safely use the base address of the stack, because
        // the stack base is 64 bytes before the real end of the stack, allowing us to
        // use this space to create the stack frame.
        kstack.write_initial_user_trampoline(entry, rsp);
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

    pub fn kernel(entry: KernelThreadFn) -> Self {
        // Allocate a kernel stack and write the initial state of the thread into it.
        let mut kstack = KernelStack::allocate(Self::KSTACK_FRAMES);
        kstack.write_initial_kernel_trampoline(entry as usize as u64);
        kstack.write_initial_state(State::default());

        // The gsbase are meaningless in a kernel thread because it will be never used
        // since the swapgs instruction is only executed when switching to an different
        // privilege level (user to kernel or kernel to user). The gsbase will be the
        // same as the one used by the kernel on the current CPU.
        Self {
            mm: Arc::new(PageTableRoot::new()),
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

    next.mm().set_current();
    set_kernel_stack(&next.kstack);
    switch_context(prev.kstack.state_ptr_mut(), next.kstack.state_ptr_mut());
}

/// Jump to the given thread. This function should be used when there is no need
/// to save any thread state, for example when the kernel is starting the first
/// thread after the boot process or when the kernel switch from a thread that
/// has exited.
/// Because this function doesn't save any thread state, it will never return to
/// the caller. The thread will only be controlled by the interruption handlers.
///
/// # Safety
/// This function is unsafe for approximately the same reasons as the `switch`
/// function.
pub unsafe fn jump_to(thread: &mut Thread) -> ! {
    msr::write(msr::Register::KERNEL_GS_BASE, thread.gsbase);
    msr::write(msr::Register::FS_BASE, thread.fsbase);

    thread.mm.set_current();
    set_kernel_stack(&thread.kstack);
    restore_context(thread.kstack.state_ptr_mut());
}

/// Exit the current thread and switch to the next thread. This function simply call
/// the `exit_thread` function written in assembly with the right arguments. Because
/// this function doesn't save any thread state, it will never return tothe caller after
/// switching to the next thread.
///
/// # Safety
/// This function is unsafe because it plays with raw pointers and CPU registers in order
/// to be able to free the memory used by the current task and to switch to the next task.
/// No need to say that this is highly unsafe, any bug or undefined behavior here can lead
/// to a kernel panic or a security issue.
pub unsafe fn exit(current: Arc<Task>, thread: &mut Thread) -> ! {
    let stack = thread.kstack.state_addr().sub(16).align_down(16);
    let prev = Arc::into_raw(current);
    exit_thread(prev, thread, stack)
}

/// Terminates the current thread by changing the current page table to the next thread
/// page table, drop the Arc to the current thread and jump to the next thread.
/// The code may appear more complicated than it should be, but it is needed because we must
/// change the kernel stack and the page table before dropping the Arc to the current thread
/// because it belong to the current thread that could be dropped here and thus creating a
/// use after free.
///
/// # Safety
/// This function is unsafe because it plays with raw pointers and CPU registers in order
/// to be able to free the memory used by the current task and to switch to the next task.
/// No need to say that this is highly unsafe, any bug or undefined behavior here can lead
/// to a kernel panic or a security issue.
#[no_mangle]
unsafe extern "C" fn terminate_thread(current: *const Task, thread: &mut Thread) {
    thread.mm().set_current();

    // Drop the Arc to the current thread before leaving this thread forever. This
    // must be done here because this function will never return to the caller and
    // therefore the Arc will never be dropped if we don't do it here.
    core::mem::drop(Arc::from_raw(current));
    jump_to(thread);
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
