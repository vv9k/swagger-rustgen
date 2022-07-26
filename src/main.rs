use swagger_rustgen::v2::{codegen::CodeGenerator, Swagger};

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

    match rustgen.subcommand {
        Command::Generate { target } => match target {
            GenerateTarget::Models { swagger_location } => {
                let yaml = std::fs::read_to_string(swagger_location).unwrap();
                let swagger: Swagger = serde_yaml::from_str(&yaml).unwrap();

                let mut codegen = CodeGenerator::new(swagger);
                codegen.generate_models(&mut std::io::stdout()).unwrap();
            }
        },
    }
}
