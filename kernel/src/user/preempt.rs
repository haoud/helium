use crate::x86_64;
use core::sync::atomic::{AtomicUsize, Ordering};
use macros::per_cpu;

/// The number of times preemption has been enabled on the current core. By default, preemption is
/// not enabled inside the kernel, so this variable is initialized to 0. When preemption is enabled,
/// this variable is incremented. When preemption is disabled, this variable is decremented and
/// preemption is only enabled when this variable is greater than 0.
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
