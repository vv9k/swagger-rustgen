pub mod backend;
mod prototyper;

use crate::v2::Swagger;
use backend::CodegenBackend;
use prototyper::{ModelPrototype, Prototyper};

use std::cmp::Ordering;

pub struct CodeGenerator {
    swagger: Swagger,
    models_to_generate: Vec<ModelPrototype>,
    backend: Box<dyn CodegenBackend>,
}

impl CodeGenerator {
    pub fn new(swagger: Swagger, backend: Box<dyn CodegenBackend>) -> Self {
        Self {
            swagger,
            models_to_generate: vec![],
            backend,
        }
    }

    pub fn generate_models(&mut self, writer: &mut Box<dyn std::io::Write>) -> std::io::Result<()> {
        let swagger = self.swagger.clone();
        let p = Prototyper::default();
        self.models_to_generate = p.generate_prototypes(swagger);
        self.generate(writer)?;

        Ok(())
    }

    fn generate(&mut self, writer: &mut Box<dyn std::io::Write>) -> std::io::Result<()> {
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
