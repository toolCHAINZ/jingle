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

When you provide it as the first argument to the `jingle` CLI, it
will save that path for future usage.

Once it has been configured, you can simple run it as follows:

```shell
jingle disassemble x86:LE:32:default 89fb
jingle lift x86:LE:32:default 89fb
jingle model x86:LE:32:default 89fb
```

These three invocations will print disassembly, pcode translation, and
a logical model respectively. None of these, particularly the logical model,
are intended to be used directly from this utility; this is merely for demonstration.
The proper way to use this tool is through the API.

The above invocations will produce the following output:
```shell
# jingle disassemble x86:LE:32:default 89fb
MOV EBX,EDI
```

```shell
# jingle lift x86:LE:32:default 89fb
EBX = COPY EDI
```

```shell
# jingle model x86:LE:32:default 89fb
; benchmark generated from rust API
(set-info :status unknown)
(declare-fun register!4 () (Array (_ BitVec 32) (_ BitVec 8)))
(declare-fun register!9 () (Array (_ BitVec 32) (_ BitVec 8)))
(declare-fun ram!3 () (Array (_ BitVec 32) (_ BitVec 8)))
(declare-fun ram!8 () (Array (_ BitVec 32) (_ BitVec 8)))
(declare-fun OTHER!1 () (Array (_ BitVec 64) (_ BitVec 8)))
(declare-fun OTHER!6 () (Array (_ BitVec 64) (_ BitVec 8)))
(assert
 (let ((?x77 (store (store register!4 (_ bv12 32) (select register!4 (_ bv28 32))) (_ bv13 32) (select register!4 (_ bv29 32)))))
 (let ((?x81 (store (store ?x77 (_ bv14 32) (select register!4 (_ bv30 32))) (_ bv15 32) (select register!4 (_ bv31 32)))))
 (let (($x82 (= register!9 ?x81)))
 (let (($x63 (= ram!8 ram!3)))
 (let (($x62 (= OTHER!6 OTHER!1)))
 (and $x62 $x63 $x82)))))))
(check-sat)

```

### Usage string

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