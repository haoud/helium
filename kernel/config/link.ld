ENTRY(_start)
OUTPUT_FORMAT("elf64-x86-64")

KERNEL_VIRT_BASE = 0xFFFFFFFF80000000;

SECTIONS
{
    . = KERNEL_VIRT_BASE;
    .init :
    {
        *(.init*)

        . = ALIGN(16);
        __percpu_start = .;

        // Some space for saving and settings stacks during syscall
        . += 16;
        
        *(.percpu*)
        __percpu_end = .;
    }

    . = ALIGN(4096);
    .text :
    {        
        *(.text*)
    }

    . = ALIGN(4096);
    .rodata :
    {
        *(.rodata*)
    }

    . = ALIGN(4096);
    .data :
    {
        *(.data*)
    }

    . = ALIGN(4096);
    .bss (NOLOAD) :
    {
        *(.bss*)
    }

    . = ALIGN(4096);
    __end = .;
    
    /DISCARD/ :
    {
        *(.fini_array*)
        *(.eh_frame*)
        *(.comment)
        *(note*)
    }
}
