#ifndef ARCH_I386_GDT_H
#define ARCH_I386_GDT_H

#include <stdint.h>

#define GDT_ACCESS_PRESENT 1 << 7
#define GDT_ACCESS_DPL_USER 3 << 5
#define GDT_ACCESS_TYPE 1 << 4
#define GDT_ACCESS_EXECUTABLE 1 << 3
#define GDT_ACCESS_DIRECTION 1 << 2;
#define GDT_ACCESS_RW 1 << 1
#define GDT_ACCESS_A 1

#define GDT_FLAGS_GRANULARITY 1 << 7
#define GDT_FLAGS_SIZE 1 << 6
#define GDT_FLAGS_LONG 1 << 5;

typedef struct gdt_entry_t
{
    uint16_t limit;
    uint16_t base_low;
    uint8_t base_mid;
    uint8_t access;
    uint8_t flags;
    uint8_t base_high;
} gdt_entry_t;

typedef struct gdt_ptr_t
{
    uint16_t size;
    gdt_entry_t* offset;
} __attribute__((packed)) gdt_ptr_t;

void gdt_init(void);

#endif