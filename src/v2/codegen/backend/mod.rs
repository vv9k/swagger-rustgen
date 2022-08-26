pub mod rust;

use crate::v2::{codegen::ModelPrototype, Swagger};

pub trait CodegenBackend {
    fn generate_model(
        &mut self,
        model: ModelPrototype,
        swagger: &Swagger,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()>;
}
