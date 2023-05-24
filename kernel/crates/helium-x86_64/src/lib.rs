//! This crate contains all the x86_64 specific code. It is responsible for the initialization of
//! the architecture specific stuff (even if the kernel will only be written for x86_64, it is
//! always better to have a clean separation between the architecture specific code and the
//! architecture agnostic one).
//!
//! The code in this crate is heavily inspired by the the [Phil Opperman's blog](https://os.phil-opp.com/)
//! about writing an OS in Rust and his [x86_64 crate](https://github.com/rust-osdev/x86_64)
#![no_std]
#![allow(dead_code)]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(core_intrinsics)]

extern crate alloc;

use limine::LimineSmpInfo;
use macros::init;

pub mod apic;
pub mod cpu;
pub mod exception;
pub mod gdt;
pub mod idt;
pub mod instruction;
pub mod io;
pub mod irq;
pub mod lapic;
pub mod msr;
pub mod paging;
pub mod percpu;
pub mod pic;
pub mod pit;
pub mod serial;
pub mod smp;
pub mod tlb;
pub mod tss;

/// The maximum number of CPUs supported by the kernel. This should be enough for a while.
const MAX_CPUS: usize = 32;

/// The clock frequency in Hz.
const CLOCK_HZ: u64 = 200;

/// Setup all the x86_64 specific stuff that does not need the memory manager to be initialized.
/// This function should be called only once and by the BSP as early as possible, because some
/// basic stuff like the GDT and the IDT are initialized here.
///
/// # Safety
/// This function is unsafe because it deals with very low level stuff that needs the use of unsafe
/// code to work properly.
#[init]
pub unsafe fn early_setup() {
    gdt::setup();
    idt::setup();
    exception::install();
    pic::remap();
    irq::install();
    pit::setup();
    tlb::install_int();
}

/// Setup all the x86_64 specific stuff that needs the memory manager to be initialized. Currently,
/// this function only setups the paging system.
#[init]
pub unsafe fn setup() {
    smp::per_cpu_setup(0);
    tss::install();
    paging::setup();
    apic::remap();
    lapic::enable();
    smp::start_cpus();
}

/// This function is called to initialize the APs. Most of the work was done by the BSP, we just
/// setup some important stuff like the GDT, the IDT and the LAPIC.
#[init]
#[inline(never)]
unsafe fn ap_setup(_: &LimineSmpInfo) {
    gdt::load();
    idt::load();
    tss::install();
    lapic::enable();
}
