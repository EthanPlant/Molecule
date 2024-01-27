#include <stdint.h>

#include <libk/string.h>
#include <drivers/video/vga.h>
#include <tty/tty.h>

static size_t tty_row;
static size_t tty_col;
static uint8_t tty_color;
static uint16_t *tty_buffer;

static void vga_printchar(char c, size_t row, size_t col, uint8_t color)
{
    size_t index = row * VGA_WIDTH + col;
    tty_buffer[index] = vga_entry(c, color);
}

void tty_init(void)
{
    tty_row = 0;
    tty_col = 0;
    tty_color = vga_entry_color(VGA_COLOR_WHITE, VGA_COLOR_BLACK);
    tty_buffer = VGA_BUFFER;

    for (size_t i = 0; i < VGA_HEIGHT; ++i)
    {
        for (size_t j = 0; j < VGA_WIDTH; ++j)
        {
            size_t index = i * VGA_WIDTH + j;
            tty_buffer[index] = vga_entry('\0', tty_color);
        }
    }
}

void tty_writestring(const char *str)
{
    for (size_t i = 0; i < strlen(str); ++i)
    {
        vga_printchar(str[i], tty_row, tty_col, tty_color);
        tty_col++;
        if (tty_col >= VGA_WIDTH)
        {
            tty_col = 0;
            tty_row++;
            if (tty_row >= VGA_HEIGHT)
            {
                tty_row = 0;
            }
        }
    }
}