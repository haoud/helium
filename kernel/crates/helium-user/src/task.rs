use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use sync::Spinlock;
use x86_64::{paging::PageTableRoot, thread::Thread};

pub const STACK_BASE: u64 = 0x0000_7FFF_FFFF_0000;
pub const STACK_SIZE: u64 = 64 * 1024;

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
    #[must_use]
    pub fn new(mm: Arc<PageTableRoot>, entry: u64) -> Self {
        let thread = Thread::new(mm, entry, STACK_BASE, STACK_SIZE);

        Self {
            id: Identifier::generate(),
            state: Spinlock::new(State::Created),
            thread: Spinlock::new(thread),
        }
    }

    pub fn change_state(&self, state: State) {
        *self.state.lock() = state;
    }

    #[must_use]
    pub fn thread(&self) -> &Spinlock<Thread> {
        &self.thread
    }

    #[must_use]
    pub fn state(&self) -> State {
        *self.state.lock()
    }

    #[must_use]
    pub fn id(&self) -> Identifier {
        self.id
    }
}
