
#include "context.h"

#include <memory>
#include <utility>
#include "sleigh_image.h"
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/loadimage.hh"
#include "varnode_translation.h"

ContextFFI::ContextFFI(rust::Str slaPath): sleigh(&image, &c_db) {
    ghidra::AttributeId::initialize();
    ghidra::ElementId::initialize();

    DummyLoadImage img = DummyLoadImage(Image());
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

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath) {
    return std::make_unique<ContextFFI>(slaPath);
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

std::unique_ptr<SleighImage> ContextFFI::makeImageContext(Image img) const {
    return std::make_unique<SleighImage>(img, sleigh);
}
