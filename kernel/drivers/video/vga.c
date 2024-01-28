#include <stdint.h>

#include <libk/string.h>
#include <drivers/video/vga.h>
#include <tty/tty.h>

static size_t tty_row;
static size_t tty_col;
static uint8_t tty_color;
static uint16_t *tty_buffer;

static void vga_printchar(char c, size_t row, size_t col, color_t color)
{
    size_t index = row * VGA_WIDTH + col;
    tty_buffer[index] = vga_entry(c, color);
}

void tty_init(void)
{
    tty_row = 0;
    tty_col = 0;
    tty_color = vga_entry_color(DEFAULT_COLOR, BLACK);
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

void tty_write(const char *data, size_t len)
{
    for (size_t i = 0; i < len; ++i)
    {
        // TODO: Better handling of special chars
        if (data[i] == '\n')
        {
            tty_col = 0;
            ++tty_row;
        }
        else
        {
            vga_printchar(data[i], tty_row, tty_col, tty_color);
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
}

void tty_writestring(const char *str)
{
    tty_write(str, strlen(str));
}

void tty_setcolor(color_t color)
{
    tty_color = vga_entry_color(color, BLACK);
}

void tty_colortest(void)
{
    for (int i = 0; i < 16; ++i)
    {
        tty_setcolor(i);
        tty_writestring("#");
    }
    tty_setcolor(DEFAULT_COLOR);
    tty_writestring("\n");
}