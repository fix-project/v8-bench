cmake_minimum_required(VERSION 3.16)
project(v8bench)
set (CMAKE_CXX_STANDARD 23)
set (CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(etc/build_type.cmake)
add_compile_options(-Wall -Wpedantic -Wextra)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti -DV8_COMPRESS_POINTERS -DV8_ENABLE_SANDBOX -DV8_ENABLE_CHECKS")

add_custom_command(
  OUTPUT "add.wasm"
  DEPENDS ${CMAKE_SOURCE_DIR}/binary/add.wat
  COMMAND $ENV{HOME}/wasm-toolchain/wabt/build/wat2wasm
          ${CMAKE_SOURCE_DIR}/binary/add.wat
          -o add.wasm)
add_custom_target(add-wasm ALL DEPENDS add.wasm)

add_custom_command(
  OUTPUT "addmemory.wasm"
  DEPENDS ${CMAKE_SOURCE_DIR}/binary/addmemory.wat
  COMMAND $ENV{HOME}/wasm-toolchain/wabt/build/wat2wasm
          ${CMAKE_SOURCE_DIR}/binary/addmemory.wat
          -o addmemory.wasm)
add_custom_target(addmemory-wasm ALL DEPENDS addmemory.wasm)

include_directories(SYSTEM "${PROJECT_SOURCE_DIR}/third-party")
include_directories(SYSTEM ${CMAKE_SOURCE_DIR}/v8/include)
include_directories(SYSTEM ${CMAKE_SOURCE_DIR}/v8)

link_directories(${CMAKE_SOURCE_DIR}/v8/out/x64.release/obj)

SET(LIBV8
    v8_monolith)

include_directories(${CMAKE_SOURCE_DIR}/src/common)
file(GLOB V8_LIB_SOURCES src/v8/*.cc)
add_library(v8lib ${V8_LIB_SOURCES})

file(GLOB W2C_LIB_SOURCES src/wasm2c/*.c src/wasm2c/*.cc)
add_library(w2clib ${W2C_LIB_SOURCES})

add_subdirectory(src/util)

add_executable(bench src/main.cc)
target_link_libraries(bench util w2clib v8lib ${LIBV8})
set_target_properties(bench PROPERTIES LINK_FLAGS "-fuse-ld=lld")
