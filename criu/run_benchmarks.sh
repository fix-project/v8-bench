#!/usr/bin/env bash

make -B >/dev/null

KIB=1024
MIB=$((1024 * KIB))

echo memory,iterations,seconds
for memory_size in 0 2 4 8 16 32 64 128 256 512 1024 2048
do
  sudo rm -rf /tmp/criu-test-*
  sudo ./main $(($memory_size * $MIB)) 10
done
