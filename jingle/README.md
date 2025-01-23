# `jingle`: Z3 + SLEIGH

`jingle` uses the sleigh bindings provided by `jingle_sleigh` and the excellent
z3 bindings from the `z3` crate to provide SMT modeling of sequences of `PCODE` instructions.

## CLI

`jingle` exposes a simple CLI tool for disassembling strings of executable bytes and modeling them in logic.

### Installation

From this folder:

```shell
cargo install --path . --features="bin_features"
```

This will install `jingle` in your path. Note that 

### Usage

`jingle` requires that a Ghidra installation be present.

```shell
Usage: jingle [GHIDRA_PATH] <COMMAND>

Commands:
  disassemble    Adds files to myapp
  lift           
  model          
  architectures  
  help           Print this message or the help of the given subcommand(s)

Arguments:
  [GHIDRA_PATH]  

Options:
  -h, --help     Print help
  -V, --version  Print version

```