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

class DummyLoadImage : public ghidra::LoadImage {
    Image img;
public:
    DummyLoadImage();

    DummyLoadImage(Image img);

    void loadFill(ghidra::uint1 *ptr, ghidra::int4 size, const ghidra::Address &addr) override;

    std::string getArchType(void) const override;

    void adjustVma(long adjust) override;

};


class ContextFFI {
    DummyLoadImage img;
    ghidra::DocumentStorage documentStorage;
    ghidra::ContextInternal contextDatabase;
    std::unique_ptr<ghidra::Sleigh> sleigh;
public:

    explicit ContextFFI(rust::Str slaPath, Image img);

    void set_initial_context(rust::Str name, uint32_t val);

    InstructionFFI get_one_instruction(uint64_t offset) const;


    [[nodiscard]] std::shared_ptr<AddrSpaceHandle> getSpaceByIndex(ghidra::int4 idx) const;

    int getNumSpaces() const;

    VarnodeInfoFFI getRegister(rust::Str name) const;

    rust::Str getRegisterName(VarnodeInfoFFI name) const;

    rust::Vec<RegisterInfoFFI> getRegisters() const;
};

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath, Image img);

#endif //JINGLE_SLEIGH_CONTEXT_H
