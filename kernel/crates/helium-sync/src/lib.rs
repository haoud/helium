//! This crate provides synchronization primitives for the kernel. Currently, it just
//! reexports some objects from the `spin` and `crossbeam` crates, but in the future
//! it will provide its own implementations of these objects, optimized for the kernel.
#![no_std]

pub type AtomicCell<T> = crossbeam::atomic::AtomicCell<T>;

pub type Spinlock<T> = spin::Mutex<T>;
pub type Lazy<T> = spin::Lazy<T>;
pub type Once<T> = spin::Once<T>;
