use super::{
    idle,
    scheduler::{Scheduler, SCHEDULER},
};
use crate::{
    user::vmm,
    vfs::{self, fd::OpenedFiles},
};
use crate::{
    vfs::dentry::Dentry,
    x86_64::thread::{KernelThreadFn, Thread},
};
use core::sync::atomic::{AtomicU64, Ordering};

pub mod elf;
pub mod mutex;
pub mod preempt;
pub mod queue;

/// By default, all task stacks as the same base address. This is because we don't have a
/// user memory manager yet, so we can't dynamically allocate stacks. This means that we
/// cannot have multiple tasks running in the same address space (multi-threading) but
/// this is not a problem for now.
pub const STACK_BASE: usize = 0x0000_7FFF_FFFF_0000;
pub const STACK_SIZE: usize = 64 * 1024;

/// A counter used to generate unique identifiers for tasks
static COUNTER: AtomicU64 = AtomicU64::new(1);

/// Contains a list of all tasks in the system
static TASK_LIST: Spinlock<Vec<Arc<Task>>> = Spinlock::new(Vec::new());

/// A unique identifier for a task. This is used to identify tasks. The algorithm used
/// to generate the identifier is very simple: it is a counter that is incremented every
/// time a new task is created. This means that the identifier is unique for each task
/// and that it is monotonically increasing.
///
/// There is no risk of overflow because the counter is 64 bits wide, which means that
/// we can create 2^64 tasks before overflowing (and it won't happen anytime soon).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identifier(pub u64);
impl Identifier {
    /// Create a new identifier with the given value.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Generate a unique identifier. The generated identifier is guaranteed to be unique
    /// across the lifetime of the kernel (identifier are never reused)
    #[must_use]
    pub fn generate() -> Self {
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Return the last identifier generated. This is used to know if an given identifier
    /// exists (or has existed, since identifiers are never reused)
    #[must_use]
    pub fn last(&self) -> Self {
        Self(COUNTER.load(Ordering::Relaxed))
    }
}

impl core::fmt::Display for Identifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Identifier {
    fn from(id: usize) -> Self {
        Self::new(id as u64)
    }
}

impl From<u64> for Identifier {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

/// Represents the state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// The task has been created but has not been scheduled yet
    Created,

    /// The task is currently running on a CPU
    Running,

    /// The task is ready to run but is not currently running
    Ready,

    /// The task is currently being rescheduled by the scheduler and is next state will
    /// either be `Running` or `Ready` depending on the scheduler decision
    Rescheduled,

    /// The task is blocked and cannot run
    Blocked,

    /// The task execution has been terminated by itself or by a signal but the task still
    /// exist in memory. It will be deleted when the last reference to it will be dropped.
    Terminated,
}

impl State {
    /// Verify if the task is in an executable state. This is used to know if the task
    /// can be picked by the scheduler to be executed or not. If a task is already running
    /// or being rescheduled, it is npt considered as executable because it is already
    /// running
    #[must_use]
    pub fn executable(&self) -> bool {
        matches!(self, State::Created | State::Ready)
    }
}

/// The priority of a task. This is used by the scheduler to know which task to pick
/// when multiple tasks are executable. If an task has a higher priority, it will be
/// picked before a task with a lower priority. Tasks with the same priority are picked
/// in a round-robin fashion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Idle,
    Low,
    Normal,
    High,
}

impl Priority {
    #[must_use]
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

pub struct Task {
    /// The identifier of the task. This is used to identify the task and is unique across
    /// the lifetime of the kernel.
    id: Identifier,

    /// The current state of the task
    state: Spinlock<State>,

    /// The thread of the task. For now, a task can only have one thread, but this will
    /// change in the future when we will implement multi-threading.
    thread: Spinlock<Thread>,

    /// The priority of the task. This is used by the scheduler to know which task to pick
    /// when multiple tasks are executable. If an task has a higher priority, it will be
    /// picked before a task with a lower priority. Tasks with the same priority are picked
    /// in a round-robin fashion.
    priority: Spinlock<Priority>,

    /// The list of opened files of the task. This is used by the VFS subsystem to know
    /// which files are opened by the task.
    files: Spinlock<OpenedFiles>,

    /// The root directory of the task. This is used by the VFS subsystem to know the
    /// root directory used by the task.
    root: Spinlock<Arc<Dentry>>,

