#!/bin/bash

mkdir -p data-process
mkdir -p data-process/add-mem
rm data-process/add-mem/*

PARALLEL=${1:-$(nproc)}
LOGTWO=`echo "l($PARALLEL) / l(2)" | bc -l | xargs -I{} echo "scale=0;{} / 1" | bc`

Benchs='v8 wasm2c-mmap'
for bench in ${Benchs}; do
  echo "parallel,iterations,duration_ns,debug" > data-process/add-mem/$bench.csv
  for ((i=0; i<=$LOGTWO; i++))
  do
    P=$((2**$i))
    for ((j=0; j<$P; j++))
    do
      ./target/release/benchmark -d 10s -w 1s -p $P -r run -o data-process/add-mem/$bench.csv $bench add-mem&
    done
    wait
    sleep 1
  done
done
