#include <cpu.h>
#include <tty/tty.h>

void kernel_main(void)
{
    tty_init();
    
    arch_init();
    tty_writestring("Welcome to Molecule!");
}