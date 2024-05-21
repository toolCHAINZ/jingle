#ifndef JINGLE_SLEIGH_COMPILE_H
#define JINGLE_SLEIGH_COMPILE_H

#include "rust/cxx.h"

struct CompileParams;

void compile(rust::Str infile, rust::Str outFile, CompileParams params);

#endif //JINGLE_SLEIGH_COMPILE_H

