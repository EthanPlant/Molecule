#include <stdint.h>

#include <cpu/gdt.h>
#include <cpu/idt.h>

idt_entry_t idt_entries[256];
idt_ptr_t idt_ptr;

void set_idt_descriptor(uint8_t interrupt, uint32_t base, uint16_t sel, uint8_t flags)
{
    idt_entries[interrupt].offset_lo = base & 0xFFFF;
    idt_entries[interrupt].segment = sel;
    idt_entries[interrupt].reserved = 0;
    idt_entries[interrupt].attrs = flags;
    idt_entries[interrupt].offset_hi = (base >> 16) & 0xFFFF;
}

void idt_init(void)
{
    idt_ptr.size = (sizeof(idt_entry_t) * IDT_ENTRIES) - 1;
    idt_ptr.offset = &idt_entries;

    set_idt_descriptor(0x00, *isr_0, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);
    set_idt_descriptor(0x01, *isr_1, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x02, *isr_2, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x03, *isr_3, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x04, *isr_4, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x05, *isr_5, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x06, *isr_6, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x07, *isr_7, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x08, *isr_8, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x09, *isr_9, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x0A, *isr_10, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x0B, *isr_11, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x0C, *isr_12, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x0D, *isr_13, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x0E, *isr_14, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x0F, *isr_15, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x10, *isr_16, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);
    set_idt_descriptor(0x11, *isr_17, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x12, *isr_18, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x13, *isr_29, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x14, *isr_20, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x15, *isr_21, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x16, *isr_22, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x17, *isr_23, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x18, *isr_24, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x19, *isr_25, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x1A, *isr_26, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x1B, *isr_27, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x1C, *isr_28, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x1D, *isr_29, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x1E, *isr_30, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);    
    set_idt_descriptor(0x1F, *isr_31, KERNEL_DATA_SEL, IDT_PRESENT | IDT_32_BIT_INT);

    flush_idt(&idt_ptr);
}