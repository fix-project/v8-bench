#!/usr/bin/env bash

make -B

for memory_size in 0 4096 16384 65536 262144 1048576 4194304 16777216 67108864 268435456
do
  sudo ./main $memory_size 10
done
