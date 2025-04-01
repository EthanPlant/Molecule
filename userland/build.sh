#!/bin/bash
nasm test.asm -f elf64 -o test.o
ld test.o -o test
cp test ../sysroot/test