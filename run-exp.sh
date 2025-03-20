#!/bin/bash

parallelism=(1 2 4 8 16 32 64 128 256 512 1024)

for p in ${parallelism[@]}
do
  echo $p
  for i in {1..5}
  do
    ./build/bench -r w2c-no-mem -j $p
    ./build/bench -r w2c-sw -j $p
    ./build/bench -r w2c-hw -j $p
    ./build/bench -r v8 -j $p -b build/add.wasm
    ./build/bench -r v8-new-context -j $p -b build/add.wasm
    ./build/bench -r v8-new-isolate -j $p -b build/add.wasm
    ./build/bench -r v8 -j $p -b build/addmemory.wasm
    ./build/bench -r v8-new-context -j $p -b build/addmemory.wasm
    ./build/bench -r v8-new-isolate -j $p -b build/addmemory.wasm
  done
done
