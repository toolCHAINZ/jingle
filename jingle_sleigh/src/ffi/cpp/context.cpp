
#include "context.h"

#include <memory>
#include <utility>
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/loadimage.hh"

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

DummyLoadImage::DummyLoadImage() : ghidra::LoadImage("jingle") {
    img = Image{};
}

DummyLoadImage::DummyLoadImage(Image image) : ghidra::LoadImage("jingle") {
    img = std::move(image);
}

void DummyLoadImage::loadFill(ghidra::uint1 *ptr, ghidra::int4 size, const ghidra::Address &addr) {
    size_t offset = addr.getOffset();
    for (const auto &section: img.sections) {
        size_t start = section.base_address;
        size_t end = start + section.data.size();
        if (start <= offset && offset < end) {
            size_t len = std::min((size_t) size, (size_t) end - (size_t) offset);
            size_t start_idx = offset - start;
            std::memcpy(ptr, &section.data[start_idx], len);
            offset = offset + len;
        }
    }
    for (size_t i = offset; i < size; ++i) {
        ptr[i] = 0;
    }
}

void DummyLoadImage::adjustVma(long adjust) {}

std::string DummyLoadImage::getArchType() const {
    return "placeholder";
}

ContextFFI::ContextFFI(rust::Str slaPath, Image image) {
    ghidra::AttributeId::initialize();
    ghidra::ElementId::initialize();

    this->img = DummyLoadImage(std::move(image));
    documentStorage = ghidra::DocumentStorage();

    std::stringstream sleighfilename;
    sleighfilename << "<sleigh>";
    sleighfilename << slaPath;
    sleighfilename << "</sleigh>";

    ghidra::Document *doc = documentStorage.parseDocument(sleighfilename);
    ghidra::Element *root = doc->getRoot();
    documentStorage.registerTag(root);
    sleigh = std::make_unique<ghidra::Sleigh>(&img, &contextDatabase);
    sleigh->initialize(documentStorage);

}

void ContextFFI::set_initial_context(rust::Str name, uint32_t val) {
    sleigh->setContextDefault(name.operator std::string(), val);
}

InstructionFFI ContextFFI::get_one_instruction(uint64_t offset) const {
    PcodeCacher pcode;
    AssemblyCacher assembly;
    ghidra::Address a = ghidra::Address(sleigh->getDefaultCodeSpace(), offset);
    sleigh->printAssembly(assembly, a);
    sleigh->oneInstruction(pcode, a);
    size_t length = sleigh->instructionLength(a);
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


std::shared_ptr<AddrSpaceHandle> ContextFFI::getSpaceByIndex(ghidra::int4 idx) const {
    return std::make_shared<AddrSpaceHandle>(sleigh->getSpace(idx));
}

ghidra::int4 ContextFFI::getNumSpaces() const {
    return sleigh->numSpaces();
}

VarnodeInfoFFI ContextFFI::getRegister(rust::Str name) const {
    ghidra::VarnodeData vn = sleigh->getRegister(name.operator std::string());
    VarnodeInfoFFI info;
    info.space = std::make_unique<AddrSpaceHandle>(vn.space);
    info.size = vn.size;
    info.offset = vn.offset;
    return info;
};

rust::Str ContextFFI::getRegisterName(VarnodeInfoFFI vn) const {
    std::string name = sleigh->getRegisterName(vn.space->getRaw(), vn.offset, vn.size);
    return {name};
}

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath, Image img) {
    return std::make_unique<ContextFFI>(slaPath, std::move(img));
}

VarnodeInfoFFI varnodeToFFI(ghidra::VarnodeData vn) {
    VarnodeInfoFFI info;
    info.space = std::make_unique<AddrSpaceHandle>(vn.space);
    info.size = vn.size;
    info.offset = vn.offset;
    return info;
}

RegisterInfoFFI collectRegInfo(std::tuple<ghidra::VarnodeData, std::string> el) {
    VarnodeInfoFFI varnode = varnodeToFFI(std::get<0>(el));
    rust::String name = std::get<1>(el);
    return {varnode, name};
}

rust::Vec<RegisterInfoFFI> ContextFFI::getRegisters() const {
    std::map<ghidra::VarnodeData, std::string> reglist;
    rust::Vec<RegisterInfoFFI> v;
    sleigh->getAllRegisters(reglist);
    v.reserve(reglist.size());
    for (auto const& vn : reglist){
        v.emplace_back(collectRegInfo(vn));
    }
    return v;
}