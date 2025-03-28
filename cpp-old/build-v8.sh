#!/bin/bash
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
SRC_REL=`dirname $0`
SRC=`realpath ${SRC_REL}`
export PATH=${SRC}/depot_tools:$PATH

fetch v8

pushd v8
mkdir -p out
mkdir -p out/x64.release
popd

cp args.gn v8/out/x64.release
pushd v8/out/x64.release
gn gen .
ninja -C . v8_monolith -j16
popd
