
#include "context.h"

#include "jingle_assembly_emitter.h"
#include "jingle_pcode_emitter.h"
#include "jingle_sleigh/src/ffi/instruction.rs.h"
#include "sleigh/globalcontext.hh"
#include "sleigh/loadimage.hh"
#include "sleigh/xml.hh"
#include "varnode_translation.h"
#include "rust_load_image.h"
#include <memory>
#include <utility>

ContextFFI::ContextFFI(rust::Str slaPath)
    : image(std::make_unique<DummyLoadImage>()),
      c_db(std::make_unique<ghidra::ContextInternal>()),
      sleigh(image.get(), c_db.get()) {
  ghidra::AttributeId::initialize();
  ghidra::ElementId::initialize();

  ghidra::DocumentStorage documentStorage = ghidra::DocumentStorage();

  std::stringstream sleighfilename;
  sleighfilename << "<sleigh>";
  sleighfilename << slaPath;
  sleighfilename << "</sleigh>";

  ghidra::Document *doc = documentStorage.parseDocument(sleighfilename);
  ghidra::Element *root = doc->getRoot();
  documentStorage.registerTag(root);
  sleigh.initialize(documentStorage);
  pristine_blob = c_db->getContext(ghidra::Address(ghidra::Address::m_maximal));
}

void ContextFFI::rebuild() {
  // Sleigh only holds raw pointers to the context database, so swap in the new one before
  // the old one is destroyed.
  auto fresh = std::make_unique<ghidra::ContextInternal>();
  sleigh.reset(image.get(), fresh.get());
  c_db = std::move(fresh);
  // Sleigh is already initialized, so this just re-registers the .sla context fields with the
  // new database and builds fresh context/disassembly caches.
  ghidra::DocumentStorage documentStorage;
  sleigh.initialize(documentStorage);
  // Register every variable before setting any default: registering can grow (and reset) the
  // default context blob.
  for (const auto &var : extra_context_vars) {
    sleigh.registerContext(std::get<0>(var), std::get<1>(var), std::get<2>(var));
  }
  for (const auto &def : context_defaults) {
    sleigh.setContextDefault(def.first, def.second);
  }
  sleigh.allowContextSet(allow_set);
  pristine_blob = c_db->getContext(ghidra::Address(ghidra::Address::m_maximal));
}

bool ContextFFI::context_dirty() const {
  // Every context commit splits the database's partition map. With no split points, every
  // address (including the maximal one) maps to the default blob; with any split point, the
  // maximal address maps to the blob of the last split instead.
  return c_db->getContext(ghidra::Address(ghidra::Address::m_maximal)) != pristine_blob;
}

void ContextFFI::reset_context(bool force) {
  if (force || context_dirty()) {
    rebuild();
  }
}

void ContextFFI::set_initial_context(rust::Str name, uint32_t val) {
  std::string n(name);
  // Throws for an unknown variable; do this before recording it so a bad name can't poison
  // every later rebuild.
  sleigh.setContextDefault(n, val);
  bool replaced = false;
  for (auto &def : context_defaults) {
    if (def.first == n) {
      def.second = val;
      replaced = true;
    }
  }
  if (!replaced) {
    context_defaults.emplace_back(n, val);
  }
  // Rebuild rather than set in place: cached parses may depend on the old default.
  rebuild();
}

void ContextFFI::allow_context_set(bool val) {
  allow_set = val;
  sleigh.allowContextSet(val);
}

void ContextFFI::register_context(rust::Str name, int32_t sbit, int32_t ebit) {
  extra_context_vars.emplace_back(std::string(name), sbit, ebit);
  // Sleigh refuses to register variables once the database holds context changes, so
  // register against a fresh database.
  rebuild();
}

rust::String ContextFFI::get_exact_register_name(int32_t space_idx, uint64_t off, uint32_t size) const {
  ghidra::AddrSpace *space = sleigh.getSpace(space_idx);
  return rust::String(sleigh.getExactRegisterName(space, off, size));
}

bool ContextFFI::is_initialized() const {
  return sleigh.isInitialized();
}

std::shared_ptr<AddrSpaceHandle>
ContextFFI::getSpaceByIndex(ghidra::int4 idx) const {
  return std::make_shared<AddrSpaceHandle>(sleigh.getSpace(idx));
}

ghidra::int4 ContextFFI::getNumSpaces() const { return sleigh.numSpaces(); }

VarnodeInfoFFI ContextFFI::getRegister(rust::Str name) const {
  ghidra::VarnodeData vn = sleigh.getRegister(name.operator std::string());
  VarnodeInfoFFI info;
  info.space = std::make_unique<AddrSpaceHandle>(vn.space);
  info.size = vn.size;
  info.offset = vn.offset;
  return info;
};

rust::Str ContextFFI::getRegisterName(VarnodeInfoFFI vn) const {
  std::string name =
      sleigh.getRegisterName(vn.space->getRaw(), vn.offset, vn.size);
  return {name};
}

rust::Vec<RegisterInfoFFI> ContextFFI::getRegisters() const {
  std::map<ghidra::VarnodeData, std::string> reglist;
  rust::Vec<RegisterInfoFFI> v;
  sleigh.getAllRegisters(reglist);
  v.reserve(reglist.size());
  for (auto const &vn : reglist) {
    v.emplace_back(collectRegInfo(vn));
  }
  return v;
}

 // Return the list of user-defined p-code userop names
rust::Vec<rust::String> ContextFFI::getUserOps() const {
  rust::Vec<rust::String> v;
  // Pull the names directly from Sleigh/SleighBase via getUserOpNames
  std::vector<std::string> names;
  sleigh.getUserOpNames(names);
  v.reserve(names.size());
  for (const auto &n : names) {
    v.emplace_back(rust::String(n));
  }
  return v;
}

void ContextFFI::setImage(ImageFFI const &img) {
  image = std::make_unique<RustLoadImage>(img);
  // A new image starts from default context: context committed while decoding the previous
  // image describes that image's addresses, not this one's.
  rebuild();
}

InstructionFFI ContextFFI::get_one_instruction(uint64_t offset) const {
  JinglePcodeEmitter pcode;
  JingleAssemblyEmitter assembly;
  ghidra::Address a = ghidra::Address(sleigh.getDefaultCodeSpace(), offset);
  sleigh.printAssembly(assembly, a);
  // oneInstruction's return value covers the instruction plus any delay slot(s) whose p-code it
  // has folded in; instructionLength() would cover only the instruction's own encoding.
  size_t length = sleigh.oneInstruction(pcode, a);
  InstructionFFI i;
  Disassembly d;
  i.ops = std::move(pcode.ops);
  d.args = std::move(assembly.body);
  d.mnemonic = std::move(assembly.mnem);
  i.disassembly = std::move(d);
  i.address = offset;
  i.length = length;
  return i;
}

std::unique_ptr<ContextFFI> makeContext(rust::Str slaPath) {
  return std::make_unique<ContextFFI>(slaPath);
}
