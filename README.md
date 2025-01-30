<div align="center">

<img src="./jingle.svg" width="350"/>

🎶 <span style="font-style: italic; font-family: serif">Jingle bells, Jingle bells, Jingle all the `SLEIGH`</span> 🎶

</div>

# `jingle`: SMT Modeling for `p-code`
`jingle` is a library that translates (a fragment of) Ghidra's `p-code` into SMT. It allows expressing symbolic state
of the pcode vm and the relational semantics between those states defined by `p-code` operations.

**I am writing in the course of my PhD work and it is still very much "in flux". Breaking changes may happen at any time
and the overall design may change too.**

The API is currently a bit of a mess because I've been trying out different approaches to figure out what I like (e.g. 
traits vs context objects). I hope to clean it up at some point and expose one right way to do things.

This repository contains a [Cargo Workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) for two
related crates:

* [`jingle_sleigh`](./jingle_sleigh): a Rust FFI in front of [Ghidra](https://github.com/NationalSecurityAgency/ghidra)'
  s
  code translator: `SLEIGH`. `SLEIGH` is written in C++ and can be
  found [here](https://github.com/NationalSecurityAgency/ghidra/tree/master/Ghidra/Features/Decompiler/src/decompile/cpp).
  This crate contains a private internal low-level API to `SLEIGH` and exposes an idiomatic high-level API to consumers.
* [`jingle`](./jingle): a set of functions built on top of `jingle_sleigh` that defines an encoding of `p-code` operations
  into SMT. `jingle` is currently
  designed for providing formulas for use in decision procedures over individual program traces. As such, it does not yet
  expose APIs for constructing or reasoning about control-flow graphs. A more robust analysis
  is forthcoming, depending on my research needs.

## Requirements

### Building

If you're working directly with the `jingle` source distribution,
you will need to manually download a copy of the `ghidra` source tree
in order to build `jingle` or `jingle_sleigh`

If you're working with `git`, this can be done using the existing submodule.
Simply run

```shell
git submodule init && git submodule update
```

If you are for some reason using a zipped source distribution,
then you can run the following:

```shell
cd jingle_sleigh
git clone https://github.com/NationalSecurityAgency/ghidra.git
```

If you are using `jingle` as a cargo `git` or `crates.io` dependency,
this step is not necessary. `cargo` will handle all this in the `git` case
and we will vendor the necessary `ghidra` sources into all `crates.io` releases.

### Running

While `jingle` can be configured to work with a single set `sleigh` architecture,
the default way to use it is to point it to an existing `ghidra` installation.
[Install ghidra](https://ghidra-sre.org) and, if you are using `jingle` programatically,
point it at the top level folder of the installation. If you are using the [CLI](./jingle),
then provide the path to ghidra as an argument in your first run.

The only thing ghidra is used for here is as a standardized folder layout for `sleigh` architectures.
`jingle` has no ghidra dependency outside of the bundled `sleigh` C++ code.
## Usage

In order to use `jingle`, include it in your `Cargo.toml` as usual:

```toml
jingle = { git = "ssh://git@github.com/toolCHAINZ/jingle", branch = "main" }
```

Again, this project is under active development an is still of "research quality" so it would probably make sense to
target
a tag or individual commit. I expect I will eventually put this on crates.io.