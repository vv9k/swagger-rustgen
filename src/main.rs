// https://swagger.io/specification/v2/#definitionsObject

use swagger_rustgen::{codegen::CodeGenerator, Swagger};

fn main() {
    let yaml = std::fs::read_to_string("/home/wojtek/Downloads/swagger-v4.2.yaml").unwrap();
    let swagger: Swagger = serde_yaml::from_str(&yaml).unwrap();

    let codegen = CodeGenerator::new(swagger);
    codegen.generate_models(&mut std::io::stdout()).unwrap();
}
