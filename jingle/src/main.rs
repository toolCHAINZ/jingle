use clap::{Parser, Subcommand};
use hex::decode;
use jingle::modeling::{ModeledBlock, ModelingContext};
use jingle::JingleContext;
use jingle_sleigh::context::{Image, SleighContext, SleighContextBuilder};
use jingle_sleigh::{Disassembly, Instruction, JingleSleighError, PcodeOperation, VarNode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use z3::{Config, Context, Solver};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct JingleConfig {
    pub ghidra_path: PathBuf,
}

impl JingleConfig {
    pub fn sleigh_builder(&self) -> Result<SleighContextBuilder, JingleSleighError> {
        SleighContextBuilder::load_ghidra_installation(&self.ghidra_path)
    }
}

impl Default for JingleConfig {
    fn default() -> Self {
        return if cfg!(target_os = "windows") {
            let path = PathBuf::from(r"C:\Program Files\ghidra");
            Self { ghidra_path: path }
        } else if cfg!(target_os = "macos") {
            let path = PathBuf::from(r"/Applications/ghidra");
            Self { ghidra_path: path }
        } else {
            let path = PathBuf::from(r"/opt/ghidra");
            Self { ghidra_path: path }
        };
    }
}

impl From<&JingleParams> for JingleConfig {
    fn from(value: &JingleParams) -> Self {
        let path = value.ghidra_path.clone();
        Self {
            ghidra_path: path
                .map(|p| PathBuf::from(p))
                .unwrap_or(JingleConfig::default().ghidra_path),
        }
    }
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct JingleParams {
    #[command(subcommand)]
    pub command: Commands,
    pub ghidra_path: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Adds files to myapp
    Disassemble {
        architecture: String,
        hex_bytes: String,
    },
    Lift {
        architecture: String,
        hex_bytes: String,
    },
    Model {
        architecture: String,
        hex_bytes: String,
    },
    Architectures,
}

fn main() {
    let params: JingleParams = JingleParams::parse();
    update_config(&params);
    let config: JingleConfig = confy::load("jingle", None).unwrap();
    match params.command {
        Commands::Disassemble {
            architecture,
            hex_bytes,
        } => disassemble(&config, architecture, hex_bytes),
        Commands::Lift {
            architecture,
            hex_bytes,
        } => lift(&config, architecture, hex_bytes),
        Commands::Model {
            architecture,
            hex_bytes,
        } => model(&config, architecture, hex_bytes),
        Commands::Architectures => list_architectures(&config),
    }
}

fn update_config(params: &JingleParams) {
    let stored_config: JingleConfig = confy::load("jingle", None).unwrap();
    let new_config = JingleConfig::from(params);
    if stored_config != new_config {
        confy::store("jingle", None, new_config).unwrap()
    }
}

fn list_architectures(config: &JingleConfig) {
    let sleigh = config.sleigh_builder().unwrap();
    for language_id in sleigh.get_language_ids() {
        println!("{}", language_id)
    }
}

fn get_instructions(
    config: &JingleConfig,
    architecture: String,
    hex_bytes: String,
) -> (SleighContext, Vec<Instruction>) {
    let sleigh_build = config.sleigh_builder().unwrap();
    let img = decode(hex_bytes).unwrap();
    let max_len = img.len();
    let mut offset = 0;
    let sleigh = sleigh_build
        .set_image(Image::from(img))
        .build(&architecture)
        .unwrap();
    let mut instrs = vec![];
    while offset < max_len {
        for instruction in sleigh.read(offset as u64, 1) {
            offset += instruction.length;
            instrs.push(instruction);
        }
        if sleigh.read(offset as u64, 1).next().is_none() {
            break;
        }
    }
    (sleigh, instrs)
}

fn disassemble(config: &JingleConfig, architecture: String, hex_bytes: String) {
    for instr in get_instructions(config, architecture, hex_bytes).1 {
        println!("{}", instr.disassembly)
    }
}

fn lift(config: &JingleConfig, architecture: String, hex_bytes: String) {
    let (sleigh, instrs) = get_instructions(config, architecture, hex_bytes);
    for instr in instrs {
        for x in instr.ops {
            let x_disp = x.display(&sleigh).unwrap();
            println!("{}", x_disp)
        }
    }
}

fn model(config: &JingleConfig, architecture: String, hex_bytes: String) {
    let z3 = Context::new(&Config::new());
    let solver = Solver::new(&z3);
    let (sleigh, mut instrs) = get_instructions(config, architecture, hex_bytes);
    // todo: this is a disgusting hack to let us read a modeled block without requiring the user
    // to enter a block-terminating instruction. Everything with reading blocks needs to be reworked
    // at some point. For now, this lets me not break anything else relying on this behavior while
    // still getting this to work.
    instrs.push(Instruction {
        address: 0,
        disassembly: Disassembly {
            args: "".to_string(),
            mnemonic: "".to_string(),
        },
        ops: vec![PcodeOperation::Branch {
            input: VarNode {
                space_index: 1,
                offset: 0,
                size: 1,
            },
        }],
        length: 1,
    });

    let jingle_ctx = JingleContext::new(&z3, &sleigh);
    let block = ModeledBlock::read(&z3, &sleigh, instrs.into_iter()).unwrap();
    let final_state = jingle_ctx.fresh_state();
    solver.assert(&final_state._eq(block.get_final_state()).unwrap());
    println!("{}", solver.to_smt2())
}
