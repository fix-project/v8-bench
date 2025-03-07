cmake_minimum_required(VERSION 3.16)
project(v8bench)
set (CMAKE_CXX_STANDARD 23)
set (CMAKE_EXPORT_COMPILE_COMMANDS ON)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti -DV8_COMPRESS_POINTERS -DV8_ENABLE_SANDBOX -DV8_ENABLE_CHECKS")

include_directories(${CMAKE_SOURCE_DIR}/v8/include)
include_directories(${CMAKE_SOURCE_DIR}/v8)

link_directories(${CMAKE_SOURCE_DIR}/v8/out/x64.release/obj)

SET(LIBV8
    v8_monolith)

add_executable(bench src/main.cpp)
target_link_libraries(bench ${LIBV8})
set_target_properties(bench PROPERTIES LINK_FLAGS "-fuse-ld=lld")
