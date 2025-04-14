#!/usr/bin/env bash

set -e

ARCH="$(uname -m)"

[ -e hello-vmlinux.bin ] || wget https://s3.amazonaws.com/spec.ccfc.min/img/hello/kernel/hello-vmlinux.bin
# [ -e busybox ] || wget https://www.busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox

# chmod +x busybox

gcc -static -nostartfiles -ffreestanding init.S -o init

rm -r rootfs
mkdir -p rootfs/sbin
cp init rootfs/sbin
dd if=/dev/zero of=rootfs.ext4 bs=1K count=256
mkfs.ext4 rootfs.ext4

[ -d mnt ] && (sudo umount mnt || true; rm -r mnt)
mkdir mnt
sudo mount rootfs.ext4 mnt
sudo cp -a rootfs/* mnt
sudo umount mnt
rmdir mnt

echo "Done!"
