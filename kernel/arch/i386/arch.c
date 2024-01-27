#include <kernel/arch.h>

#include "gdt.h"

void arch_init(void)
{
    gdt_init();
}