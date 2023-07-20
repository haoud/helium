use super::{fpu, gdt::Selector, msr, paging::PAGE_SIZE, percpu, tss};
use crate::{
    mm::{
        frame::{allocator::Allocator, AllocationFlags, Frame},
        vmm::{
            self,
            area::{self, Area},
        },
        FRAME_ALLOCATOR,
    },
    user::task::Task,
};
use addr::{user::UserVirtual, virt::Virtual};
use alloc::sync::Arc;
use core::{
    num::NonZeroU64,
    ops::{Range, Sub},
};
use lib::align::Align;
use sync::{Lazy, Spinlock};

core::arch::global_asm!(include_str!("asm/thread.asm"));

extern "C" {
    fn switch_context(prev: *mut *mut State, next: *mut *mut State);
    fn restore_context(next: *mut *mut State) -> !;
    fn enter_thread() -> !;

    #[allow(improper_ctypes)]
    fn exit_thread(current: *const Task, next: &mut Thread, stack: usize) -> !;
}

/// The virtual memory manager of the kernel. All kernel threads share the same
/// virtual memory manager to save some memory since they share the same address
/// space.
static KERNEL_VMM: Lazy<Arc<Spinlock<vmm::Manager>>> =
    Lazy::new(|| Arc::new(Spinlock::new(vmm::Manager::kernel())));

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
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
struct State {
    pub rflags: usize,
    pub rbp: usize,
    pub rbx: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    pub rip: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rflags: 0,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rip: enter_thread as usize,
        }
    }
}

