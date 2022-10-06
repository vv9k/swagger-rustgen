pub mod python;
pub mod rust;

use crate::v2::{codegen::ModelPrototype, Swagger, Type};

pub trait CodegenBackend<T: Type> {
    fn generate_model(
        &mut self,
        model: ModelPrototype,
        swagger: &Swagger<T>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()>;

    fn generate_helpers(
        &mut self,
        swagger: &Swagger<T>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()>;
}
