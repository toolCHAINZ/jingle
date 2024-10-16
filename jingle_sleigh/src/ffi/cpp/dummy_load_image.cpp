#include "dummy_load_image.h"

DummyLoadImage::DummyLoadImage() : ghidra::LoadImage("jingle") {
}


void DummyLoadImage::loadFill(ghidra::uint1 *ptr, ghidra::int4 size,
                              const ghidra::Address &addr) {
    ghidra::ostringstream errmsg;
    errmsg << "Unable to load " << std::dec << size << " bytes at "
           << addr.getShortcut();
    addr.printRaw(errmsg);
    throw ghidra::DataUnavailError(errmsg.str());
}

void DummyLoadImage::adjustVma(long adjust) {}

std::string DummyLoadImage::getArchType() const { return "placeholder"; }
