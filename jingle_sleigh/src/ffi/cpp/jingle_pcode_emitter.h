//
// Created by toolCHAINZ on 10/14/24.
//

#ifndef JINGLE_SLEIGH_JINGLE_PCODE_EMITTER_H
#define JINGLE_SLEIGH_JINGLE_PCODE_EMITTER_H

#include "sleigh/translate.hh"
#include "jingle_sleigh/src/ffi/instruction.rs.h"

class JinglePcodeEmitter : public ghidra::PcodeEmit {

    void dump(const ghidra::Address &addr, ghidra::OpCode opc, ghidra::VarnodeData *outvar, ghidra::VarnodeData *vars,
              ghidra::int4 isize) override;

public:
    rust::Vec<RawPcodeOp> ops;
};

#endif //JINGLE_SLEIGH_JINGLE_PCODE_EMITTER_H
