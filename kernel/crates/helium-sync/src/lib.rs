//! This crate provides synchronization primitives for the kernel. Currently, it just
//! reexports some objects from the `spin` and `crossbeam` crates, but in the future
//! it will provide its own implementations of these objects, optimized for the kernel.
#![no_std]

pub mod lazy;
pub mod once;

pub type Spinlock<T> = spin::Mutex<T>;
pub type Lazy<T> = lazy::Lazy<T>;
pub type Once<T> = once::Once<T>;
