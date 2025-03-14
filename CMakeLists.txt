cmake_minimum_required(VERSION 3.16)
project(v8bench)
set (CMAKE_CXX_STANDARD 23)
set (CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(etc/build_type.cmake)
add_compile_options(-Wall -Wpedantic -Werror -Wextra)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti -DV8_COMPRESS_POINTERS -DV8_ENABLE_SANDBOX -DV8_ENABLE_CHECKS")

include_directories(SYSTEM "${PROJECT_SOURCE_DIR}/third-party")
include_directories(SYSTEM ${CMAKE_SOURCE_DIR}/v8/include)
include_directories(SYSTEM ${CMAKE_SOURCE_DIR}/v8)

link_directories(${CMAKE_SOURCE_DIR}/v8/out/x64.release/obj)

SET(LIBV8
    v8_monolith)

file(GLOB V8_LIB_SOURCES src/v8/*.cc)
add_library(v8lib ${V8_LIB_SOURCES})

add_subdirectory(src/util)

add_executable(bench src/main.cc)
target_link_libraries(bench util v8lib ${LIBV8})
set_target_properties(bench PROPERTIES LINK_FLAGS "-fuse-ld=lld")
