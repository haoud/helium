# Exit the application. This function does not return in any way.
#
# Parameters:
#  - RDI - exit code
.globl exit
exit:
    xor rax, rax
    syscall
    