//
// Created by toolCHAINZ on 10/10/24.
//

#ifndef JINGLE_SLEIGH_SLEIGH_IMAGE_H
#define JINGLE_SLEIGH_SLEIGH_IMAGE_H

#include "addrspace_handle.h"
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/sleigh.hh"
#include "rust/cxx.h"
#include "jingle_sleigh/src/ffi/image.rs.h"
#include "dummy_load_image.h"

class SleighImage{
    ghidra::Sleigh sl;
    ghidra::ContextInternal c_db;
    DummyLoadImage image;

public:
    SleighImage(Image img, ghidra::Sleigh sl);

    InstructionFFI get_one_instruction(uint64_t offset) const;

    [[nodiscard]] std::shared_ptr<AddrSpaceHandle> getSpaceByIndex(ghidra::int4 idx) const;

    int getNumSpaces() const;

    VarnodeInfoFFI getRegister(rust::Str name) const;

    rust::Str getRegisterName(VarnodeInfoFFI name) const;

    rust::Vec<RegisterInfoFFI> getRegisters() const;
};

#endif //JINGLE_SLEIGH_SLEIGH_IMAGE_H
