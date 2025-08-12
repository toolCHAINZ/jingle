# `jingle_sleigh`: A Rust FFI Layer around Ghidra's SLEIGH

`sleigh` is the code translator at the heart of [ghidra](https://ghidra-sre.org/)'s excellent decompiler. `sleigh` converts
instruction bytes into sequences of `PCODE`, an intermediate representation of processor semantics on an idealized machine.

## License
`sleigh`, like the rest of ghidra is licensed APACHE 2, and you can find the original license reproduced [here](./SLEIGH_LICENSE).
`jingle` makes no modifications to the existing code in `sleigh`, but instead adds code outside it for FFI purposes. While
the `sleigh` source is not distributed in this repo (and is instead pulled in through a submodule), I am including the license
as distribution through crates.io would require vendoring the sleigh sources into the crate for distribution.

## Why do this?

This is [hardly](https://crates.io/crates/sleigh-sys) [the](https://crates.io/crates/sleigh) [only](https://crates.io/crates/libsla)
time someone has written a rust FFI around `sleigh`. However, I wanted to have control over the FFI since I had particular
requirements for `jingle` and figured "what's one more?"

## How do I use this?

This library provides only the rust binding around `sleigh`, you will need to provide your own architecture definition
for sleigh to parse. The easiest way to do this is to install ghidra, open a file of the given architecture, and then
point `jingle_sleigh` towards that ghidra installation. More enterprising users can run the sleigh compiler themselves.

I would have preferred to allow compiling slaspecs in this library, but there are some difficulties linking in those parts
of sleigh because much of the logic for parsing architectures exists in a file that also has a `main` and of course no linker
likes dealing with multiple `main`s.

But anyway, here's an example of usage yanked from the tests:

```rust
fn load_slice() {
    let ctx_builder =
        SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
    let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

    // an x86 push
    let img = vec![0x55u8];
    let sleigh = sleigh.initialize_with_image(img).unwrap();
    let instr = sleigh.instruction_at(0).unwrap();
    assert_eq!(instr.disassembly.mnemonic, "PUSH");
    assert_eq!(instr.ops.len(), 3);
    // the stages of a push in pcode
    assert_eq!(instr.ops[0].opcode(), OpCode::CPUI_COPY);
    assert_eq!(instr.ops[1].opcode(), OpCode::CPUI_INT_SUB);
    assert_eq!(instr.ops[2].opcode(), OpCode::CPUI_STORE);
}
```