    /// The current working directory of the task. This is used by the VFS subsystem to
    /// know the current working directory of the task.
    cwd: Spinlock<Arc<Dentry>>,
}

impl Task {
    /// Create a new kernel task in the `Created` state with the given entry point and
    /// priority, add it to the task list and return it.
    ///
    /// # Panics
    /// This function will panic the VFS subsystem is not initialized.
    #[must_use]
    pub fn kernel(entry: KernelThreadFn, priority: Priority) -> Arc<Task> {
        let thread = Thread::kernel(entry);
        let task = Arc::new(Self {
            id: Identifier::generate(),
            state: Spinlock::new(State::Created),
            thread: Spinlock::new(thread),
            priority: Spinlock::new(priority),
            files: Spinlock::new(OpenedFiles::empty()),
            root: Spinlock::new(vfs::dentry::ROOT.get().unwrap().clone()),
            cwd: Spinlock::new(vfs::dentry::ROOT.get().unwrap().clone()),
        });
        TASK_LIST.lock().push(Arc::clone(&task));
        task
    }

    /// Create a new task in the `Created` state with the given memory map and entry
    /// point, add it to the task list and return it. It return an `Arc` to the task
    /// so that it can be shared between multiple kernel subsystems.
    ///
    /// # Panics
    /// This function will panic the VFS subsystem is not initialized.
    #[must_use]
    pub fn user(mm: Arc<Spinlock<vmm::Manager>>, entry: usize) -> Arc<Task> {
        let thread = Thread::new(mm, entry, STACK_BASE, STACK_SIZE);
        let task = Arc::new(Self {
            id: Identifier::generate(),
            state: Spinlock::new(State::Created),
            thread: Spinlock::new(thread),
            priority: Spinlock::new(Priority::Normal),
            files: Spinlock::new(OpenedFiles::empty()),
            root: Spinlock::new(vfs::dentry::ROOT.get().unwrap().clone()),
            cwd: Spinlock::new(vfs::dentry::ROOT.get().unwrap().clone()),
        });
        TASK_LIST.lock().push(Arc::clone(&task));
        task
    }

    /// Create an idle task. This is a special task that is executed when no other task
    /// is executable. Unlike other task creation functions, this function automatically
    /// add the task to the scheduler.
    ///
    /// # Safety
    /// This function is technically safe to use, but should only be used by the scheduler
    /// subsystem.
    #[must_use]
    pub fn idle() -> Arc<Task> {
        let task = Task::kernel(idle, Priority::Idle);
        SCHEDULER.add_task(Arc::clone(&task));
        task
    }

    /// Change the priority of the task.
    pub fn change_priority(&self, priority: Priority) {
        *self.priority.lock() = priority;
    }

    /// Change the state of the task.
    pub fn change_state(&self, state: State) {
        *self.state.lock() = state;
    }

    /// Set the current working directory of the task.
    pub fn set_cwd(&self, cwd: Arc<Dentry>) {
        *self.cwd.lock() = cwd;
    }

    /// Return a reference to the thread of the task. The thread is wrapped in a spinlock
    /// and must be locked before use.
    #[must_use]
    pub fn thread(&self) -> &Spinlock<Thread> {
        &self.thread
    }

    /// Return the priority of the task.
    #[must_use]
    pub fn priority(&self) -> Priority {
        *self.priority.lock()
    }

    /// Return the current state of the task.
    #[must_use]
    pub fn state(&self) -> State {
        *self.state.lock()
    }

    /// Return the list of opened files of the task.
    #[must_use]
    pub fn files(&self) -> &Spinlock<OpenedFiles> {
        &self.files
    }

    /// Get the root directory of the task.
    #[must_use]
    pub fn root(&self) -> Arc<Dentry> {
        self.root.lock().clone()
    }

    /// Get the current working directory of the task.
    #[must_use]
    pub fn cwd(&self) -> Arc<Dentry> {
        self.cwd.lock().clone()
    }

    /// Return the identifier of the task. The identifier of an task is unique and will
    /// never change during the lifetime of the task.
    #[must_use]
    pub fn id(&self) -> Identifier {
        self.id
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        log::debug!("Task {} dropped", self.id);
    }
}

/// Remove a task by its identifier. This function just removes the task from the
/// task list. In most cases, this function will effectively destroy the task, but there are
/// more references to the task, it will not be destroyed until all references are dropped.
pub fn remove(tid: Identifier) {
    TASK_LIST.lock().retain(|t| t.id() != tid);
}

/// Try to get a task by its identifier. If the task is not found, `None` is returned,
/// orthwise the Arc to the task is cloned and returned.
pub fn get(tid: Identifier) -> Option<Arc<Task>> {
    TASK_LIST.lock().iter().find(|t| t.id() == tid).cloned()
}

/// Sleep the current task. This function will change the state of the current task to
/// `Blocked` and reschedule the next task to run. The current task will not be picked
/// by the scheduler until its state is changed back to `Ready` by another kernel
/// subsystem.
pub fn sleep() {
    SCHEDULER.current_task().change_state(State::Blocked);
    unsafe {
        SCHEDULER.schedule();
    }
}
