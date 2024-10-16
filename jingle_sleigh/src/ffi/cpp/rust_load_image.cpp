//
// Created by toolCHAINZ on 10/15/24.
//

#include "rust_load_image.h"
#include "sleigh/pcoderaw.hh"
#include "varnode_translation.h"

void RustLoadImage::loadFill(ghidra::uint1 *ptr, ghidra::int4 size, const ghidra::Address &addr) {
    ghidra::VarnodeData vn = {addr.getSpace(), addr.getOffset(), static_cast<ghidra::uint4>(size)};

    size_t result = img.load(varnodeToFFI(vn), rust::Slice(ptr, size));
    if(result == 0){
        ghidra::ostringstream errmsg;
        errmsg << "Unable to load " << std::dec << size << " bytes at "
               << addr.getShortcut();
        addr.printRaw(errmsg);
        throw ghidra::DataUnavailError(errmsg.str());
    }
}

std::string RustLoadImage::getArchType(void) const {
    return "placeholder";
}

void RustLoadImage::adjustVma(long adjust) {

}
