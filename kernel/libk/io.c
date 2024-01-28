#include <stdarg.h>
#include <stddef.h>

#include <libk/io.h>
#include <libk/string.h>
#include <tty/tty.h>

static void kprint(const char *str, size_t len)
{
    tty_write(str, len);
}

void kprintf(const char *format, ...)
{
    va_list parameters;
    va_start(parameters, format);

    int written = 0;
    while (*format != '\0')
    {
        if (format[0] != '%' || format[1] == '%')
        {
            if (format[0] == '%')
            {
                ++format;
            }
            size_t amount = 1;
            while (format[amount] && format[amount] != '%')
            {
                ++amount;
            }
            kprint(format, amount * 1);
            format += amount;
            written += amount;
            continue;
        }

        const char *format_begun_at = format++;
        if (*format == 'c')
        {
            ++format;
            char c = (char) va_arg(parameters, int);
            kprint(&c, 1);
            ++written;
        }
        else if (*format == 's')
        {
            ++format;
            const char *str = va_arg(parameters, const char*);
            size_t len = strlen(str);
            kprint(str, len);
            written += len;
        }
        else if (*format == 'd')
        {
            ++format;
            int i = va_arg(parameters, int);
            char buf[50];
            itoa(i, buf, 10);
            size_t len = strlen(buf);
            kprint(buf, len);
            written += len;
        }
        else if (*format == 'x')
        {
            ++format;
            int i = va_arg(parameters, int);
            char buf[50];
            buf[0] = '0';
            buf[1] = 'x';
            itoa(i, buf + 2, 16);
            size_t len = strlen(buf);
            kprint(buf, len);
            written += len;
        }
        else
        {
            format = format_begun_at;
            size_t len = strlen(format);
            kprint(format, len);
            written += len;
            format += len;
        }
    }
}