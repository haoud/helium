.intel_syntax noprefix
.section .text
.global _start

_start:
    # Test the stack
    push 0

    # Unknow syscall (for now)
    mov rax, 1
    syscall

    # task_exit
    mov rax, 0
    syscall
    ud2
