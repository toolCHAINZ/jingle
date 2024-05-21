# `jingle`: SMT Modeling for SLEIGH

`jingle` is a library for program analysis over traces of PCODE operations. I
am writing in the course of my PhD work and it is still very much "in flux".

This repository contains a [Cargo Workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) for two
related crates:

* [`jingle_sleigh`](./jingle_sleigh): a Rust FFI in front of [Ghidra](https://github.com/NationalSecurityAgency/ghidra)'
  s
  code translator: SLEIGH. Sleigh is written in C++ and can be
  found [here](https://github.com/NationalSecurityAgency/ghidra/tree/master/Ghidra/Features/Decompiler/src/decompile/cpp).
  This crate contains a private internal low-level API to SLEIGH and exposes an idiomatic high-level API to consumers.
* [`jingle`](./jingle): a set of functions built on top of `jingle_sleigh` that defines an encoding of PCODE operations
  into quantifier-free SMT statements operating on objects of the `Array(BitVec, BitVec)` sort. `jingle` is currently
  designed for providing formulas for use in decision procedures over program traces. A more robust analysis
  is forthcoming, depending on my research needs.

## Usage

In order to use `jingle`, include it in your `Cargo.toml` as usual:

```toml
jingle = { git = "ssh://git@github.com/toolCHAINZ/jingle", branch = "main" }
```

Again, this project is under active development an is still of "research quality" so it would probably make sense to
target
a tag or individual commit. I expect I will eventually put this on crates.io.