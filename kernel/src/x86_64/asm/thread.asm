# Parameters:
# - rdi: Pointer to a pointer to the current state struct from where the current state will 
#        be saved. The pointer to the pointer will be updated to point to the new state struct,
#        which will be allocated on the stack.
#
# - rsi: Pointer to a pointer to the new state struct from where the new state will be loaded. It
#        will be set to null, because the state is no longer valid after the context switch.
# 
# This function may return to the caller if the saved state is restored. If the task is
# detroyed, this function will not return, but this should not be a problem since this is
# the desired behavior.
switch_context:
    # Save the current state on the stack
    push r15
    push r14
    push r13
    push r12
    push rbx
    push rbp
    pushfq

    # Set the pointer to the pointer to the current state struct to point to 
    # the stack and then change the stack pointer to the top of the new kernelstack
    mov [rdi], rsp
    mov rsp, [rsi]

    # Restore the next state from the stack
    popfq
    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15
    ret

# Enter to userland. This function is called from the kernel when it wants to enter to userland
# for a task for the first time. It will set the stack pointer to the top of the new stack, clear
# all registers to avoid leaking sensitive data and then swap to the user GS segment and go to
# userland with the iretq instruction.
#
# Parameters:
# - rdi: Pointer to the top of the new stack
# 
# This function never returns to the caller.
enter_userland:
    # Set the stack pointer to the new stack 
    mov rsp, rdi

    # Clear all registers to avoid leaking sensitive data
    xor r8, r8
    xor r9, r9
    xor r10, r10
    xor r11, r11
    xor r12, r12
    xor r13, r13
    xor r14, r14
    xor r15, r15
    xor rax, rax
    xor rbx, rbx
    xor rcx, rcx
    xor rdx, rdx
    xor rsi, rsi
    xor rdi, rdi
    xor rbp, rbp

    # Swap to the user GS segment and go to userland
    swapgs
    iretq
