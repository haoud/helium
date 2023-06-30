use crate::x86_64::{
    paging::PageTableRoot,
    thread::{KernelThreadFn, Thread},
};
use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use log::debug;
use sync::Spinlock;

/// By default, all task stacks as the same base address. This is because we don't have a
/// user memory manager yet, so we can't dynamically allocate stacks. This means that we
/// cannot have multiple tasks running in the same address space (multi-threading) but
/// this is not a problem for now.
pub const STACK_BASE: u64 = 0x0000_7FFF_FFFF_0000;
pub const STACK_SIZE: u64 = 64 * 1024;

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

impl From<u64> for Identifier {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

/// Represents the state of a task. A task can be in one of the following states:
/// - `Created`: the task has been created but has not been scheduled yet
/// - `Running`: the task is currently running on a CPU
/// - `Ready`: the task is ready to run but is not currently running
/// - `Blocked`: the task is blocked and cannot run
/// - `Exiting`: the task is currently exiting
/// - `Exited`: the task has exited and is waiting to be destroyed by the `task::destroy`
/// - `Terminated`: the task was destroyed by the `task::destroy` syscall, but still exists
///  in memory. It will be deleted when the last reference to it will be dropped.
/// the `task::destroy` syscall
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    Created,
    Running,
    Ready,
    Blocked,
    Exited,
    Terminated,
}

impl State {
    /// Verify if the task is in an executable state. This is used to know if the task
    /// can be picked by the scheduler to be executed or not. If a task is already running,
    /// this function considers it as not executable (because it is already executed)
    #[must_use]
    pub fn executable(&self) -> bool {
        matches!(self, State::Created | State::Ready)
    }
}

pub struct Task {
    id: Identifier,
    state: Spinlock<State>,
    thread: Spinlock<Thread>,
}

impl Task {
    #[must_use]
    pub fn kernel(entry: KernelThreadFn) -> Arc<Task> {
        let thread = Thread::kernel(entry);
        let task = Arc::new(Self {
            id: Identifier::generate(),
            state: Spinlock::new(State::Created),
            thread: Spinlock::new(thread),
        });
        TASK_LIST.lock().push(Arc::clone(&task));
        task
    }

    /// Create a new task in the `Created` state with the given memory map and entry
    /// point, add it to the task list and return it. It return an `Arc` to the task
    /// so that it can be shared between multiple kernel subsystems.
    #[must_use]
    pub fn user(mm: Arc<PageTableRoot>, entry: u64) -> Arc<Task> {
        let thread = Thread::new(mm, entry, STACK_BASE, STACK_SIZE);
        let task = Arc::new(Self {
            id: Identifier::generate(),
            state: Spinlock::new(State::Created),
            thread: Spinlock::new(thread),
        });
        TASK_LIST.lock().push(Arc::clone(&task));
        task
    }

    #[must_use]
    pub fn idle() -> Arc<Task> {
        Self::kernel(super::idle)
    }

    /// Atomically change the state of the task.
    pub fn change_state(&self, state: State) {
        *self.state.lock() = state;
    }

    /// Return a reference to the thread of the task. The thread is wrapped in a spinlock
    /// and must be locked before use.
    #[must_use]
    pub fn thread(&self) -> &Spinlock<Thread> {
        &self.thread
    }

    /// Return the current state of the task.
    #[must_use]
    pub fn state(&self) -> State {
        *self.state.lock()
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
        debug!("Task {} dropped", self.id);
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
