#include "varnode_translation.h"
#include "addrspace_handle.h"

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