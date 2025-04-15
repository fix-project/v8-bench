#!/bin/bash

set -e

OUTPUT=$(realpath $1)
if [[ "$OUTPUT" = "" ]]
then
  echo "Error: no output directory specified." >&2
  exit 1
fi

DURATION=10s
WARMUP=1s

[ -d $OUTPUT ] && rm -r $OUTPUT
mkdir -p $OUTPUT

run_processes() {
  program=$1
  bench=$2
  parallel=$3
  data=$4
  for ((j=0; j<$parallel; j++))
  do
    sudo ./target/release/benchmark -d $DURATION -w $WARMUP -p $parallel -r run -o $data/${program}/${bench}-process.csv $bench $program > /dev/null 2>&1 &
  done
  wait
  sleep 1
}

run_threads() {
  program=$1
  bench=$2
  parallel=$3
  data=$4
  sudo ./target/release/benchmark -d $DURATION -w $WARMUP -p $parallel run -o $data/${program}/${bench}.csv $bench $program > /dev/null 2>&1
  sleep 1
}

cargo build --release

Benchs='v8 v8-isolate-per-call wasm2c-mmap wasm2c-bounds-checked clone-thread clone-process clone arca arca-serial arca-shootdown arca-lock'
ProcessBenchs='v8 v8-isolate-per-call wasm2c-mmap wasm2c-bounds-checked clone-thread clone-process clone'
Programs='jpeg matmul64 add-mem'

for i in {1..10}
do
  output=$OUTPUT/$i
  mkdir -p $output

  for program in ${Programs}; do
    mkdir -p $output/${program}
    for bench in ${Benchs}; do
      if [[ "$program" == "jpeg" ]] && [[ "$bench" == "arca-"* ]]
      then
        continue
      fi
      echo "$i: $program + $bench (threads)"

      echo "parallel,iterations,duration_ns,debug" > $output/${program}/${bench}.csv

      echo "    1 thread"
      run_threads $program $bench 1 $output
      echo "    128 threads"
      run_threads $program $bench 128 $output
    done
    for bench in ${ProcessBenchs}; do
      echo "$i: $program + $bench (processes)"

      echo "parallel,iterations,duration_ns,debug" > $output/${program}/${bench}-process.csv

      echo "    1 process"
      run_processes $program $bench 1 $output
      echo "    128 processes"
      run_processes $program $bench 128 $output
    done
  done

  mkdir -p $output/continuations
  echo "$i: continuations"

  echo "    arca"
  pushd arca > /dev/null
  cargo run --release 2>&1 | grep --line-buffered -v "kernel" | grep --line-buffered -v "vmm" > $output/continuations/arca.csv
  popd
  sleep 1

  echo "    criu"
  pushd criu > /dev/null
  ./run_benchmarks.sh > $output/continuations/criu.csv
  popd
  sleep 1

  echo "    firecracker"
  pushd firecracker > /dev/null
  cargo run --release -- $output/continuations/firecracker.csv
  popd
  sleep 1
  reset
done
