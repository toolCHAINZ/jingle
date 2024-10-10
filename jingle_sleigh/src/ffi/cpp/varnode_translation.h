
#ifndef JINGLE_SLEIGH_VARNODE_TRANSLATION_H
#define JINGLE_SLEIGH_VARNODE_TRANSLATION_H
#include "sleigh/types.h"
#include "sleigh/translate.hh"
#include "jingle_sleigh/src/ffi/instruction.rs.h"


VarnodeInfoFFI varnodeToFFI(ghidra::VarnodeData vn);

RegisterInfoFFI collectRegInfo(std::tuple<ghidra::VarnodeData, std::string> el);

#endif //JINGLE_SLEIGH_VARNODE_TRANSLATION_H
