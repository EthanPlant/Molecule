	; Declare constants for multiboot header
	MBALIGN equ 1 << 0           ; Align loaded modules on page boundaries
	MEMINFO equ 1 << 1           ; Provide memory map to kernel
	MBFLAGS equ MBALIGN | MEMINFO ; Set multiboot flag field
	MAGIC equ 0x1BADB002         ; Magic number to let multiboot find the header
	CHECKSUM equ - (MAGIC + MBFLAGS) ; Checksum of above
	
	; Declare a multiboot header
	section .multiboot
	align 4
	dd MAGIC
	dd MBFLAGS
	dd CHECKSUM
	
	; Allocate space for the kernel stack
	section .bss
	align 16
stack_bottom:
	resb 16384                   ; 16 KiB
	
stack_top:
	
	; Bootloader will jump to here, entry point to kernel
	section .text
global _start:function (_start.end - _start)
_start:
	mov esp, stack_top           ; Initialize the stack, needed before jumping to C
	
	; Call the kernel entrypoint
	extern kernel_main
	call kernel_main
	
	; If we somehow return from kernel_main, just hang the system
	cli
.hang: hlt
	jmp .hang
.end:
