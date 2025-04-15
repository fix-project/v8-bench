#!/bin/bash

mkdir -p data

PARALLEL=${1:-$(nproc)}
LOGTWO=`echo "l($PARALLEL) / l(2)" | bc -l | xargs -I{} echo "scale=0;{} / 1" | bc`

Benchs='v8 wasm2c-mmap clone-thread clone-process clone'
Programs='add-mem matmul64 jpeg'
for program in ${Programs}; do
  mkdir -p data/${program}
  for bench in ${Benchs}; do
    echo "rm data/${program}/${bench}-process.csv"
    echo "parallel,iterations,duration_ns,debug" > data/${program}/${bench}-process.csv
    for ((i=0; i<=$LOGTWO; i++))
    do
      P=$((2**$i))
      for ((j=0; j<$P; j++))
      do
        ./target/release/benchmark -d 10s -w 1s -p $P -r run -o data/${program}/${bench}-process.csv $bench $program&
      done
      wait
      sleep 1
    done
  done
done
