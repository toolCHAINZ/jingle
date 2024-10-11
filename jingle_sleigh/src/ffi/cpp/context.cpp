
#include "context.h"

#include <memory>
#include <utility>
#include "sleigh/globalcontext.hh"
#include "sleigh_image.h"
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/loadimage.hh"
#include "varnode_translation.h"


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


ContextFFI::ContextFFI(rust::Str slaPath): sleigh(new DummyLoadImage(Image()), &c_db) {
    ghidra::AttributeId::initialize();
    ghidra::ElementId::initialize();

    ghidra::DocumentStorage documentStorage = ghidra::DocumentStorage();

    std::stringstream sleighfilename;
    sleighfilename << "<sleigh>";
    sleighfilename << slaPath;
    sleighfilename << "</sleigh>";

    ghidra::Document *doc = documentStorage.parseDocument(sleighfilename);
    ghidra::Element *root = doc->getRoot();
    documentStorage.registerTag(root);
    sleigh.initialize(documentStorage);

}

void ContextFFI::set_initial_context(rust::Str name, uint32_t val) {
    sleigh.setContextDefault(name.operator std::string(), val);
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

rust::Vec<RegisterInfoFFI> ContextFFI::getRegisters() const {
    std::map<ghidra::VarnodeData, std::string> reglist;
    rust::Vec<RegisterInfoFFI> v;
    sleigh.getAllRegisters(reglist);
    v.reserve(reglist.size());
    for (auto const &vn: reglist) {
        v.emplace_back(collectRegInfo(vn));
    }
    return v;
}

void ContextFFI::setImage(Image img) {
    sleigh.reset(new DummyLoadImage(std::move(img)), &c_db);
}

InstructionFFI ContextFFI::get_one_instruction(uint64_t offset) const {
    PcodeCacher pcode;
    AssemblyCacher assembly;
    ghidra::Address a = ghidra::Address(sleigh.getDefaultCodeSpace(), offset);
    sleigh.printAssembly(assembly, a);
    sleigh.oneInstruction(pcode, a);
    size_t length = sleigh.instructionLength(a);
    InstructionFFI i;
    Disassembly d;
    i.ops = std::move(pcode.ops);
    d.args = std::move(assembly.body);
    d.mnemonic = std::move(assembly.mnem);
    i.disassembly = std::move(d);
    i.address = offset;
    i.length = length;
    return i;
}

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath) {
    return std::make_unique<ContextFFI>(slaPath);
}
