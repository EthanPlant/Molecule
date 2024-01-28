#ifndef LIBK_STRING_H
#define LIBK_STRING_H

#include <stddef.h>

int memcmp(const void*, const void*, size_t);
void* memcpy(void*  __restrict, const void* __restrict, size_t);
void* memmove(void*, const void*, size_t);
void* memset(void*, int, size_t);
size_t strlen(const char*);

char *itoa(int num, char *str, int base);

#endif