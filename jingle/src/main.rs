use clap::builder::Str;
use clap::{Parser, Subcommand};
use jingle_sleigh::context::SleighContextBuilder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::Level;

#[derive(Debug, Serialize, Deserialize)]
struct JingleConfig {
    ghidra_path: PathBuf,
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

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct JingleParams {
    #[command(subcommand)]
    command: Commands,
    ghidra_path: Option<String>,
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
    if let Some(ghidra) = params.ghidra_path {
        update_config(ghidra);
    }
    let config: JingleConfig = confy::load("jingle", None).unwrap();
    match params.command {
        Commands::Disassemble { .. } => {}
        Commands::Lift { .. } => {}
        Commands::Model { .. } => {}
        Commands::Architectures => list_architectures(config.ghidra_path),
    }

    fn list_architectures(ghidra: PathBuf) {
        let sleigh = SleighContextBuilder::load_ghidra_installation(ghidra).unwrap();
        for language_id in sleigh.get_language_ids() {
            println!("{}", language_id)
        }
    }
    fn update_config(ghidra: String) {
        let config: JingleConfig = confy::load("jingle", None).unwrap();
        let ghidra = PathBuf::from(ghidra);
        if ghidra != config.ghidra_path {
            let new_config = JingleConfig {
                ghidra_path: ghidra,
            };
            confy::store("jingle", None, new_config).unwrap()
        }
    }
}
