use crate::x86_64::{paging::PageTableRoot, thread::Thread};
use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use sync::Spinlock;

/// By default, all task stacks as the same base address. This is because we don't have a
/// user memory manager yet, so we can't dynamically allocate stacks. This means that we
/// cannot have multiple tasks running in the same address space (multi-threading) but
/// this is not a problem for now.
pub const STACK_BASE: u64 = 0x0000_7FFF_FFFF_0000;
pub const STACK_SIZE: u64 = 64 * 1024;

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
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn generate() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl core::fmt::Display for Identifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents the state of a task. A task can be in one of the following states:
/// - `Created`: the task has been created but has not been scheduled yet
/// - `Running`: the task is currently running on a CPU
/// - `Ready`: the task is ready to run but is not currently running
/// - `Blocked`: the task is blocked and cannot run
/// - `Terminated`: the task has terminated and is waiting to be destroyed by
/// the `task::destroy` syscall
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    Created,
    Running,
    Ready,
    Blocked,
    Terminated,
}

pub struct Task {
    id: Identifier,
    state: Spinlock<State>,
    thread: Spinlock<Thread>,
}

impl Task {
    /// Create a new task in the `Created` state with the given memory map and entry
    /// point, add it to the task list and return it. It return an `Arc` to the task
    /// so that it can be shared between multiple kernel subsystems.
    #[must_use]
    pub fn new(mm: Arc<PageTableRoot>, entry: u64) -> Arc<Task> {
        let thread = Thread::new(mm, entry, STACK_BASE, STACK_SIZE);
        let task = Arc::new(Self {
            id: Identifier::generate(),
            state: Spinlock::new(State::Created),
            thread: Spinlock::new(thread),
        });
        TASK_LIST.lock().push(Arc::clone(&task));
        task
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

/// Destroy a task by its identifier. Actually, this function just removes the task from the
/// task list. In most cases, this function will effectively destroy the task, but there are
/// more references to the task, it will not be destroyed until all references are dropped.
pub fn destroy(tid: Identifier) {
    TASK_LIST.lock().retain(|t| t.id() != tid);
}

/// Try to get a task by its identifier. If the task is not found, `None` is returned,
/// orthwise the Arc to the task is cloned and returned.
pub fn get(tid: Identifier) -> Option<Arc<Task>> {
    TASK_LIST.lock().iter().find(|t| t.id() == tid).cloned()
}
