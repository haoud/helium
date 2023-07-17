# Switch between the current and the next state. The current state will be saved on the stack
# and then this function will restore the next state from the stack. This function may not
# return to the caller if the task is destroyed.
#
# Parameters:
# - rdi: Pointer to a pointer to the current state struct from where the current state will 
#        be saved. The pointer to the pointer will be updated to point to the new state struct,
#        which will be allocated on the stack.
#
# - rsi: Pointer to a pointer to the new state struct from where the new state will be loaded. It
#        will be set to null, because the state is no longer valid after the context switch.
#
# Return value:
#  - This function does not return any value.
switch_context:
    # Save the current state on the stack
    push r15
    push r14
    push r13
    push r12
    push rbx
    push rbp
    pushfq

    # Set the pointer to the pointer to the current state struct to point to the stack
    # and then call the restore_context function to restore the next state from the stack.
    mov [rdi], rsp
    mov rdi, rsi
    jmp restore_context

# Restore a previously saved state from the stack.
#
# Parameters:
# - rdi:  Pointer to a pointer to the state struct to be restored. The pointer to the pointer
#         will be set to null, because the state is no longer valid after the context restore.
#
# Return value:
#  - This function does not return.
restore_context:
    mov rsp, [rdi]          # Restore the stack pointer of the thread
    mov QWORD ptr [rdi], 0  # Set the pointer to the new state struct to null

    # Restore the next state from the stack
    popfq
    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15
    ret

# Called when a thread is executed for the first time. This function will clear all registers
# to avoid leaking sensitive data and then go to the thread entry point, stored on the stack.
# For more information about the stack layout, see the documentation for the thread struct and
# its `new` method.
# This function should not be called directly as it assume a specific stack layout that should
# only be created by the `new` method of the thread struct.
#
# Parameters:
# - This function does not take any parameters.
#
# Return value:
# - This function does not return.
enter_thread:
    # Unlock the previously saved thread to allow it to be resumed
    call unlock_saved_thread

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

    # Restore user GS if we was in user mode
    cmp QWORD ptr [rsp + 8], 0x08
    je 1f
    swapgs

1:
    iretq

# Called when a thread is terminated. This function simply change the kernel stack to the
# one passed as parameter and then call the `terminate_thread` function to terminate the
# thread and switch to the next thread.
# 
# Parameters:
# - rdi: The task that will be exited. This not used by this function but passed to the 
#        `terminate_thread` function.
# - rsi: The next thread to be executed after the current thread is terminated. This not
#        used by this function but passed to the `terminate_thread` function.
# - rdx: The stack that will be used before calling the `terminate_thread` function.
exit_thread:
    mov rsp, rdx
    jmp terminate_thread
