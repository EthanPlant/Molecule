#include <stdint.h>

#include <libk/io.h>

typedef struct interrupt_registers_t
{
    uint32_t ds;
    uint32_t edi, esi, ebp, useless, ebx, edx, ecs, eax;
    uint32_t int_no, err_code;
    uint32_t eip, cs, eflags, esp, ss;
} interrupt_registers_t;

void isr_handler(interrupt_registers_t *regs)
{
    kprintf("Recieved interrupt %x\n", regs->int_no);
}