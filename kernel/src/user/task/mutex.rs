use super::queue::WaitQueue;
use core::mem::ManuallyDrop;

/// A mutex that can be used to synchronize access to a resource between
/// multiple threads. It is similar to the [`std::sync::Mutex`] but it is
/// optimized for the kernel.
pub struct Mutex<T> {
    lock: Spinlock<T>,
    queue: WaitQueue,
}

impl<T> Mutex<T> {
    /// Create a new mutex with the given data. The mutex is initially unlocked.
    #[must_use]
    pub fn new(data: T) -> Self {
        Self {
            lock: Spinlock::new(data),
            queue: WaitQueue::new(),
        }
    }

    /// Lock the mutex. If the mutex is already locked, the current thread
    /// will be blocked until the mutex is unlocked and then it will lock
    /// the mutex and return a guard.
    /// 
    /// The wait queue used use a FIFO policy to wake up threads. However,
    /// it is not guaranteed that the thread that has been waiting the longest
    /// will acquire the lock first. It is possible that a thread that has
    /// been waiting for a shorter time acquires the lock first if it is
    /// woken up externally, for example, by a signal.
    #[must_use]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_lock() {
                break guard;
            }
            self.queue.sleep();
        }
    }

    /// Try to lock the mutex. If the mutex is already locked, this method
    /// will return `None`. Otherwise, it will lock the mutex and return
    /// a guard.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.lock.try_lock().map(|guard| MutexGuard {
            guard: ManuallyDrop::new(guard),
            mutex: self,
        })
    }
}

/// The mutex guard. It is used to ensure that the mutex is unlocked and
/// a waiter is woken up when the guard is dropped.
pub struct MutexGuard<'a, T> {
    guard: ManuallyDrop<sync::MutexGuard<'a, T>>,
    mutex: &'a Mutex<T>,
}

impl<T> Drop for MutexGuard<'_, T> {
    /// Drop the guard first to ensure that the mutex is not locked when
    /// we wake up a waiter. Then wake up a waiter if there is one.
    fn drop(&mut self) {
        // SAFETY: This is safe because as required by the `ManuallyDrop::drop()`,
        // we ensure that the guard is not used after it is dropped, nor used
        // after the data it points to is dropped.
        unsafe {
            ManuallyDrop::drop(&mut self.guard);
        }
        self.mutex.queue.wake_up_someone();
    }
}
