use macros::init;

/// The virtual address of the local APIC base address.
pub const LAPIC_BASE: u64 = 0xFFFF_8000_FEE0_0000;

/// Represents the local APIC registers. The values are the offsets from the base address of the
/// local APIC base address.
#[repr(u64)]
pub enum Register {
    Id = 0x0020,
    Version = 0x0030,
    TaskPriority = 0x0080,
    ArbitrationPriority = 0x0090,
    ProcessorPriority = 0x00A0,
    EndOfInterrupt = 0x00B0,
    RemoteRead = 0x00C0,
    LogicalDestination = 0x00D0,
    DestinationFormat = 0x00E0,
    SpuriousInterruptVector = 0x00F0,

    InService0 = 0x0100,
    InService1 = 0x0110,
    InService2 = 0x0120,
    InService3 = 0x0130,
    InService4 = 0x0140,
    InService5 = 0x0150,
    InService6 = 0x0160,
    InService7 = 0x0170,

    TriggerMode0 = 0x0180,
    TriggerMode1 = 0x0190,
    TriggerMode2 = 0x01A0,
    TriggerMode3 = 0x01B0,
    TriggerMode4 = 0x01C0,
    TriggerMode5 = 0x01D0,
    TriggerMode6 = 0x01E0,
    TriggerMode7 = 0x01F0,

    InterruptRequest0 = 0x0200,
    InterruptRequest1 = 0x0210,
    InterruptRequest2 = 0x0220,
    InterruptRequest3 = 0x0230,
    InterruptRequest4 = 0x0240,
    InterruptRequest5 = 0x0250,
    InterruptRequest6 = 0x0260,
    InterruptRequest7 = 0x0270,

    ErrorStatus = 0x0280,
    LvtCmci = 0x02F0,
    InterruptCommand0 = 0x0300,
    InterruptCommand1 = 0x0310,
    LvtTimer = 0x0320,
    LvtThermalSensor = 0x0330,
    LvtPerformanceCounter = 0x0340,
    LvtLint0 = 0x0350,
    LvtLint1 = 0x0360,
    LvtError = 0x0370,

    InitialCount = 0x0380,
    CurrentCount = 0x0390,

    DivideConfiguration = 0x03E0,
}

/// Represents the destination of an IPI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpiDestination {
    /// Send the IPI to the given core
    Core(u8),

    /// Send the IPI to the current core
    Current,

    /// Send the IPI to all cores
    All,

    // Send the IPI to all cores except the current one
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpiPriority {
    Normal = 0,
    Low = 1,
    Smi = 2,
    Nmi = 4,
}

/// Initialize the local APIC on the current core.
///
/// # Safety
/// This function is unsafe for various reasons, but the main one is that is must be called only
/// once per core, and make some assumptions about the state of the system, like the fact that the
/// LAPIC is mapped at the `LAPIC_BASE` address.
#[init]
pub unsafe fn enable() {
    let spurious = read(Register::SpuriousInterruptVector);
    write(Register::SpuriousInterruptVector, spurious | 1 << 8);
    send_eoi();
}

/// Send an IPI to the given destination with the given priority to trigger the
/// given interrupt vector.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the given interrupt vector is valid
/// and can be triggered by an IPI. Otherwise, the kernel may panic or crash.
pub unsafe fn send_ipi(destination: IpiDestination, priority: IpiPriority, vector: u8) {
    let cmd = match destination {
        IpiDestination::Core(core) => (
            u32::from(core) << 24,
            u32::from(vector) | (priority as u32) << 8,
        ),
        IpiDestination::All => (0, u32::from(vector) | ((priority as u32) << 8) | 2 << 18),
        IpiDestination::Other => (0, u32::from(vector) | ((priority as u32) << 8) | 3 << 18),
        IpiDestination::Current => (0, u32::from(vector) | ((priority as u32) << 8) | 1 << 18),
    };

    write(Register::InterruptCommand1, cmd.0);
    write(Register::InterruptCommand0, cmd.1);

    // Wait for the IPI to be sent
    while read(Register::InterruptCommand0) & (1 << 12) != 0 {
        core::hint::spin_loop();
    }
}

/// Send an end-of-interrupt signal to the local APIC. This function must be called after an
/// local APIC interrupt has been handled. Otherwise, no local APIC interrupts will be triggered
/// until this function is called.
pub fn send_eoi() {
    unsafe {
        write(Register::EndOfInterrupt, 0);
    }
}

/// Write the given value to the given register.
///
/// # Safety
/// This function is unsafe because writing to a register can have side effects and may break the
/// memory safety of the program, or may crash the kernel if used improperly.
pub unsafe fn write(register: Register, value: u32) {
    let addr = LAPIC_BASE + register as u64;
    let ptr = addr as *mut u32;
    ptr.write_volatile(value);
}

/// Read the value of the given register.
///
/// # Safety
/// This function is unsafe because reading to a register can have side effects and may break the
/// memory safety of the program, or may crash the kernel if used improperly.
#[must_use]
pub unsafe fn read(register: Register) -> u32 {
    let addr = LAPIC_BASE + register as u64;
    let ptr = addr as *const u32;
    ptr.read_volatile()
}
