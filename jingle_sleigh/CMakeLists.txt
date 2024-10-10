cmake_minimum_required(VERSION 3.26)
project(jingle_sleigh)

set(CMAKE_CXX_STANDARD 17)

include_directories(src/ffi/cpp ../target/cxxbridge)

add_library(jingle_sleigh_cpp
        src/ffi/cpp/sleigh/address.cc
        src/ffi/cpp/sleigh/context.cc
        src/ffi/cpp/sleigh/globalcontext.cc
        src/ffi/cpp/sleigh/float.cc
        src/ffi/cpp/sleigh/marshal.cc
        src/ffi/cpp/sleigh/opcodes.cc
        src/ffi/cpp/sleigh/pcoderaw.cc
        src/ffi/cpp/sleigh/semantics.cc
        src/ffi/cpp/sleigh/sleigh.cc
        src/ffi/cpp/sleigh/sleighbase.cc
        src/ffi/cpp/sleigh/slghpatexpress.cc
        src/ffi/cpp/sleigh/slghpattern.cc
        src/ffi/cpp/sleigh/slghsymbol.cc
        src/ffi/cpp/sleigh/space.cc
        src/ffi/cpp/sleigh/translate.cc
        src/ffi/cpp/sleigh/xml.cc
        src/ffi/cpp/sleigh/filemanage.cc
        src/ffi/cpp/sleigh/pcodecompile.cc
        src/ffi/cpp/sleigh/slghscan.cc
        src/ffi/cpp/sleigh/slghparse.cc
        src/ffi/cpp/context.cpp
        src/ffi/cpp/compile.cpp
        src/ffi/cpp/addrspace_handle.cpp
        src/ffi/cpp/addrspace_manager_handle.cpp
        src/ffi/cpp/context.h
        src/ffi/cpp/image_context.cpp
        src/ffi/cpp/image_context.h
        src/ffi/cpp/exception.h)

add_executable(sleigh_compile
        src/ffi/cpp/sleigh/address.cc
        src/ffi/cpp/sleigh/context.cc
        src/ffi/cpp/sleigh/globalcontext.cc
        src/ffi/cpp/sleigh/float.cc
        src/ffi/cpp/sleigh/marshal.cc
        src/ffi/cpp/sleigh/opcodes.cc
        src/ffi/cpp/sleigh/pcoderaw.cc
        src/ffi/cpp/sleigh/semantics.cc
        src/ffi/cpp/sleigh/sleigh.cc
        src/ffi/cpp/sleigh/sleighbase.cc
        src/ffi/cpp/sleigh/slghpatexpress.cc
        src/ffi/cpp/sleigh/slghpattern.cc
        src/ffi/cpp/sleigh/slghsymbol.cc
        src/ffi/cpp/sleigh/space.cc
        src/ffi/cpp/sleigh/translate.cc
        src/ffi/cpp/sleigh/xml.cc
        src/ffi/cpp/sleigh/filemanage.cc
        src/ffi/cpp/sleigh/pcodecompile.cc
        src/ffi/cpp/sleigh/slghscan.cc
        src/ffi/cpp/sleigh/slghparse.cc
        src/ffi/cpp/sleigh/slgh_compile.cc)
