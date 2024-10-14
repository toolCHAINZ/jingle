//
// Created by mark denhoed on 10/10/24.
//
#include "sleigh_image.h"
#include "dummy_load_image.h"
#include "varnode_translation.h"
#include "sleigh/sleigh.hh"
#include <utility>


SleighImage::SleighImage(Image img, ghidra::Sleigh sl): sl(ghidra::Sleigh(sl)) {
    this->sl.reset(new DummyLoadImage(img), new ghidra::ContextInternal());
}

std::shared_ptr<AddrSpaceHandle> SleighImage::getSpaceByIndex(ghidra::int4 idx) const {
    return std::make_shared<AddrSpaceHandle>(sl.getSpace(idx));
}

ghidra::int4 SleighImage::getNumSpaces() const {
    return sl.numSpaces();
}

VarnodeInfoFFI SleighImage::getRegister(rust::Str name) const {
    ghidra::VarnodeData vn = sl.getRegister(name.operator std::string());
    VarnodeInfoFFI info;
    info.space = std::make_unique<AddrSpaceHandle>(vn.space);
    info.size = vn.size;
    info.offset = vn.offset;
    return info;
};

rust::Str SleighImage::getRegisterName(VarnodeInfoFFI vn) const {
    std::string name = sl.getRegisterName(vn.space->getRaw(), vn.offset, vn.size);
    return {name};
}

rust::Vec<RegisterInfoFFI> SleighImage::getRegisters() const {
    std::map<ghidra::VarnodeData, std::string> reglist;
    rust::Vec<RegisterInfoFFI> v;
    sl.getAllRegisters(reglist);
    v.reserve(reglist.size());
    for (auto const &vn: reglist) {
        v.emplace_back(collectRegInfo(vn));
    }
    return v;
}
