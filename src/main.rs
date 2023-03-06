use swagger_gen::v2::{
    codegen::{
        backend::{python, rust},
        CodeGenerator,
    },
    Swagger,
};

use clap::{Parser, Subcommand};
use std::fmt;

#[derive(Parser)]
struct SwaggerGen {
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
    Python,
}

impl AsRef<str> for Language {
    fn as_ref(&self) -> &str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[derive(Copy, Clone, Debug)]
enum DataFormat {
    Json,
    Yaml,
}

impl DataFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            _ => None,
        }
    }

    pub fn deserialize_from_slice<T: serde::de::DeserializeOwned>(
        self,
        data: &[u8],
    ) -> Result<T, Box<dyn std::error::Error>> {
        match self {
            DataFormat::Json => Ok(serde_json::from_slice::<T>(&data)?),
            DataFormat::Yaml => Ok(serde_yaml::from_slice::<T>(&data)?),
        }
    }
}

fn main() {
    let gen = SwaggerGen::parse();
    pretty_env_logger::init();

    match gen.subcommand {
        Command::Generate { target } => match target {
            GenerateTarget::Models {
                swagger_location,
                language,
            } => {
                let data_format = swagger_location
                    .extension()
                    .and_then(|ext| DataFormat::from_extension(&ext.to_string_lossy()))
                    .unwrap_or(DataFormat::Yaml);
                let data = std::fs::read(swagger_location).unwrap();

                match language {
                    Language::Rust => {
                        let swagger: Swagger<rust::Type> =
                            data_format.deserialize_from_slice(&data).unwrap();
                        let backend = Box::new(rust::Codegen::default());
                        let mut codegen = CodeGenerator::new(swagger, backend);
                        let mut writer = Box::new(std::io::stdout()) as Box<dyn std::io::Write>;
                        codegen.generate_models(&mut writer).unwrap();
                    }
                    Language::Python => {
                        let swagger: Swagger<python::Type> =
                            data_format.deserialize_from_slice(&data).unwrap();
                        let backend = Box::new(python::Codegen::default());
                        let mut codegen = CodeGenerator::new(swagger, backend);
                        let mut writer = Box::new(std::io::stdout()) as Box<dyn std::io::Write>;
                        codegen.generate_models(&mut writer).unwrap();
                    }
                };
            }
        },
    }
}
