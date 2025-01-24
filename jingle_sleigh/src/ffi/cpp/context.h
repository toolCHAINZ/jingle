#ifndef JINGLE_SLEIGH_CONTEXT_H
#define JINGLE_SLEIGH_CONTEXT_H

class ContextFFI;
#include "jingle_sleigh/src/ffi/context_ffi.rs.h"
#include "rust/cxx.h"
#include "sleigh/types.h"
#include "addrspace_handle.h"
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/globalcontext.hh"
#include "sleigh/sleigh.hh"
#include "sleigh/loadimage.hh"
#include "dummy_load_image.h"

class ContextFFI {
    ghidra::Sleigh sleigh;
    ghidra::ContextInternal c_db;
    DummyLoadImage image;
public:

    explicit ContextFFI(rust::Str slaPath);

    void set_initial_context(rust::Str name, uint32_t val);

    void setImage(ImageFFI const&img);

    InstructionFFI get_one_instruction(uint64_t offset) const;

    [[nodiscard]] std::shared_ptr<AddrSpaceHandle> getSpaceByIndex(ghidra::int4 idx) const;

    int getNumSpaces() const;

    VarnodeInfoFFI getRegister(rust::Str name) const;

    rust::Str getRegisterName(VarnodeInfoFFI name) const;

    rust::Vec<RegisterInfoFFI> getRegisters() const;
};

RegisterInfoFFI collectRegInfo(std::tuple<ghidra::VarnodeData*, std::string> el);

VarnodeInfoFFI varnodeToFFI(ghidra::VarnodeData vn);

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath);

#endif //JINGLE_SLEIGH_CONTEXT_H
