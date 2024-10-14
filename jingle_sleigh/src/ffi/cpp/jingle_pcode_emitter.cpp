
//
// Created by toolCHAINZ on 10/14/24.
//

#include "jingle_pcode_emitter.h"
#include "addrspace_handle.h"

void JinglePcodeEmitter::dump(const ghidra::Address &addr, ghidra::OpCode opc, ghidra::VarnodeData *outvar,
                              ghidra::VarnodeData *vars, ghidra::int4 isize) {
    RawPcodeOp op;
    op.op = opc;
    op.has_output = false;
    if (outvar != nullptr && outvar->space != nullptr) {
        op.has_output = true;
        op.output.offset = outvar->offset;
        op.output.size = outvar->size;
        op.output.space = std::make_unique<AddrSpaceHandle>(AddrSpaceHandle(outvar->space));
        outvar->space->getType();
    }
    op.inputs.reserve(isize);
    for (int i = 0; i < isize; i++) {
        VarnodeInfoFFI info;
        info.space = std::make_unique<AddrSpaceHandle>(vars[i].space);
        info.size = vars[i].size;
        info.offset = vars[i].offset;
        op.space = std::make_unique<AddrSpaceHandle>(addr.getSpace());
        op.inputs.emplace_back(std::move(info));
    }
    ops.emplace_back(op);


}
