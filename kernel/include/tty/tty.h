#ifndef KERNEL_TTY_H
#define KERNEL_TTY_H

void tty_init(void);
void tty_writestring(const char *str);

#endif