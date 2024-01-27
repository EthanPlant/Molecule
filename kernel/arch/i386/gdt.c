#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "gdt.h"

#define GDT_ENTRIES 5

// Defined in flush-gdt.asm
extern void flush_gdt(gdt_ptr_t*);

gdt_entry_t gdt_entries[5];
gdt_ptr_t gdt_ptr;

static void gdt_set_gate(size_t num, uint32_t base, uint32_t limit, uint8_t access, uint8_t flags)
{
    gdt_entries[num].limit = limit & 0xFFFF;
    gdt_entries[num].base_low = (base & 0xFFFF);
    gdt_entries[num].base_mid = (base >> 16) & 0xFF;
    gdt_entries[num].access = access;
    gdt_entries[num].flags = (flags & 0xF0) | ((limit >> 16) & 0xF);
    gdt_entries[num].base_high = (base >> 24) & 0xFF;
}

void gdt_init(void)
{
    gdt_ptr.size = (sizeof(gdt_entry_t) * GDT_ENTRIES) - 1;
    gdt_ptr.offset = gdt_entries;

    gdt_set_gate(0, 0, 0, 0, 0); // Null segment
    gdt_set_gate(1, 0, 0xFFFFFFFF, 
        GDT_ACCESS_PRESENT | GDT_ACCESS_TYPE | GDT_ACCESS_EXECUTABLE | GDT_ACCESS_RW, 
        GDT_FLAGS_GRANULARITY | GDT_FLAGS_SIZE); // Kernel code segment
    gdt_set_gate(2, 0, 0xFFFFFFF, 
        GDT_ACCESS_PRESENT | GDT_ACCESS_TYPE | GDT_ACCESS_RW,
        GDT_FLAGS_GRANULARITY| GDT_FLAGS_SIZE); // Kernel data segment
    gdt_set_gate(3, 0, 0xFFFFFFFF, 
        GDT_ACCESS_PRESENT | GDT_ACCESS_DPL_USER | GDT_ACCESS_TYPE | GDT_ACCESS_EXECUTABLE | GDT_ACCESS_RW, 
        GDT_FLAGS_GRANULARITY | GDT_FLAGS_SIZE); // User code segment
    gdt_set_gate(4, 0, 0xFFFFFFF, 
        GDT_ACCESS_PRESENT | GDT_ACCESS_DPL_USER | GDT_ACCESS_TYPE | GDT_ACCESS_RW,
        GDT_FLAGS_GRANULARITY| GDT_FLAGS_SIZE); // Kernel data segment

    flush_gdt(&gdt_ptr);
}