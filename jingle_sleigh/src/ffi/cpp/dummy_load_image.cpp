#include "dummy_load_image.h"

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
    for (size_t i = bytes_written; i < size; ++i) {
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
