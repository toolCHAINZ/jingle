
#ifndef JINGLE_SLEIGH_DUMMY_LOAD_IMAGE_H
#define JINGLE_SLEIGH_DUMMY_LOAD_IMAGE_H


#include "sleigh/loadimage.hh"

class DummyLoadImage : public ghidra::LoadImage {
public:

    DummyLoadImage();

    void loadFill(ghidra::uint1 *ptr, ghidra::int4 size, const ghidra::Address &addr) override;

    std::string getArchType(void) const override;

    void adjustVma(long adjust) override;

};

#endif //JINGLE_SLEIGH_DUMMY_LOAD_IMAGE_H