/// The kernel stack of a thread. This structure is used to know where the kernel stack
/// of a thread is located, and to know the base address of the stack. This also contain
/// a pointer to the saved state of the thread if the thread is not running.
#[derive(Debug, PartialEq, Eq)]
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
        // FIXME: This memory is never freed
        KernelStack::new(unsafe {
            FRAME_ALLOCATOR
                .lock()
                .allocate_range(frames, AllocationFlags::KERNEL)
                .expect("Failed to allocate kernel stack")
        })
    }

    fn write_initial_kernel_trampoline(&mut self, entry: usize) {
        let cs = usize::from(Selector::KERNEL_CODE.0);
        let ss = usize::from(Selector::KERNEL_DATA.0);
        self.write_initial_trampoline(entry, usize::from(self.base()), cs, ss);
    }

    fn write_initial_user_trampoline(&mut self, entry: usize, stack: usize) {
        let cs = usize::from(Selector::USER_CODE.0);
        let ss = usize::from(Selector::USER_DATA.0);
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
    fn write_initial_trampoline(&mut self, entry: usize, stack: usize, cs: usize, ss: usize) {
        let base = self.base().as_mut_ptr::<usize>();
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
    pub fn state_addr(&self) -> usize {
        self.state as usize
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
    /// The virtual memory manager of this thread
    vmm: Arc<Spinlock<vmm::Manager>>,

    /// The kernel stack for this thread
    kstack: KernelStack,

    /// The user base address for the GS segment. If the thread is a kernel thread,
    /// this field must be `None` because the kernel GS is not thread-local but
    /// core-local.
    gsbase: Option<NonZeroU64>,

    /// The user base address for the FS segment. If the thread is a kernel thread,
    /// this field must be `None` because the kernel should not use the FS segment.
    fsbase: Option<NonZeroU64>,

    /// The FPU state of this thread. If the thread is a kernel thread, this field
    /// must be `None` because the kernel cannot use the FPU.
    fpu: Option<fpu::State>,
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
    pub fn new(vmm: Arc<Spinlock<vmm::Manager>>, entry: usize, rsp: usize, size: usize) -> Self {
        let mut kstack = KernelStack::allocate(Self::KSTACK_FRAMES);
        let fpu = Some(fpu::State::zeroed());

        // Create the stack frame for the first time the thread will be executed, so
        // the `enter_user` function will be abled to return to the thread with the
        // `iretq` instruction. We can safely use the base address of the stack, because
        // the stack base is 64 bytes before the real end of the stack, allowing us to
        // use this space to create the stack frame.
        kstack.write_initial_user_trampoline(entry, rsp);
        kstack.write_initial_state(State::default());

        // Compute the start and end address of the user stack
        let stack_start = UserVirtual::new((rsp - size).align_down(PAGE_SIZE));
        let stack_end = UserVirtual::new(rsp.align_down(PAGE_SIZE));

        // Map the user stack at the given address with the given size
        let area = Area::builder()
            .flags(area::Flags::FIXED | area::Flags::GROW_DOWN)
            .access(area::Access::READ | area::Access::WRITE)
            .range(stack_start..stack_end)
            .kind(area::Type::Anonymous)
            .build();

        vmm.lock().mmap(area).expect("Failed to map the user stack");

        Self {
            gsbase: None,
            fsbase: None,
            kstack,
            fpu,
            vmm,
        }
    }

    /// Create a new kernel thread with the given entry point.
    ///
    /// # Panics
    /// Panics if the kernel stack could not be allocated.
    pub fn kernel(entry: KernelThreadFn) -> Self {
        // Allocate a kernel stack and write the initial state of the thread into it.
        let mut kstack = KernelStack::allocate(Self::KSTACK_FRAMES);
        kstack.write_initial_kernel_trampoline(entry as usize);
        kstack.write_initial_state(State::default());

        // The gsbase are meaningless in a kernel thread because it will be never used
        // since the swapgs instruction is only executed when switching to an different
        // privilege level (user to kernel or kernel to user). The actual gsbase will be
        // the same as the one used by the kernel on the current CPU.
        Self {
            vmm: Arc::clone(&KERNEL_VMM),
            gsbase: None,
            fsbase: None,
            fpu: None,
            kstack,
        }
    }

    /// Clone and return a `Arc` to the virtual memory manager of this thread.
    #[must_use]
    pub fn vmm(&self) -> Arc<Spinlock<vmm::Manager>> {
        Arc::clone(&self.vmm)
    }

    // Save the current GS and FS segment base addresses into the thread structure.
    unsafe fn save_fsgsbase(&mut self) {
        // Here, we need to read the kernel GS base to get the user GS base because
        // the kernel use the `swapgs` instruction when entering in kernel mode, so
        // the user GS base is saved in the KERNEL_GS_BASE while the kernel GS base
        // is set in the GS_BASE register (the GS_BASE register is the active GS base)
        self.gsbase = NonZeroU64::try_from(msr::read(msr::Register::KERNEL_GS_BASE)).ok();
        self.fsbase = NonZeroU64::try_from(msr::read(msr::Register::FS_BASE)).ok();
    }

    /// Restore the saved GS and FS segment base addresses into the thread structure.
    unsafe fn restore_fsgsbase(&self) {
        if let Some(gsbase) = self.gsbase {
            msr::write(msr::Register::KERNEL_GS_BASE, u64::from(gsbase));
        }
        if let Some(fsbase) = self.fsbase {
            msr::write(msr::Register::FS_BASE, u64::from(fsbase));
        }
    }

    /// Restore the saved FPU state of the thread. If the thread does not have a FPU
    /// state (because it is a kernel thread), this function does nothing.
    unsafe fn restore_fpu_state(&mut self) {
        if let Some(state) = &self.fpu {
            fpu::restore(state);
        }
    }

    /// Save the current FPU state into the thread structure. If the thread does not
    /// have a FPU state (because it is a kernel thread), this function does nothing.
    unsafe fn save_fpu_state(&mut self) {
        if let Some(state) = &mut self.fpu {
            fpu::save(state);
        }
    }

    /// Set the thread kernel stack as the current kernel stack. This will update the
    /// TSS, so the CPU can switch to the new kernel stack when an interrupt occurs,
    /// and also set the kernel stack in the percpu structure, to switch stack when
    /// entering in the syscall handler.
    unsafe fn set_kernel_stack(&self) {
        percpu::set_kernel_stack(self.kstack.base());
        tss::set_kernel_stack(self.kstack.base());
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
    prev.save_fsgsbase();
    prev.save_fpu_state();

    next.restore_fpu_state();
    next.restore_fsgsbase();
    next.set_kernel_stack();
    next.vmm().lock().table().set_current();
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
    thread.restore_fpu_state();
    thread.restore_fsgsbase();
    thread.set_kernel_stack();
    thread.vmm.lock().table().set_current();
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
    thread.vmm().lock().table().set_current();

    // Drop the Arc to the current thread before leaving this thread forever. This
    // must be done here because this function will never return to the caller and
    // therefore the Arc will never be dropped if we don't do it here.
    core::mem::drop(Arc::from_raw(current));
    jump_to(thread);
}
