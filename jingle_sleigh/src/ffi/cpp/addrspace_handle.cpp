#include "rust/cxx.h"
#include "sleigh/space.hh"
#include "addrspace_handle.h"
#include "addrspace_manager_handle.h"

AddrSpaceHandle::AddrSpaceHandle(const ghidra::AddrSpace *h) {
    handle = h;
}

ghidra::AddrSpace* AddrSpaceHandle::getRaw(void){
 return const_cast<ghidra::AddrSpace *>(handle);
}

rust::Str AddrSpaceHandle::getName(void) const {
    return rust::Str(handle->getName());
}

ghidra::spacetype AddrSpaceHandle::getType(void) const {
    return handle->getType();
}

ghidra::int4 AddrSpaceHandle::getDelay(void) const {
    return handle->getDelay();
}

ghidra::int4 AddrSpaceHandle::getDeadcodeDelay(void) const {
    return handle->getDeadcodeDelay();
}

ghidra::int4 AddrSpaceHandle::getIndex(void) const {
    return handle->getIndex();
}

ghidra::uint4 AddrSpaceHandle::getWordSize(void) const {
    return handle->getWordSize();
}

ghidra::uint4 AddrSpaceHandle::getAddrSize(void) const {
    return handle->getAddrSize();
}

ghidra::uintb AddrSpaceHandle::getHighest(void) const {
    return handle->getHighest();
}

ghidra::uintb AddrSpaceHandle::getPointerLowerBound(void) const {
    return handle->getPointerLowerBound();
}

ghidra::uintb AddrSpaceHandle::getPointerUpperBound(void) const {
    return handle->getPointerUpperBound();
}

ghidra::int4 AddrSpaceHandle::getMinimumPtrSize(void) const {
    return handle->getMinimumPtrSize();
}

ghidra::uintb AddrSpaceHandle::wrapOffset(ghidra::uintb off) const {
    return handle->wrapOffset(off);
}

char AddrSpaceHandle::getShortcut(void) const {
    return handle->getShortcut();
}

bool AddrSpaceHandle::isHeritaged(void) const {
    return handle->isHeritaged();
}

bool AddrSpaceHandle::doesDeadcode(void) const {
    return handle->doesDeadcode();
}

bool AddrSpaceHandle::hasPhysical(void) const {
    return handle->hasPhysical();
}

bool AddrSpaceHandle::isBigEndian(void) const {
    return handle->isBigEndian();
}

bool AddrSpaceHandle::isReverseJustified(void) const {
    return handle->isReverseJustified();
}

bool AddrSpaceHandle::isFormalStackSpace(void) const {
    return handle->isFormalStackSpace();
}

bool AddrSpaceHandle::isOverlay(void) const {
    return handle->isOverlay();
}

bool AddrSpaceHandle::isOverlayBase(void) const {
    return handle->isOverlayBase();
}

bool AddrSpaceHandle::isOtherSpace(void) const {
    return handle->isOtherSpace();
}

bool AddrSpaceHandle::isTruncated(void) const {
    return handle->isTruncated();
}

bool AddrSpaceHandle::hasNearPointers(void) const {
    return handle->hasNearPointers();
}

std::shared_ptr<AddrSpaceManagerHandle> AddrSpaceHandle::getManager() const {
    return std::shared_ptr<AddrSpaceManagerHandle>(new AddrSpaceManagerHandle(handle->getManager()));
}

















