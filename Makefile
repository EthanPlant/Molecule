ARCH?=i386
ARCHDIR:=kernel/arch/$(ARCH)

AS=nasm
CC=i686-elf-gcc
QEMU=qemu-system-$(ARCH)

CFLAGS:=-O2 -g -ffreestanding -Wall -Wextra
CPPFLAGS:=-Ikernel/include -Ikernel/arch/$(ARCH)/include
LDFLAGS:=-nostdlib -lgcc
QEMU_FLAGS:= -s

C_SOURCES:=$(wildcard kernel/kernel/*.c kernel/libk/*.c)
C_SOURCES:=$(C_SOURCES) $(wildcard kernel/drivers/video/*.c)
C_SOURCES:=$(C_SOURCES) $(wildcard $(ARCHDIR)/cpu/*c)

ASM_SOURCES:=$(wildcard $(ARCHDIR)/boot/*.asm)
ASM_SOURCES:=$(ASM_SOURCES) $(wildcard $(ARCHDIR)/cpu/*.asm)

C_OBJ:=${C_SOURCES:.c=.o}
ASM_OBJ:=${ASM_SOURCES:.asm=.o}

.PHONY: all run debug clean
.SUFFIXES: .o .c .asm

all: molecule.bin

molecule.bin: $(C_OBJ) $(ASM_OBJ) $(ARCHDIR)/linker.ld
	$(CC) -T $(ARCHDIR)/linker.ld -o $@ $(CFLAGS) $(LDFLAGS) $(C_OBJ) $(ASM_OBJ)
	grub-file --is-x86-multiboot molecule.bin

.c.o:
	$(CC) -MD -c $< -o $@ -std=gnu11 $(CFLAGS) $(CPPFLAGS)

.asm.o:
	$(AS) $< -f elf -o $@

clean:
	rm -f molecule.bin
	rm -f $(C_OBJ)
	rm -f $(C_OBJ:.o=.d)
	rm -f $(ASM_OBJ)
	rm -rf isodir

molecule.iso: molecule.bin
	mkdir -p isodir
	mkdir -p isodir/boot
	mkdir -p isodir/boot/grub
	cp molecule.bin isodir/boot/molecule.bin
	cat $(ARCHDIR)/boot/grub.cfg > isodir/boot/grub/grub.cfg
	grub-mkrescue -o molecule.iso isodir

run: molecule.iso
	$(QEMU) $(QEMU_FLAGS) -cdrom molecule.iso