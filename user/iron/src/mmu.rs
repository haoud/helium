use bitflags::bitflags;

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Access : u64 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
    }

    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Flags : u64 {
        const FIXED = 1 << 0;
        const SHARED = 1 << 1;
        const GROW_UP = 1 << 2;
        const GROW_DOWN = 1 << 3;
        const PERMANENT = 1 << 4;
    }
}

pub fn map(base: usize, len: usize, access: Access, flags: Flags) -> Result<usize, ()> {
    unsafe {
        let ret: usize;
        core::arch::asm!(
            "syscall",
            in("rax") 6,
            in("rsi") base,
            in("rdx") len,
            in("r10") access.bits(),
            in("r8") flags.bits(),
            lateout("rax") ret,
        );

        if ret > usize::MAX - 4096 {
            Err(())
        } else {
            Ok(ret)
        }
    }
}

pub fn unmap(base: usize, len: usize) -> Result<(), ()> {
    unsafe {
        let ret: usize;
        core::arch::asm!(
            "syscall",
            in("rax") 7,
            in("rsi") base,
            in("rdx") len,
            lateout("rax") ret,
        );

        if ret > usize::MAX - 4096 {
            Err(())
        } else {
            Ok(())
        }
    }
}
