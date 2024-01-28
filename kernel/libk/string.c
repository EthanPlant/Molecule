#include <stdbool.h>

#include <libk/string.h>

static void reverse(char *str, size_t len)
{
    int start = 0;
    int end = len - 1;
    while (start < end)
    {
        char tmp = str[start];
        str[start] = str[end];
        str[end] = tmp;
        ++start;
        --end;
    }
}

int memcmp(const void *aptr, const void *bptr, size_t size)
{
    const unsigned char *a = (const unsigned char*) aptr;
    const unsigned char *b = (const unsigned char*) bptr;
    for (size_t i = 0; i < size; ++i)
    {
        if (a[i] < b[i])
        {
            return -1;
        }
        else if (b[i] < a[i])
        {
            return 1;
        }
    }

    return 0;
}

void* memcpy(void* restrict dstptr, const void* restrict srcptr, size_t size)
{
    unsigned char *dst = (unsigned char*) dstptr;
    const unsigned char *src = (const unsigned char*) srcptr;
    for (size_t i = 0; i < size; ++i)
    {
        dst[i] = src[i];
    }

    return dstptr;
}

void* memmove(void *dstptr, const void *srcptr, size_t size)
{
    unsigned char *dst = (unsigned char*) dstptr;
    const unsigned char *src = (const unsigned char*) srcptr;
    if (dst < src)
    {
        for (size_t i = 0; i < size; ++i)
        {
            dst[i] = src[i];
        }
    }
    else
    {
        for (size_t i = size; i != 0; --i)
        {
            dst[i - 1] = src[i - 1];
        }
    }

    return dstptr;
}

void* memset(void *bufptr, int value, size_t size)
{
    unsigned char *buf = (unsigned char*) bufptr;
    for(size_t i = 0; i < size; ++i)
    {
        buf[i] = (unsigned char) value;
    }
    return bufptr;
}

size_t strlen(const char *str)
{
    size_t len = 0;
    while (str[len])
    {
        len++;
    }
    return len;
}

char *itoa(int num, char *str, int base)
{
    int i = 0;
    bool is_negative = false;

    if (num == 0)
    {
        str[i++] = '0';
        str[i] = '\0';
        return str;
    }

    if (num < 10 && base == 10)
    {
        is_negative = true;
        num = -num;
    }

    while (num != 0)
    {
        int rem = num % base;
        str[i++] = (rem > 9) ? (rem - 10) + 'a' : rem + '0';
        num /= base;
    }

    if (is_negative)
    {
        str[i++] = '-';
    }
    str[i] = '\0';
    reverse(str, i);

    return str;
}