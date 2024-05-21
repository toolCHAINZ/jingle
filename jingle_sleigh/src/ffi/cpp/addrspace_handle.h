#ifndef JINGLE_SLEIGH_ADDRSPACE_HANDLE_H
#define JINGLE_SLEIGH_ADDRSPACE_HANDLE_H

#include "sleigh/types.h"
#include "sleigh/space.hh"
#include "rust/cxx.h"
#include <memory>

class AddrSpaceManagerHandle;

class AddrSpaceHandle {
private:
    const ghidra::AddrSpace *handle;
public:
    AddrSpaceHandle(const ghidra::AddrSpace *h);

    ghidra::AddrSpace* getRaw(void);

    rust::Str getName(void) const;

    ghidra::spacetype getType(void) const; ///< Get the type of space
    ghidra::int4 getDelay(void) const;     ///< Get number of heritage passes being delayed
    ghidra::int4 getDeadcodeDelay(void) const; ///< Get number of passes before deadcode removal is allowed
    ghidra::int4 getIndex(void) const;    ///< Get the integer identifier
    ghidra::uint4 getWordSize(void) const; ///< Get the addressable unit size
    ghidra::uint4 getAddrSize(void) const; ///< Get the size of the space
    ghidra::uintb getHighest(void) const;  ///< Get the highest byte-scaled address
    ghidra::uintb getPointerLowerBound(void) const;    ///< Get lower bound for assuming an offset is a pointer
    ghidra::uintb getPointerUpperBound(void) const;    ///< Get upper bound for assuming an offset is a pointer
    ghidra::int4 getMinimumPtrSize(void) const;    ///< Get the minimum pointer size for \b this space
    ghidra::uintb wrapOffset(ghidra::uintb off) const; ///< Wrap -off- to the offset that fits into this space
    char getShortcut(void) const; ///< Get the shortcut character
    bool isHeritaged(void) const;    ///< Return \b true if dataflow has been traced
    bool doesDeadcode(void) const; ///< Return \b true if dead code analysis should be done on this space
    bool hasPhysical(void) const;  ///< Return \b true if data is physically stored in this
    bool isBigEndian(void) const;  ///< Return \b true if values in this space are big endian
    bool isReverseJustified(void) const;  ///< Return \b true if alignment justification does not match endianness
    bool isFormalStackSpace(void) const;    ///< Return \b true if \b this is attached to the formal \b stack \b pointer
    bool isOverlay(void) const;  ///< Return \b true if this is an overlay space
    bool isOverlayBase(void) const; ///< Return \b true if other spaces overlay this space
    bool isOtherSpace(void) const;    ///< Return \b true if \b this is the \e other address space
    bool isTruncated(void) const; ///< Return \b true if this space is truncated from its original size
    bool
    hasNearPointers(void) const;    ///< Return \b true if \e near (truncated) pointers into \b this space are possible
    std::shared_ptr<AddrSpaceManagerHandle> getManager(void) const; ///< Get the space manager

};

#endif //JINGLE_SLEIGH_ADDRSPACE_HANDLE_H
