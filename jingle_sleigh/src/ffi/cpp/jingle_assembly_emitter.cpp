//
// Created by toolCHAINZ on 10/14/24.
//

#include "jingle_assembly_emitter.h"

void JingleAssemblyEmitter::dump(const ghidra::Address &addr, const std::string &mnem, const std::string &body) {
    this->mnem = mnem;
    this->body = body;

}
