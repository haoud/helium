# This is the function called when a syscall is made from user mode. It is 
# written in assembly because the syscall instruction is a bit special because
# of some optimizations that are done by the CPU. The most important one is 
# that the CPU will not save the user stack pointer nor switch to the kernel
# stack. Of course, it does not save any register either. So we have to do all
# of this manually, and this cannot be done in Rust.
#
# Parameters: 
# - RAX: syscall number
# - RSI: arg1 (optional)
# - RDX: arg2 (optional)
# - R10: arg3 (optional)
# - R8:  arg4 (optional)
# - R9:  arg5 (optional)
# 
# Return value:
# - RAX contains the return value (if the syscall return to user mode)
syscall_enter:
    swapgs              # Switch to the kernel GS
    mov gs:0x08, rsp    # Save the user stack pointer in the per-cpu area
    mov rsp, gs:0x0     # Set the kernel stack pointer

    # Save the user's stack pointer. We cannot rely where we just saved it
    # because a syscall can block and be resumed later, on a different core
    # and a different per-cpu data. So we save it in the kernel stack, which
    # will not change as long as we are in the same thread.
    push gs:0x08

    # Sysret clobbers rcx and r11 so we save them
    push rcx
    push r11

    # Save the user's registers (caller-saved)
    push r15
    push r14
    push r13
    push r12
    push rbp
    push rbx

    # Save the user syscall arguments
    push r9
    push r8
    push r10
    push rdx
    push rsi
    push rdi

    # Our syscall convention is almost the same as the system V ABI, except
    # that r10 is used instead of rcx, and rax is used instead of rdi.
    mov rdi, rax
    mov rcx, r10
    call syscall_handler
   
    # Restore the user's syscall arguments
    pop rdi
    pop rsi
    pop rdx
    pop r10
    pop r8
    pop r9

    # Restore the user's registers
    pop rbx
    pop rbp
    pop r12
    pop r13
    pop r14
    pop r15

    # Restore registers clobbered by sysret
    pop r11
    pop rcx

    # Restore the user stack pointer
    pop rsp

    # Return to user mode
    swapgs
    sysretq
