<div align="center">

<img src="./jingle.svg" width="350"/>

🎶 <span style="font-style: italic; font-family: serif">Jingle bells, Jingle bells, Jingle all the `SLEIGH`</span> 🎶

</div>

# `jingle`: SMT Modeling for `p-code`
`jingle` is a library that models (a fragment of) Ghidra's `p-code` in the language of SMT. It represents states of
the `p-code` Virtual Machine as expressions on the `QF_ABV` logic, and represents `p-code` operations as relations
between these states.

**ALPHA SOFTWARE:  this software is fresh, largely untested, and subject to change. It is not yet using semantic versioning.**

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
* [`jingle_python`](./jingle_python): a set of [pyo3](https://pyo3.rs) bindings for `jingle`. These bindings expose a 
  simple interface to both SLEIGH and our logical modeling of `p-code` in SMT. SMT formulae are exposed wrapped in
  their "native" python z3 classes, allowing easy integration with other tools. These bindings are _especially_ raw and
  subject to change.

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
`jingle` has no code dependency on ghidra outside of the bundled `sleigh` C++ code.

## Usage as a Library

In order to use `jingle`, include it in your `Cargo.toml` as usual:

```toml
jingle = { git = "ssh://git@github.com/toolCHAINZ/jingle", branch = "main" }
```

Again, this project is under active development an is still of "research quality" so we recommend targeting 
the latest commit SHA on `main` or a tag.

# Research Paper

`jingle` was developed in support of our research paper _Synthesis of Code-Reuse Attacks from `p-code` Programs_.
You can find the author accepted manuscript [here](https://ora.ox.ac.uk/objects/uuid:906d32ca-407c-4cab-beab-b90200f81d65).
This work has been accepted to [Usenix Security 2025](https://www.usenix.org/conference/usenixsecurity25/presentation/denhoed).

You can cite this work with the following BibTex:

```bibtex
@inproceedings {denhoed2025synthesis,
author = {Mark DenHoed and Thomas Melham},
title = {Synthesis of Code-Reuse Attacks from p-code Programs},
booktitle = {34th USENIX Security Symposium (USENIX Security 25)},
year = {2025},
address = {Seattle, WA},
publisher = {USENIX Association},
month = aug
}
```
