use swagger_rustgen::v2::{
    codegen::{backend::rust::RustCodegen, CodeGenerator},
    Swagger,
};

use clap::{Parser, Subcommand};

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
        swagger_location: std::path::PathBuf,
    },
}

fn main() {
    let rustgen = SwaggerRustgen::parse();
    pretty_env_logger::init();

    match rustgen.subcommand {
        Command::Generate { target } => match target {
            GenerateTarget::Models { swagger_location } => {
                let yaml = std::fs::read_to_string(swagger_location).unwrap();
                let swagger: Swagger = serde_yaml::from_str(&yaml).unwrap();

                let backend = Box::new(RustCodegen::default());
                let mut codegen = CodeGenerator::new(swagger, backend);
                let mut writer = Box::new(std::io::stdout()) as Box<dyn std::io::Write>;
                codegen.generate_models(&mut writer).unwrap();
            }
        },
    }
}
