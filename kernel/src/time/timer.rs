use super::{units::Nanosecond, uptime_fast};
use core::sync::atomic::{AtomicBool, Ordering};

/// The list of active timers.
static TIMERS: Lazy<Spinlock<Vec<Timer>>> = Lazy::new(|| Spinlock::new(Vec::new()));

/// The callback type for timers.
type Callback = Box<dyn FnMut(&mut Timer) + Send>;

/// A timer that will invoke a callback when it expires. The callback is guaranteed to be
/// invoked after the expiration time, but not necessarily immediately after due to the way
/// the timer system works.
pub struct Timer {
    /// The time at which the timer will expire, expressed in nanoseconds after
    /// the system was booted.
    expiration: Nanosecond,

    /// The callback to invoke when the timer expires.
    callback: Option<Callback>,

    /// The guard that will cancel the timer when dropped.
    guard: Guard,
}

impl Timer {
    /// Creates a new timer that will expire at the given time and will invoke the given
    /// callback. The expiration time is expressed in nanoseconds after the system was
    /// booted.
    /// It returns a guard that will cancel the timer when dropped if the `ignore` method
    /// is not called on it.
    #[must_use]
    #[allow(clippy::new_ret_no_self)]
    pub fn new<T, F>(expiration: T, callback: F) -> Guard
    where
        T: Into<Nanosecond>,
        F: FnMut(&mut Timer) + Send + 'static,
    {
        let guard = Guard {
            active: Arc::new(AtomicBool::new(true)),
            ignore: false,
        };

        let timer = Timer {
            expiration: expiration.into(),
            callback: Some(Box::new(callback)),
            guard: guard.clone(),
        };

        timer.activate();
        guard
    }

    /// Executes the timer callback, but only if the timer is still active. If the timer
    /// callback returns true, the timer will be reactivated and pushed back to the active
    /// timers list.
    ///
    /// # Panics
    /// This function will panic if an active timer does not have a callback. This should
    /// never happen and indicates a bug in the timer system.
    pub fn execute(mut self) {
        let mut callback = self.callback.take().expect("Active timer without callback");
        let old_expiration = self.expiration;

        if !self.guard.ignore {
            (callback)(&mut self);

            // If the timer was modified, we need to reinsert it into the active timers list
            // with the new expiration time and eventually run it again.
            if self.expiration != old_expiration {
                self.callback = Some(callback);
                self.activate();
            }
        }
    }

    /// Adds a duration to the timer expiration.
    pub fn delay<T>(&mut self, duration: T)
    where
        T: Into<Nanosecond>,
    {
        self.expiration += duration.into();
    }

    /// Substracts a duration to the timer expiration.
    pub fn advance<T>(&mut self, duration: T)
    where
        T: Into<Nanosecond>,
    {
        self.expiration -= duration.into();
    }

    /// Modifies the timer expiration to the given nanosecond time after the system was booted.
    pub fn modify<T>(&mut self, expiration: T)
    where
        T: Into<Nanosecond>,
    {
        self.expiration = expiration.into();
    }

    /// Returns true if the timer has expired.
    #[must_use]
    pub fn expired(&self) -> bool {
        self.expiration <= uptime_fast()
    }

    /// Returns true if the timer is active. If the timer was deactivated by a guard
    /// drop, this will return false.
    #[must_use]
    pub fn active(&self) -> bool {
        self.guard.active()
    }

    /// Activates the timer. If the timer has expired, it will be executed immediately,
    /// otherwise it will be pushed to the active timers list.
    fn activate(self) {
        if self.expired() {
            self.execute();
        } else {
            TIMERS.lock().push(self);
        }
    }
}

/// A guard that will cancel the timer when dropped. It can be cloned to create multiple
/// guards that will all cancel the timer when dropped. If one guard is dropped, the
/// corresponding timer will be cancelled even if multiple guards are still active.
#[derive(Debug, Clone)]
pub struct Guard {
    /// The atomic boolean that will be set to false when the timer is cancelled. It is
    /// shared with the timer and with all the guards that have been cloned from the
    /// original guard.
    active: Arc<AtomicBool>,

    /// Set to true when the guard shoud be ignored when dropped.
    ignore: bool,
}

impl Guard {
    /// Returns true if the timer is active.
    #[must_use]
    pub fn active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Ignore the guard when dropped: the timer will not be cancelled when
    /// this guard will be dropped.
    pub fn ignore(&mut self) {
        self.ignore = true;
    }

    /// Cancels the timer.
    pub fn cancel(&self) {
        self.active.store(false, Ordering::Relaxed);
    }
}

impl Drop for Guard {
    /// When a guard is dropped, it will cancel the timer it is guarding if
    /// the ignore flag is not set in the current guard.
    fn drop(&mut self) {
        if !self.ignore {
            self.cancel();
        }
    }
}

/// Setup the timer subsystem.
#[init]
pub fn setup() {
    Lazy::force(&TIMERS);
}

/// Called every tick to update the timers. It will execute remove all inactive timers
/// and execute all expired timers.
pub fn tick() {
    // Drain all expired and inactive timers and collect expired timers.
    let expired: Vec<Timer> = TIMERS
        .lock()
        .extract_if(|timer| timer.expired() || !timer.active())
        .filter(Timer::active)
        .collect();

    // Execute all expired timers. We need to do this outside of the lock on the active
    // timers list to allow callbacks to modify the active timers list. Without this,
    // a callback could deadlock the system by trying to acquire the active timers list
    // lock
    expired.into_iter().for_each(Timer::execute);
}
