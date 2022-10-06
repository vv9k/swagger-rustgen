pub mod backend;
mod prototyper;

use crate::v2::{Swagger, Type};
use backend::CodegenBackend;
use prototyper::{ModelPrototype, Prototyper};

use std::cmp::Ordering;

pub struct CodeGenerator<T: Type> {
    swagger: Swagger<T>,
    models_to_generate: Vec<ModelPrototype>,
    backend: Box<dyn CodegenBackend<T>>,
}

impl<T: Type> CodeGenerator<T> {
    pub fn new(swagger: Swagger<T>, backend: Box<dyn CodegenBackend<T>>) -> Self {
        Self {
            swagger,
            models_to_generate: vec![],
            backend,
        }
    }

    pub fn generate_models(&mut self, writer: &mut Box<dyn std::io::Write>) -> std::io::Result<()> {
        let p = Prototyper::default();
        self.models_to_generate = p.generate_prototypes(&self.swagger);
        self.generate(writer)?;

        Ok(())
    }

    fn generate(&mut self, writer: &mut Box<dyn std::io::Write>) -> std::io::Result<()> {
        self.backend.generate_helpers(&self.swagger, writer)?;

        let mut models = std::mem::take(&mut self.models_to_generate);

        // Generate object schemas first so that all references are valid
        // and fallback to alphabetical sorting
        models.sort_by(
            |a, b| match (a.schema.is_reference(), b.schema.is_reference()) {
                (true, true) | (false, false) => a.name.cmp(&b.name),
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
            },
        );

        for model in models {
            self.backend.generate_model(model, &self.swagger, writer)?;
        }

        Ok(())
    }
}
