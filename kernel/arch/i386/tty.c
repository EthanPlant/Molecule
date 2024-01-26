#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <kernel/tty.h>

#include "vga.h"

static const size_t VGA_WIDTH = 80;
static const size_t VGA_HEIGHT = 25;

static const uint16_t *VGA_BUFFER = (uint16_t*) 0xB8000;

static size_t tty_row;
static size_t tty_col;
static uint8_t tty_color;
static uint16_t *tty_buffer;

static void tty_putentryat(unsigned char c, uint8_t color, size_t x, size_t y)
{
    const size_t index = y * VGA_WIDTH + x;
    tty_buffer[index] = vga_entry(c, color);
}

void tty_init(void)
{
    tty_row = 0;
    tty_col = 0;
    tty_color = vga_entry_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
    tty_buffer = VGA_BUFFER;
    for (size_t i = 0; i < VGA_HEIGHT; ++i)
    {
        for (size_t j = 0; j < VGA_WIDTH; ++j)
        {
            const size_t index = i * VGA_WIDTH + j;
            tty_buffer[index] = vga_entry('\0', tty_color);
        }
    }
}

void tty_putchar(char c)
{
    unsigned char uc = c;

    tty_putentryat(uc, tty_color, tty_col, tty_row);
    if (++tty_col >= VGA_WIDTH)
    {
        tty_col = 0;
        if (++tty_row >= VGA_HEIGHT)
        {
            tty_row = 0;
        }
    }
}

void tty_write(const char *data, size_t size)
{
    for (size_t i = 0; i < size; ++i)
    {
        tty_putchar(data[i]);
    }
}

void tty_writestring(const char *data)
{
    tty_write(data, strlen(data));
}