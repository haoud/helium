.intel_syntax noprefix
.section .text
.global _start

_start:
    # task_exit
    mov rax, 0
    syscall
