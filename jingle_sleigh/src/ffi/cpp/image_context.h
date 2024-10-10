//
// Created by toolCHAINZ on 10/10/24.
//

#ifndef JINGLE_SLEIGH_IMAGE_CONTEXT_H
#define JINGLE_SLEIGH_IMAGE_CONTEXT_H

#include "sleigh/sleigh.hh"
#include "rust/cxx.h"
#include "jingle_sleigh/src/ffi/image.rs.h"
#include "context.h"

class SleighImage{
    ghidra::Sleigh sl;
    ghidra::ContextInternal c_db;
    DummyLoadImage image;

public:
    SleighImage(ghidra::Sleigh, Image);

    InstructionFFI get_one_instruction(uint64_t offset) const;


    [[nodiscard]] std::shared_ptr<AddrSpaceHandle> getSpaceByIndex(ghidra::int4 idx) const;

    int getNumSpaces() const;

    VarnodeInfoFFI getRegister(rust::Str name) const;

    rust::Str getRegisterName(VarnodeInfoFFI name) const;

    rust::Vec<RegisterInfoFFI> getRegisters() const;
};

#endif //JINGLE_SLEIGH_IMAGE_CONTEXT_H
