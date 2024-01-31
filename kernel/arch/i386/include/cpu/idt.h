#ifndef ARCH_I386_IDT_H
#define ARCH_I386_IDT_H

#define IDT_ENTRIES 256

#define IDT_TASK_GATE 0x05
#define IDT_16_BIT_INT 0x06
#define IDT_16_BIT_TRAP 0x07
#define IDT_32_BIT_INT 0x0E
#define IDT_32_BIT_TRAP 0x0F
#define IDT_PRESENT 1 << 7

void idt_init(void);

typedef struct idt_entry_t
{
    uint16_t offset_lo;
    uint16_t segment;
    uint8_t reserved;
    uint8_t attrs;
    uint16_t offset_hi;
} idt_entry_t;

typedef struct idt_ptr_t
{
    uint16_t size;
    uint32_t offset;
} __attribute__((packed)) idt_ptr_t;

// Defined in interrupt.asm
extern void flush_idt(idt_ptr_t*);
extern void isr_0(void);
extern void isr_1(void);
extern void isr_2(void);
extern void isr_3(void);
extern void isr_4(void);
extern void isr_5(void);
extern void isr_6(void);
extern void isr_7(void);
extern void isr_8(void);
extern void isr_9(void);
extern void isr_10(void);
extern void isr_11(void);
extern void isr_12(void);
extern void isr_13(void);
extern void isr_14(void);
extern void isr_15(void);
extern void isr_16(void);
extern void isr_17(void);
extern void isr_18(void);
extern void isr_19(void);
extern void isr_20(void);
extern void isr_21(void);
extern void isr_22(void);
extern void isr_23(void);
extern void isr_24(void);
extern void isr_25(void);
extern void isr_26(void);
extern void isr_27(void);
extern void isr_28(void);
extern void isr_29(void);
extern void isr_30(void);
extern void isr_31(void);


#endif