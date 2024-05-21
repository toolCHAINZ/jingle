#ifndef JINGLE_SLEIGH_ADDRSPACE_MANAGER_HANDLE_H
#define JINGLE_SLEIGH_ADDRSPACE_MANAGER_HANDLE_H

#include "sleigh/types.h"
#include "sleigh/translate.hh"
#include "addrspace_handle.h"
#include "rust/cxx.h"

class AddrSpaceManagerHandle {
private:
    ghidra::AddrSpaceManager const *handle;
public:
    AddrSpaceManagerHandle(const ghidra::AddrSpaceManager *h);

    std::shared_ptr<AddrSpaceHandle> getSpaceByName(rust::Str nm) const; ///< Get address space by name
    std::shared_ptr<AddrSpaceHandle> getSpaceByShortcut(char sc) const;    ///< Get address space from its shortcut
    std::shared_ptr<AddrSpaceHandle> getJoinSpace(void) const; ///< Get the joining space
    std::shared_ptr<AddrSpaceHandle> getStackSpace(void) const; ///< Get the stack space for this processor
    std::shared_ptr<AddrSpaceHandle>
    getUniqueSpace(void) const; ///< Get the temporary register space for this processor
    std::shared_ptr<AddrSpaceHandle>
    getDefaultCodeSpace(void) const; ///< Get the default address space of this processor
    std::shared_ptr<AddrSpaceHandle>
    getDefaultDataSpace(void) const; ///< Get the default address space where data is stored
    std::shared_ptr<AddrSpaceHandle> getConstantSpace(void) const; ///< Get the constant space
    std::shared_ptr<AddrSpaceHandle> getSpaceFromPointer(uint64_t i) const; ///< Get an address space via its index
    std::shared_ptr<AddrSpaceHandle> getSpaceByIndex(ghidra::int4 idx) const;

    int getNumSpaces() const;
};

#endif //JINGLE_SLEIGH_ADDRSPACE_MANAGER_HANDLE_H
