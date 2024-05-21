#include "addrspace_manager_handle.h"

AddrSpaceManagerHandle::AddrSpaceManagerHandle(const ghidra::AddrSpaceManager *h) {
    handle = h;
}

std::shared_ptr<AddrSpaceHandle> AddrSpaceManagerHandle::getSpaceByName(rust::Str nm) const {
    ghidra::AddrSpace *space = handle->getSpaceByName(nm.operator std::string());
    return std::shared_ptr<AddrSpaceHandle>(new AddrSpaceHandle(space));
}

std::shared_ptr<AddrSpaceHandle> AddrSpaceManagerHandle::getSpaceFromPointer(uint64_t idx) const {
    for (ghidra::int4 i = 0; i < handle->numSpaces(); i++) {
        if (reinterpret_cast<ghidra::AddrSpace *>(idx) == handle->getSpace(i)) {
            return std::shared_ptr<AddrSpaceHandle>(new AddrSpaceHandle(handle->getSpace(i)));
        }
    }
    throw "Something horrible has happened";
}

std::shared_ptr<AddrSpaceHandle> AddrSpaceManagerHandle::getSpaceByIndex(ghidra::int4 idx) const {
    return std::shared_ptr<AddrSpaceHandle>(new AddrSpaceHandle(handle->getSpace(idx)));
}

std::shared_ptr<AddrSpaceHandle> AddrSpaceManagerHandle::getDefaultCodeSpace() const {
return std::shared_ptr<AddrSpaceHandle>(new AddrSpaceHandle(handle->getDefaultCodeSpace()));
}

ghidra::int4 AddrSpaceManagerHandle::getNumSpaces() const {
    return handle->numSpaces();
}