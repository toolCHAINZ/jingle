//
// Created by toolCHAINZ on 10/15/24.
//

#ifndef JINGLE_SLEIGH_RUST_LOAD_IMAGE_H
#define JINGLE_SLEIGH_RUST_LOAD_IMAGE_H

#include "context.h"
#include "sleigh/loadimage.hh"

class RustLoadImage : public ghidra::LoadImage {
    ImageFFI const &img;
public:
    RustLoadImage(ImageFFI const& img) : LoadImage("placeholder"), img(img) {};

    void loadFill(ghidra::uint1 *ptr, ghidra::int4 size, const ghidra::Address &addr) override;

    std::string getArchType(void) const override;

    void adjustVma(long adjust) override;

};

#endif //JINGLE_SLEIGH_RUST_LOAD_IMAGE_H
