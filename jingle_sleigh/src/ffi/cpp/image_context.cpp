//
// Created by mark denhoed on 10/10/24.
//
#include "image_context.h"
#include "context.h"

#include <utility>

class PcodeCacher : public ghidra::PcodeEmit {
public:
    rust::Vec<RawPcodeOp> ops;

    PcodeCacher() = default;

    void dump(const ghidra::Address &addr, ghidra::OpCode opc, ghidra::VarnodeData *outvar, ghidra::VarnodeData *vars,
              ghidra::int4 isize) override {
        RawPcodeOp op;
        op.op = opc;
        op.has_output = false;
        if (outvar != nullptr && outvar->space != nullptr) {
            op.has_output = true;
            op.output.offset = outvar->offset;
            op.output.size = outvar->size;
            op.output.space = std::make_unique<AddrSpaceHandle>(AddrSpaceHandle(outvar->space));
            outvar->space->getType();
        }
        op.inputs.reserve(isize);
        for (int i = 0; i < isize; i++) {
            VarnodeInfoFFI info;
            info.space = std::make_unique<AddrSpaceHandle>(vars[i].space);
            info.size = vars[i].size;
            info.offset = vars[i].offset;
            op.space = std::make_unique<AddrSpaceHandle>(addr.getSpace());
            op.inputs.emplace_back(std::move(info));
        }
        ops.emplace_back(op);

    }
};

class AssemblyCacher : public ghidra::AssemblyEmit {
public:
    rust::String mnem;
    rust::String body;

    AssemblyCacher() : mnem(""), body("") {

    };

    void dump(const ghidra::Address &addr, const std::string &mnem, const std::string &body) override {
        this->mnem = mnem;
        this->body = body;
    }
};

SleighImage::SleighImage(Image img, ghidra::Sleigh sl) {
    sl = sl;
    image = DummyLoadImage(img);
    sl.reset(&image, &c_db);
}



std::shared_ptr<AddrSpaceHandle> ContextFFI::getSpaceByIndex(ghidra::int4 idx) const {
    return std::make_shared<AddrSpaceHandle>(sleigh.getSpace(idx));
}

ghidra::int4 ContextFFI::getNumSpaces() const {
    return sleigh.numSpaces();
}

VarnodeInfoFFI ContextFFI::getRegister(rust::Str name) const {
    ghidra::VarnodeData vn = sleigh.getRegister(name.operator std::string());
    VarnodeInfoFFI info;
    info.space = std::make_unique<AddrSpaceHandle>(vn.space);
    info.size = vn.size;
    info.offset = vn.offset;
    return info;
};

rust::Str ContextFFI::getRegisterName(VarnodeInfoFFI vn) const {
    std::string name = sleigh.getRegisterName(vn.space->getRaw(), vn.offset, vn.size);
    return {name};
}

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath, Image img) {
    return std::make_unique<ContextFFI>(slaPath, std::move(img));
}