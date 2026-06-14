#!/bin/sh

EFI=$(realpath "$1")
WKDIR="${EFI%/*}"
OUT="$WKDIR/boot.img"

rm -f $OUT

dd if=/dev/zero of="$OUT" bs=1M count=64
mkfs.vfat "$OUT"

mmd -i "$OUT" ::/EFI
mmd -i "$OUT" ::/EFI/BOOT

mcopy -i "$OUT" "$EFI" ::/EFI/BOOT/BOOTX64.EFI

VARS="$WKDIR/OVMF_VARS.fd"
cp /usr/share/OVMF/OVMF_VARS_4M.fd "$VARS"

qemu-system-x86_64 \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive if=pflash,format=raw,file="$VARS" \
    -drive format=raw,file="$OUT" \
    -serial stdio \
    -m 8G
