global flush_gdt:function
flush_gdt:
	mov eax, [esp + 4]           ; Get the GDT pointer from the stack
	lgdt [eax]                   ; Load the GDT
	
	; Update the data segments
	mov ax, 0x10                 ; 0x10 is the offset in the GDT to our data segment
	mov ds, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	mov ss, ax
	
	; Far jump to update cs register
	; 0x08 is the offset in the GDT to our code segment
    jmp 0x08:flush
flush:
    mov eax, 0xDEADBEEF
    ret