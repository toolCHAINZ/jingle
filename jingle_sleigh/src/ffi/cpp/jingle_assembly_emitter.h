//
// Created by toolCHAINZ on 10/14/24.
//

#ifndef JINGLE_SLEIGH_JINGLE_ASSEMBLY_EMITTER_H
#define JINGLE_SLEIGH_JINGLE_ASSEMBLY_EMITTER_H

#include "sleigh/translate.hh"
#include "rust/cxx.h"

class JingleAssemblyEmitter : public ghidra::AssemblyEmit {


    void dump(const ghidra::Address &addr, const std::string &mnem, const std::string &body) override;

public:
    rust::String body;
    rust::String mnem;
};

#endif //JINGLE_SLEIGH_JINGLE_ASSEMBLY_EMITTER_H
