use crate::x86_64;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The number of times preemption has been enabled on the current core. By default, preemption is
/// enabled inside the kernel, so this variable is initialized to 0. When preemption is disabled,
/// this variable is incremented. When preemption is restaured, this variable is decremented. Using
/// a counter instead of a boolean allows to call `enable` and `disable` multiple times without
/// losing the state of preemption.
#[per_cpu]
pub static PREEMTABLE: AtomicUsize = AtomicUsize::new(0);

/// The value of `PREEMTABLE` when preemption is enabled
const PREEMPT_ENABLED: usize = 0;

/// Check if preemption is enabled on the current core.
#[must_use]
pub fn enabled() -> bool {
    x86_64::irq::without(|| unsafe {
        PREEMTABLE.local_unchecked().load(Ordering::SeqCst) == PREEMPT_ENABLED
    })
}

/// Enable preemption on the current core. This function can be called multiple times, but it must
/// be balanced with the same number of calls to `disable` before preemption is actually enabled.
///
/// # Panics
/// This function will panic if preemption is already enabled, because it means that `enable` has
/// been called more times than `disable` and is a kernel bug.
pub fn enable() {
    x86_64::irq::without(|| unsafe {
        assert!(PREEMTABLE.local_unchecked().fetch_sub(1, Ordering::SeqCst) > 0);
    });
}

/// Disable preemption on the current core. This function can be called multiple times, but it must
/// be balanced with the same number of calls to `enable` to actually re-enable preemption.
pub fn disable() {
    x86_64::irq::without(|| unsafe {
        PREEMTABLE.local_unchecked().fetch_add(1, Ordering::SeqCst);
    });
}

/// Disable preemption during the execution of the given closure. This function is useful to avoid
/// race conditions when multiple threads are accessing the same data. This function is implemented
/// by disabling interrupts and preemption before executing the closure, and then restoring the
/// previous state after the closure has finished.
///
/// # Important
/// Even if preemption is disabled, interrupts are still enabled. This means that the closure can
/// still be interrupted by an IRQ handler. If you want to disable interrupts too, then you should
pub fn without<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    x86_64::irq::without(|| {
        disable();
        let ret = f();
        enable();
        ret
    })
}
