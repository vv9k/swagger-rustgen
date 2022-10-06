pub mod backend;
mod prototyper;

use crate::v2::{Swagger, Type};
use backend::CodegenBackend;
use prototyper::{ModelPrototype, Prototyper};

pub struct CodeGenerator<T: Type> {
    swagger: Swagger<T>,
    backend: Box<dyn CodegenBackend<T>>,
}

impl<T: Type> CodeGenerator<T> {
    pub fn new(swagger: Swagger<T>, backend: Box<dyn CodegenBackend<T>>) -> Self {
        Self { swagger, backend }
    }

    pub fn generate_models(&mut self, writer: &mut Box<dyn std::io::Write>) -> std::io::Result<()> {
        self.backend.generate(&self.swagger, writer)
    }
}
