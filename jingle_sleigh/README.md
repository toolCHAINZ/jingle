# `jingle_sleigh`: A Rust FFI Layer around Ghidra's SLEIGH

`sleigh` is the code translator at the heart of [ghidra](https://ghidra-sre.org/)'s excellent decompiler. `sleigh` converts
instruction bytes into sequences of `PCODE`, an intermediate representation of processor semantics on an idealized machine.

## License
`sleigh`, like the rest of ghidra is licensed APACHE 2, and you can find the original license reproduced [here](./SLEIGH_LICENSE).
`jingle` makes no modifications to the existing code in `sleigh`, but instead adds code outside it for FFI purposes. While
the `sleigh` source is not distributed in this repo (and is instead pulled in through a submodule), I am including the license
as distribution through crates.io would require vendoring the sleigh sources into the crate for distribution.

## Why do this?

This is [hardly](https://crates.io/crates/sleigh-sys) [the](https://crates.io/crates/sleigh) [first](https://crates.io/crates/sleigh-rs)
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
    #[test]
    fn get_one() {
        let mov_eax_0: [u8; 6] = [0xb8, 0x00, 0x00, 0x00, 0x00, 0xc3];
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let ctx = ctx_builder
            .set_image(Image::from(mov_eax_0.as_slice()))
            .build("x86:LE:64:default")
            .unwrap();
        let instr = ctx.read(0, 1).last().unwrap();
        assert_eq!(instr.length, 5);
        assert!(instr.disassembly.mnemonic.eq("MOV"));
        assert!(!instr.ops.is_empty());
        varnode!(&ctx, #0:4).unwrap();
        let _op = PcodeOperation::Copy {
            input: varnode!(&ctx, #0:4).unwrap(),
            output: varnode!(&ctx, "register"[0]:4).unwrap(),
        };
        assert!(matches!(&instr.ops[0], _op))
    }
```
