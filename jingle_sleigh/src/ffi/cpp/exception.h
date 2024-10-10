
#ifndef JINGLE_EXCEPTION_H
#define JINGLE_EXCEPTION_H

#include "sleigh/error.hh"
#include "sleigh/xml.hh"
#include "sleigh/loadimage.hh"

namespace rust {
    namespace behavior {

        template<typename Try, typename Fail>
        static void trycatch(Try &&func, Fail &&fail) noexcept try {
            func();
        } catch (const ghidra::LowlevelError &e) {
            fail(e.explain);
        } catch (const ghidra::DecoderError &e) {
            fail(e.explain);
        } catch (const ghidra::DataUnavailError &e){
            fail(e.explain);
        } catch (const std::exception &e) {
            fail(e.what());
        }
    }
}
#endif //JINGLE_EXCEPTION_H