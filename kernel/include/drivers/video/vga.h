#ifndef VGA_DRIVER_H
#define VGA_DRIVER_H

#include <stdint.h>

#include <tty/tty.h>

#define VGA_WIDTH 80
#define VGA_HEIGHT 25
#define VGA_BUFFER (uint16_t*) 0xB8000

static inline uint8_t vga_entry_color(color_t fg, color_t bg)
{
    return fg | bg << 4;
}

static inline uint16_t vga_entry(unsigned char uc, uint8_t color)
{
    return (uint16_t) uc | (uint16_t) color << 8;
}

#endif