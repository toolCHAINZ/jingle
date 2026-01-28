<div align="center">

<img src="./jingle.svg" width="350"/>

🎶 <span style="font-style: italic; font-family: serif">Jingle bells, Jingle bells, Jingle all the `SLEIGH`</span> 🎶

</div>

# `jingle`: SMT Modeling for `p-code`
`jingle` provides SMT modeling Ghidra's `p-code`. It represents states of
the `p-code` Virtual Machine as expressions on the `QF_ABV` logic, and represents `p-code` operations as relations between these states. It additionally implements the Configurable
Program Analysis algorithm over pcode allowing for quickly implementing flexible custom analyses.

**ALPHA SOFTWARE:  this software is suitable for research usage but is not yet ready to be used in production.**

This repository contains a [Cargo Workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) for three
related crates:

* [`jingle_sleigh`](./jingle_sleigh): a Rust FFI in front of [Ghidra](https://github.com/NationalSecurityAgency/ghidra)'
  s
  code translator: `SLEIGH`. `SLEIGH` is written in C++ and can be
  found [here](https://github.com/NationalSecurityAgency/ghidra/tree/master/Ghidra/Features/Decompiler/src/decompile/cpp).
  This crate contains a private internal low-level API to `SLEIGH` and exposes an idiomatic high-level API to consumers.
* [`jingle`](./jingle): defines SMT modeling of p-code states and operations using [z3.rs](https://github.com/prove-rs/z3.rs) as well as a small program analysis framework. `jingle` implements [Configurable Program Analysis](https://doi.org/10.1007/978-3-319-10575-8_16), allowing for flexible custom program analysis, as well as pre-built analyses for building SMT models of unwound p-code programs.
* [`jingle_python`](./jingle_python): a set of [pyo3](https://pyo3.rs) bindings for `jingle`. These bindings expose a 
  simple interface to both SLEIGH and our logical modeling of `p-code` in SMT. SMT formulae are exposed wrapped in
  their "native" python z3 classes, allowing easy integration with other tools. These bindings are _especially_ raw and
  subject to change and do not yet expose any of the program analysis APIs.

## Usage

In order to use `jingle` in your project, you can just `cargo add` it:

```sh
cargo add jingle
```

While `jingle` can be configured to work with a single set `sleigh` architecture,
the default way to use it is to point it to an existing `ghidra` installation.
[Install ghidra](https://ghidra-sre.org) and use the installation root when instantiating the `SleighBuilder`. 
The only thing ghidra is used for here is as a standardized folder layout for `sleigh` architectures.
`jingle` has no code dependency on ghidra outside of the bundled `sleigh` C++ code.

### CLI 

You can install a simple CLI demonstrating jingle's modeling by running

```sh
cargo install --features bin jingle
```

If you are using the [CLI](./jingle),
then provide the path to ghidra as an argument in your first run.

The CLI produces disassembly, pcode, and SMT models for small hex-encoded instruction encodings. Note that the CLI uses an older version of `jingle`'s modeling that does not support arbitrary control flow.


## Development

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

# Research Paper

`jingle` was initially developed in support of our research paper, _Synthesis of Code-Reuse Attacks from `p-code` Programs_,
presented at [Usenix Security 2025](https://www.usenix.org/conference/usenixsecurity25/presentation/denhoed).

If you found the paper or the implementation useful, you can cite it with the following BibTeX:

```bibtex
@inproceedings{10.5555/3766078.3766099,
author = {DenHoed, Mark and Melham, Tom},
title = {Synthesis of code-reuse attacks from p-code programs},
year = {2025},
isbn = {978-1-939133-52-6},
publisher = {USENIX Association},
address = {USA},
booktitle = {Proceedings of the 34th USENIX Conference on Security Symposium},
articleno = {21},
numpages = {17},
location = {Seattle, WA, USA},
series = {SEC '25}
}
```
