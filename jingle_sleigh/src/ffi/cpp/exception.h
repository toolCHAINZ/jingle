
#ifndef JINGLE_EXCEPTION_H
#define JINGLE_EXCEPTION_H

#include "sleigh/error.hh"

namespace rust {
    namespace behavior {

    template <typename Try, typename Fail>
        static void trycatch(Try &&func, Fail &&fail) noexcept try {
          func();
        } catch (const ghidra::LowlevelError &e) {
          fail(e.explain);
        } catch (const std::exception &e) {
          fail(e.what());
        }
    }
}
#endif //JINGLE_EXCEPTION_H