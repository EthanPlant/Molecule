#!/bin/sh
set -e
. ./build.sh

mkdir -p isodir
mkdir -p isodir/boot
mkdir -p isodir/boot/grub

cp sysroot/boot/molecule.kernel isodir/boot/molecule.kernel
cat > isodir/boot/grub/grub.cfg << EOF
menuentry "molecule" {
    multiboot /boot/molecule.kernel
}
EOF
grub-mkrescue -o molecule.iso isodir