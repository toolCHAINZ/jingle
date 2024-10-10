
#include "context.h"

#include <memory>
#include <utility>
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/loadimage.hh"


DummyLoadImage::DummyLoadImage() : ghidra::LoadImage("jingle") {
    img = Image{};
}

DummyLoadImage::DummyLoadImage(Image image) : ghidra::LoadImage("jingle") {
    img = std::move(image);
}

void DummyLoadImage::loadFill(ghidra::uint1 *ptr, ghidra::int4 size, const ghidra::Address &addr) {
    size_t offset = addr.getOffset();
    size_t bytes_written = 0;
    for (const auto &section: img.sections) {
        size_t start = section.base_address;
        size_t end = start + section.data.size();
        if (start <= offset && offset < end) {
            size_t len = std::min((size_t) size, (size_t) end - (size_t) offset);
            size_t start_idx = offset - start;
            std::memcpy(ptr, &section.data[start_idx], len);
            offset = offset + len;
            bytes_written += len;
        }
    }
    for (size_t i = offset; i < size; ++i) {
        ptr[i] = 0;
    }
    if (bytes_written == 0) {
        ghidra::ostringstream errmsg;
        errmsg << "Unable to load " << std::dec << size << " bytes at " << addr.getShortcut();
        addr.printRaw(errmsg);
        throw ghidra::DataUnavailError(errmsg.str());
    }
}

void DummyLoadImage::adjustVma(long adjust) {}

std::string DummyLoadImage::getArchType() const {
    return "placeholder";
}

ContextFFI::ContextFFI(rust::Str slaPath) {
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
    sleigh = ghidra::Sleigh(&img, &c_db);
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
    sleigh.getAllRegisters(reglist);
    v.reserve(reglist.size());
    for (auto const &vn: reglist) {
        v.emplace_back(collectRegInfo(vn));
    }
    return v;
}

std::unique_ptr<SleighImage> ContextFFI::makeImageContext(Image img) {
    return std::unique_ptr<SleighImage>(sleigh, img);
}
