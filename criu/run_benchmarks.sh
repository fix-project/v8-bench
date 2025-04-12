#!/usr/bin/env bash

make -B

echo memory, iterations, seconds
for memory_size in 0 4096 16384 65536 262144 1048576 4194304 16777216 67108864 268435456
do
  sudo rm -rf /tmp/criu-test-*
  sudo ./main $memory_size 10
done
