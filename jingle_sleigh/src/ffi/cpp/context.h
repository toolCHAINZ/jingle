#ifndef JINGLE_SLEIGH_CONTEXT_H
#define JINGLE_SLEIGH_CONTEXT_H

class ContextFFI;
#include "jingle_sleigh/src/ffi/context_ffi.rs.h"
#include "rust/cxx.h"
#include "sleigh/types.h"
#include "addrspace_handle.h"
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/globalcontext.hh"
#include "sleigh/sleigh.hh"
#include "sleigh/loadimage.hh"
#include "dummy_load_image.h"
#include <memory>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

class ContextFFI {
    // Declared before `sleigh` so they outlive it: `sleigh` holds raw pointers to both.
    std::unique_ptr<ghidra::LoadImage> image;
    std::unique_ptr<ghidra::ContextInternal> c_db;
    ghidra::Sleigh sleigh;

    // Context configuration applied on top of the .sla definitions. Recorded so that it can be
    // replayed whenever the context database is rebuilt.
    std::vector<std::tuple<std::string, int32_t, int32_t>> extra_context_vars;
    std::vector<std::pair<std::string, uint32_t>> context_defaults;
    bool allow_set = true;

    // The context blob returned for any address while the database holds no context changes.
    const ghidra::uintm *pristine_blob = nullptr;

    // Replace the context database with a fresh one (defaults only), drop all of sleigh's
    // parse caches, and replay the recorded context configuration.
    void rebuild();

public:

    explicit ContextFFI(rust::Str slaPath);

    void set_initial_context(rust::Str name, uint32_t val);

    void allow_context_set(bool val);

    void register_context(rust::Str name, int32_t sbit, int32_t ebit);

    // True if any decode since the last rebuild committed a context change (`globalset`).
    bool context_dirty() const;

    // Restore the context database to its defaults and clear the instruction cache. When
    // `force` is false this is a no-op unless `context_dirty()`.
    void reset_context(bool force);

    rust::String get_exact_register_name(int32_t space_idx, uint64_t off, uint32_t size) const;

    bool is_initialized() const;

    void setImage(ImageFFI const&img);

    InstructionFFI get_one_instruction(uint64_t offset) const;

    [[nodiscard]] std::shared_ptr<AddrSpaceHandle> getSpaceByIndex(ghidra::int4 idx) const;

    int getNumSpaces() const;

    VarnodeInfoFFI getRegister(rust::Str name) const;

    rust::Str getRegisterName(VarnodeInfoFFI name) const;

    rust::Vec<RegisterInfoFFI> getRegisters() const;

    // Return the list of user-defined p-code userop names
    rust::Vec<rust::String> getUserOps() const;
};

RegisterInfoFFI collectRegInfo(std::tuple<ghidra::VarnodeData*, std::string> el);

VarnodeInfoFFI varnodeToFFI(ghidra::VarnodeData vn);

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath);

#endif //JINGLE_SLEIGH_CONTEXT_H
