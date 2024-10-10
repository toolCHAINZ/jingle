
#ifndef JINGLE_SLEIGH_DUMMY_LOAD_IMAGE_H
#define JINGLE_SLEIGH_DUMMY_LOAD_IMAGE_H


#include "jingle_sleigh/src/ffi/image.rs.h"
#include "sleigh/loadimage.hh"

class DummyLoadImage : public ghidra::LoadImage {
    Image img;
public:

    DummyLoadImage();

    DummyLoadImage(Image img);

    void loadFill(ghidra::uint1 *ptr, ghidra::int4 size, const ghidra::Address &addr) override;

    std::string getArchType(void) const override;

    void adjustVma(long adjust) override;

};

#endif //JINGLE_SLEIGH_DUMMY_LOAD_IMAGE_H
