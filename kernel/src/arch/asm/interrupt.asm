# Called by the interrupt stubs before calling the interrupt handler. 
# This function saves the registers and aligns the stack to 16 bytes before
# returning to the caller
.globl interrupt_enter
interrupt_enter:
    # Swap the kernel and user GS if we was in user mode
    cmp QWORD ptr [rsp + 8 * 4], 0x2B
    jne 1f
    swapgs

1:
    # Save scratch registers
    push r11
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push rcx
    push rax

    # Save preserved registers
    push r15
    push r14
    push r13
    push r12
    push rbx
    push rbp
    
    # Align the stack to 16 bytes and move the pointer 
    # to the pushed registers in RDI
    mov rdi, rsp
    sub rsp, 8

    # Go to the caller return address: we can't use RET because we 
    # have pushed registers on the stack, so the return address is
    # not at the top of the stack
    mov rax, [rsp + 16 * 8]
    jmp rax


# Called by the interrupt stubs after the interrupt handler returns. 
# This function restores the registers and aligns the stack to 16 bytes, swaps kernel and 
# user GS if we was in user mode and returns to the interrupted code
.globl interrupt_exit
interrupt_exit:
    # Dealign the stack
    add rsp, 8

    # Restore preserved registers
    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15

    # Restore scratch registers
    pop rax
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop r11

    # Restore user GS if we was in user mode
    cmp QWORD ptr [rsp + 8 * 4], 0x2B
    jne 1f
    swapgs

1:
    # Skip error code and the return address and return
    add rsp, 16
    iretq
