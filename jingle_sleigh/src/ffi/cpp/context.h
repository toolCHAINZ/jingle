#ifndef JINGLE_SLEIGH_CONTEXT_H
#define JINGLE_SLEIGH_CONTEXT_H

#include "rust/cxx.h"
#include "sleigh/types.h"
#include "addrspace_handle.h"
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/globalcontext.hh"
#include "sleigh/sleigh.hh"
#include "jingle_sleigh/src/ffi/image.rs.h"
#include "sleigh/loadimage.hh"
#include "image_context.h"
#include "dummy_load_image.h"

class ContextFFI {
    ghidra::Sleigh sleigh;
    ghidra::ContextInternal c_db;
    DummyLoadImage image;
public:

    explicit ContextFFI(rust::Str slaPath);

    void set_initial_context(rust::Str name, uint32_t val);

    std::unique_ptr<SleighImage> makeImageContext(Image img);

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
