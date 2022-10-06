use swagger_rustgen::v2::{
    codegen::{
        backend::rust::{RustCodegen, RustType},
        CodeGenerator,
    },
    Swagger,
};

use clap::{Parser, Subcommand};
use std::fmt;

#[derive(Parser)]
struct SwaggerRustgen {
    #[clap(subcommand)]
    subcommand: Command,
}

#[derive(Subcommand)]
enum Command {
    Generate {
        #[clap(subcommand)]
        target: GenerateTarget,
    },
}

#[derive(Subcommand)]
enum GenerateTarget {
    Models {
        #[arg(short, long, default_value_t = Language::Rust)]
        language: Language,
        swagger_location: std::path::PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum Language {
    Rust,
}

impl AsRef<str> for Language {
    fn as_ref(&self) -> &str {
        match self {
            Language::Rust => "rust",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

fn main() {
    let rustgen = SwaggerRustgen::parse();
    pretty_env_logger::init();

    match rustgen.subcommand {
        Command::Generate { target } => match target {
            GenerateTarget::Models {
                swagger_location,
                language,
            } => {
                let yaml = std::fs::read_to_string(swagger_location).unwrap();

                let mut codegen = match language {
                    Language::Rust => {
                        let swagger: Swagger<RustType> = serde_yaml::from_str(&yaml).unwrap();
                        let backend = Box::new(RustCodegen::default());
                        CodeGenerator::new(swagger, backend)
                    }
                };
                let mut writer = Box::new(std::io::stdout()) as Box<dyn std::io::Write>;
                codegen.generate_models(&mut writer).unwrap();
            }
        },
    }
}
